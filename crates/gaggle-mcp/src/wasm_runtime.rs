use anyhow::{Result, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Feature-enabled implementation
// ---------------------------------------------------------------------------

#[cfg(feature = "wasm")]
use wasmtime::component::{Component, Linker, Val};
#[cfg(feature = "wasm")]
use wasmtime::{Engine, Store};

#[cfg(feature = "wasm")]
pub struct WasmExtensionRuntime {
    engine: Engine,
    component: Option<Component>,
    path: Option<PathBuf>,
}

#[cfg(feature = "wasm")]
impl Default for WasmExtensionRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "wasm")]
impl WasmExtensionRuntime {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
            component: None,
            path: None,
        }
    }

    /// Load a `.wasm` component from disk, returning a ready runtime.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut rt = Self::new();
        rt.load_module(path)?;
        Ok(rt)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::load(path)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        Self::load(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::load(path)
    }

    /// Load a module into an existing runtime instance.
    pub fn load_module(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let component = Component::from_file(&self.engine, path)
            .map_err(|e| anyhow::anyhow!("failed to load wasm module '{}': {}", path.display(), e))?;
        self.component = Some(component);
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.component.is_some()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Call the exported `run` function with JSON input, returning JSON output.
    pub fn run(&self, input: Value) -> Result<Value> {
        let component = self
            .component
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no wasm module loaded"))?;
        run_component(&self.engine, component, input)
    }

    pub fn call(&self, input: Value) -> Result<Value> {
        self.run(input)
    }

    pub fn execute(&self, input: Value) -> Result<Value> {
        self.run(input)
    }

    pub fn run_json(&self, input: Value) -> Result<Value> {
        self.run(input)
    }

    pub fn run_with_path(&self, path: impl AsRef<Path>, input: Value) -> Result<Value> {
        let mut rt = Self::new();
        rt.load_module(path)?;
        rt.run(input)
    }

    pub fn run_module(path: impl AsRef<Path>, input: Value) -> Result<Value> {
        Self::load(path)?.run(input)
    }

    pub fn call_run(path: impl AsRef<Path>, input: Value) -> Result<Value> {
        Self::run_module(path, input)
    }
}

#[cfg(feature = "wasm")]
fn run_component(engine: &Engine, component: &Component, input: Value) -> Result<Value> {
    let linker = Linker::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker
        .instantiate(&mut store, component)
        .map_err(|e| anyhow::anyhow!("failed to instantiate wasm module: {}", e))?;
    let func = instance
        .get_func(&mut store, "run")
        .context("exported 'run' function not found")?;

    let input_json = serde_json::to_string(&input).context("failed to serialize input")?;
    let params = [Val::String(input_json)];
    let mut results = [Val::String(String::new())];
    func.call(&mut store, &params, &mut results)
        .map_err(|e| anyhow::anyhow!("failed to call exported 'run' function: {}", e))?;

    let output = match &results[0] {
        Val::String(s) => s.clone(),
        other => bail!("exported 'run' did not return a string: {other:?}"),
    };

    match serde_json::from_str::<Value>(&output) {
        Ok(v) => Ok(v),
        Err(_) => Ok(Value::String(output)),
    }
}

// ---------------------------------------------------------------------------
// Stub implementation when feature disabled
// ---------------------------------------------------------------------------

#[cfg(not(feature = "wasm"))]
#[derive(Debug, Default, Clone)]
pub struct WasmExtensionRuntime {
    path: Option<PathBuf>,
}

#[cfg(not(feature = "wasm"))]
impl WasmExtensionRuntime {
    pub fn new() -> Self {
        Self { path: None }
    }

    pub fn load(_path: impl AsRef<Path>) -> Result<Self> {
        bail!("wasm feature not enabled")
    }

    pub fn from_file(_path: impl AsRef<Path>) -> Result<Self> {
        bail!("wasm feature not enabled")
    }

    pub fn from_path(_path: impl AsRef<Path>) -> Result<Self> {
        bail!("wasm feature not enabled")
    }

    pub fn open(_path: impl AsRef<Path>) -> Result<Self> {
        bail!("wasm feature not enabled")
    }

    pub fn load_module(&mut self, _path: impl AsRef<Path>) -> Result<()> {
        bail!("wasm feature not enabled")
    }

    pub fn is_loaded(&self) -> bool {
        false
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn run(&self, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }

    pub fn call(&self, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }

    pub fn execute(&self, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }

    pub fn run_json(&self, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }

    pub fn run_with_path(&self, _path: impl AsRef<Path>, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }

    pub fn run_module(_path: impl AsRef<Path>, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }

    pub fn call_run(_path: impl AsRef<Path>, _input: Value) -> Result<Value> {
        bail!("wasm feature not enabled")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(feature = "wasm")]
    #[test]
    fn invalid_module_path_errors() {
        let err =
            WasmExtensionRuntime::run_module("/nonexistent/module.wasm", json!({})).unwrap_err();
        let msg = err.to_string();
        assert!(!msg.is_empty());
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    fn stub_returns_feature_not_enabled() {
        let err = WasmExtensionRuntime::run_module("/tmp/x.wasm", json!({})).unwrap_err();
        assert!(err.to_string().contains("feature not enabled"));
    }
}
