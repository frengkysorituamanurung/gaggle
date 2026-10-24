use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::broadcast;

/// Maximum bytes retained in the in-memory scrollback buffer per terminal session.
pub const DEFAULT_SCROLLBACK_BYTES: usize = 1024 * 1024;

/// Allowlisted environment variables inherited from the host process.
const SAFE_ENV_VARS: &[&str] = &["PATH", "HOME", "USER", "LANG", "LC_ALL", "TERM", "TMPDIR"];

#[derive(Debug, Error)]
pub enum TerminalPtyError {
    #[error("Failed to open PTY: {0}")]
    OpenPty(String),

    #[error("Failed to spawn shell: {0}")]
    Spawn(String),

    #[error("PTY I/O error: {0}")]
    Io(String),

    #[error("Invalid terminal dimensions: cols={cols}, rows={rows}")]
    InvalidDimensions { cols: u16, rows: u16 },
}

#[derive(Debug, Clone)]
pub struct TerminalPtyConfig {
    pub shell: String,
    pub cwd: PathBuf,
    pub cols: u16,
    pub rows: u16,
}

impl TerminalPtyConfig {
    pub fn validate(&self) -> Result<(), TerminalPtyError> {
        if self.cols == 0 || self.rows == 0 {
            return Err(TerminalPtyError::InvalidDimensions {
                cols: self.cols,
                rows: self.rows,
            });
        }
        Ok(())
    }
}

pub struct TerminalPty {
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Option<Box<dyn portable_pty::Child + Send + Sync>>>>,
    process_id: Option<u32>,
    output_tx: broadcast::Sender<Vec<u8>>,
    scrollback: Arc<Mutex<Vec<u8>>>,
    exit_code: Arc<Mutex<Option<i32>>>,
    exited: Arc<AtomicBool>,
    _reader_task: tokio::task::JoinHandle<()>,
    _wait_task: tokio::task::JoinHandle<()>,
}

impl TerminalPty {
    pub fn spawn(config: TerminalPtyConfig) -> Result<Self, TerminalPtyError> {
        config.validate()?;

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalPtyError::OpenPty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&config.shell);
        cmd.cwd(&config.cwd);

        cmd.env_clear();
        for key in SAFE_ENV_VARS {
            if let Ok(val) = std::env::var(key) {
                cmd.env(key, val);
            }
        }
        if std::env::var("TERM").is_err() {
            cmd.env("TERM", "xterm-256color");
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalPtyError::Spawn(e.to_string()))?;

        let process_id = child.process_id();

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalPtyError::Io(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalPtyError::Io(e.to_string()))?;

        let master = Arc::new(Mutex::new(pair.master));
        let writer = Arc::new(Mutex::new(writer));
        let child = Arc::new(Mutex::new(Some(child)));

        let (output_tx, _) = broadcast::channel::<Vec<u8>>(256);
        let scrollback = Arc::new(Mutex::new(Vec::new()));
        let exit_code = Arc::new(Mutex::new(None));
        let exited = Arc::new(AtomicBool::new(false));

        let tx_clone = output_tx.clone();
        let scrollback_clone = scrollback.clone();
        let reader_task = tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();
                        {
                            let mut sb = scrollback_clone.lock().unwrap_or_else(|p| p.into_inner());
                            append_bounded(&mut sb, &chunk, DEFAULT_SCROLLBACK_BYTES);
                        }
                        let _ = tx_clone.send(chunk);
                    }
                    Err(_) => break,
                }
            }
        });

        let child_for_wait = child.clone();
        let exit_code_clone = exit_code.clone();
        let exited_clone = exited.clone();
        let wait_task = tokio::task::spawn_blocking(move || {
            let mut child_opt = child_for_wait
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .take();
            if let Some(mut child) = child_opt.take() {
                let status = child.wait();
                let code = status.map(|s| s.exit_code() as i32).unwrap_or(-1);
                *exit_code_clone.lock().unwrap_or_else(|p| p.into_inner()) = Some(code);
            } else {
                *exit_code_clone.lock().unwrap_or_else(|p| p.into_inner()) = Some(-1);
            }
            exited_clone.store(true, Ordering::SeqCst);
        });

        Ok(Self {
            master,
            writer,
            child,
            process_id,
            output_tx,
            scrollback,
            exit_code,
            exited,
            _reader_task: reader_task,
            _wait_task: wait_task,
        })
    }

    pub fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.output_tx.subscribe()
    }

    pub fn write(&self, data: &[u8]) -> Result<(), TerminalPtyError> {
        let mut w = self.writer.lock().unwrap_or_else(|p| p.into_inner());
        w.write_all(data)
            .map_err(|e| TerminalPtyError::Io(e.to_string()))?;
        w.flush().map_err(|e| TerminalPtyError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalPtyError> {
        if cols == 0 || rows == 0 {
            return Err(TerminalPtyError::InvalidDimensions { cols, rows });
        }
        let master = self.master.lock().unwrap_or_else(|p| p.into_inner());
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalPtyError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn scrollback(&self) -> Vec<u8> {
        self.scrollback
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn has_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    pub fn exit_code(&self) -> Option<i32> {
        *self.exit_code.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn kill(&self) {
        if let Some(child) = self
            .child
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_mut()
        {
            let _ = child.kill();
        }
        if let Some(pid) = self.process_id {
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
            #[cfg(not(unix))]
            {
                let _ = pid;
            }
        }
    }
}

fn append_bounded(buf: &mut Vec<u8>, chunk: &[u8], cap: usize) {
    if chunk.len() >= cap {
        buf.clear();
        buf.extend_from_slice(&chunk[chunk.len() - cap..]);
        return;
    }
    if buf.len() + chunk.len() > cap {
        let overflow = buf.len() + chunk.len() - cap;
        if overflow >= buf.len() {
            buf.clear();
        } else {
            buf.drain(0..overflow);
        }
    }
    buf.extend_from_slice(chunk);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn pty_echo_output() {
        let tmp = std::env::temp_dir();
        let cfg = TerminalPtyConfig {
            shell: "/bin/sh".to_string(),
            cwd: tmp,
            cols: 80,
            rows: 24,
        };
        let pty = TerminalPty::spawn(cfg).expect("spawn");
        pty.write(b"echo HELLO_PTY\n").unwrap();
        let mut rx = pty.subscribe();
        let mut collected = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Ok(chunk)) => {
                    collected.extend_from_slice(&chunk);
                    if String::from_utf8_lossy(&collected).contains("HELLO_PTY") {
                        break;
                    }
                }
                _ => {
                    if pty.has_exited() {
                        break;
                    }
                }
            }
        }
        let text = String::from_utf8_lossy(&collected);
        assert!(text.contains("HELLO_PTY"), "output: {text}");
        pty.kill();
    }

    #[tokio::test]
    async fn pty_resize() {
        let cfg = TerminalPtyConfig {
            shell: "/bin/sh".to_string(),
            cwd: std::env::temp_dir(),
            cols: 80,
            rows: 24,
        };
        let pty = TerminalPty::spawn(cfg).expect("spawn");
        pty.resize(120, 40).expect("resize");
        pty.kill();
    }

    #[tokio::test]
    async fn pty_termination() {
        let cfg = TerminalPtyConfig {
            shell: "/bin/sh".to_string(),
            cwd: std::env::temp_dir(),
            cols: 80,
            rows: 24,
        };
        let pty = TerminalPty::spawn(cfg).expect("spawn");
        pty.write(b"exit 42\n").unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !pty.has_exited() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(pty.has_exited(), "did not exit");
        pty.kill();
    }
}
