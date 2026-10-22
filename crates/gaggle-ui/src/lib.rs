use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct DesignTokens {
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub borders: BorderTokens,
    pub motion: MotionTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTokens {
    pub bg_app: &'static str,
    pub bg_panel: &'static str,
    pub bg_surface: &'static str,
    pub text_primary: &'static str,
    pub text_muted: &'static str,
    pub accent: &'static str,
    pub danger: &'static str,
    pub success: &'static str,
    pub border: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypographyTokens {
    pub font_ui: &'static str,
    pub font_mono: &'static str,
    pub size_xs: &'static str,
    pub size_sm: &'static str,
    pub size_md: &'static str,
    pub size_lg: &'static str,
    pub size_xl: &'static str,
    pub weight_regular: u16,
    pub weight_medium: u16,
    pub weight_bold: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingTokens {
    pub xs: &'static str,
    pub sm: &'static str,
    pub md: &'static str,
    pub lg: &'static str,
    pub xl: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderTokens {
    pub radius_sm: &'static str,
    pub radius_md: &'static str,
    pub radius_lg: &'static str,
    pub width: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionTokens {
    pub duration_fast: &'static str,
    pub duration_normal: &'static str,
    pub duration_slow: &'static str,
    pub easing_standard: &'static str,
}

pub fn light_theme_tokens() -> DesignTokens {
    DesignTokens {
        colors: ColorTokens {
            bg_app: "#f8f9fa",
            bg_panel: "#ffffff",
            bg_surface: "#f1f3f5",
            text_primary: "#212529",
            text_muted: "#5c646d",
            accent: "#1c7ed6",
            danger: "#e03131",
            success: "#2f9e44",
            border: "#dee2e6",
        },
        typography: TypographyTokens {
            font_ui: "Inter, -apple-system, BlinkMacSystemFont, sans-serif",
            font_mono: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, monospace",
            size_xs: "0.75rem",
            size_sm: "0.875rem",
            size_md: "1rem",
            size_lg: "1.125rem",
            size_xl: "1.25rem",
            weight_regular: 400,
            weight_medium: 500,
            weight_bold: 700,
        },
        spacing: SpacingTokens {
            xs: "0.25rem",
            sm: "0.5rem",
            md: "1rem",
            lg: "1.5rem",
            xl: "2rem",
        },
        borders: BorderTokens {
            radius_sm: "0.25rem",
            radius_md: "0.5rem",
            radius_lg: "1rem",
            width: "1px",
        },
        motion: MotionTokens {
            duration_fast: "100ms",
            duration_normal: "200ms",
            duration_slow: "350ms",
            easing_standard: "cubic-bezier(0.4, 0, 0.2, 1)",
        },
    }
}

pub fn dark_theme_tokens() -> DesignTokens {
    DesignTokens {
        colors: ColorTokens {
            bg_app: "#1a1b1e",
            bg_panel: "#25262b",
            bg_surface: "#2c2e33",
            text_primary: "#e9ecef",
            text_muted: "#909296",
            accent: "#4dabf7",
            danger: "#ff6b6b",
            success: "#51cf66",
            border: "#373a40",
        },
        typography: TypographyTokens {
            font_ui: "Inter, -apple-system, BlinkMacSystemFont, sans-serif",
            font_mono: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, monospace",
            size_xs: "0.75rem",
            size_sm: "0.875rem",
            size_md: "1rem",
            size_lg: "1.125rem",
            size_xl: "1.25rem",
            weight_regular: 400,
            weight_medium: 500,
            weight_bold: 700,
        },
        spacing: SpacingTokens {
            xs: "0.25rem",
            sm: "0.5rem",
            md: "1rem",
            lg: "1.5rem",
            xl: "2rem",
        },
        borders: BorderTokens {
            radius_sm: "0.25rem",
            radius_md: "0.5rem",
            radius_lg: "1rem",
            width: "1px",
        },
        motion: MotionTokens {
            duration_fast: "100ms",
            duration_normal: "200ms",
            duration_slow: "350ms",
            easing_standard: "cubic-bezier(0.4, 0, 0.2, 1)",
        },
    }
}

/// Formats a `DesignTokens` instance into CSS custom property declarations.
pub fn format_css_variables(tokens: &DesignTokens) -> String {
    format!(
        "--bg-app: {bg_app};\n      \
         --bg-panel: {bg_panel};\n      \
         --bg-surface: {bg_surface};\n      \
         --text-primary: {text_primary};\n      \
         --text-muted: {text_muted};\n      \
         --accent: {accent};\n      \
         --danger: {danger};\n      \
         --success: {success};\n      \
         --border: {border};\n      \
         --font-ui: {font_ui};\n      \
         --font-mono: {font_mono};\n      \
         --size-xs: {size_xs};\n      \
         --size-sm: {size_sm};\n      \
         --size-md: {size_md};\n      \
         --size-lg: {size_lg};\n      \
         --size-xl: {size_xl};\n      \
         --weight-regular: {weight_regular};\n      \
         --weight-medium: {weight_medium};\n      \
         --weight-bold: {weight_bold};\n      \
         --space-xs: {space_xs};\n      \
         --space-sm: {space_sm};\n      \
         --space-md: {space_md};\n      \
         --space-lg: {space_lg};\n      \
         --space-xl: {space_xl};\n      \
         --radius-sm: {radius_sm};\n      \
         --radius-md: {radius_md};\n      \
         --radius-lg: {radius_lg};\n      \
         --border-width: {border_width};\n      \
         --duration-fast: {duration_fast};\n      \
         --duration-normal: {duration_normal};\n      \
         --duration-slow: {duration_slow};\n      \
         --easing-standard: {easing_standard};",
        bg_app = tokens.colors.bg_app,
        bg_panel = tokens.colors.bg_panel,
        bg_surface = tokens.colors.bg_surface,
        text_primary = tokens.colors.text_primary,
        text_muted = tokens.colors.text_muted,
        accent = tokens.colors.accent,
        danger = tokens.colors.danger,
        success = tokens.colors.success,
        border = tokens.colors.border,
        font_ui = tokens.typography.font_ui,
        font_mono = tokens.typography.font_mono,
        size_xs = tokens.typography.size_xs,
        size_sm = tokens.typography.size_sm,
        size_md = tokens.typography.size_md,
        size_lg = tokens.typography.size_lg,
        size_xl = tokens.typography.size_xl,
        weight_regular = tokens.typography.weight_regular,
        weight_medium = tokens.typography.weight_medium,
        weight_bold = tokens.typography.weight_bold,
        space_xs = tokens.spacing.xs,
        space_sm = tokens.spacing.sm,
        space_md = tokens.spacing.md,
        space_lg = tokens.spacing.lg,
        space_xl = tokens.spacing.xl,
        radius_sm = tokens.borders.radius_sm,
        radius_md = tokens.borders.radius_md,
        radius_lg = tokens.borders.radius_lg,
        border_width = tokens.borders.width,
        duration_fast = tokens.motion.duration_fast,
        duration_normal = tokens.motion.duration_normal,
        duration_slow = tokens.motion.duration_slow,
        easing_standard = tokens.motion.easing_standard,
    )
}

pub fn render_index_html() -> String {
    let light_vars = format_css_variables(&light_theme_tokens());
    let dark_vars = format_css_variables(&dark_theme_tokens());

    let mut html = String::with_capacity(16384);
    html.push_str(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="system">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Gaggle — Self-Hosted AI Workspace Agent</title>
  <style>
    :root, [data-theme="light"] {
      "#,
    );
    html.push_str(&light_vars);
    html.push_str(
        r#"
    }
    [data-theme="dark"] {
      "#,
    );
    html.push_str(&dark_vars);
    html.push_str(
        r#"
    }
    @media (prefers-color-scheme: dark) {
      :root:not([data-theme="light"]) {
        "#,
    );
    html.push_str(&dark_vars);
    html.push_str(
        r#"
      }
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: var(--font-ui);
      font-size: var(--size-md);
      background: var(--bg-app);
      color: var(--text-primary);
      height: 100vh;
      overflow: hidden;
    }
    :focus-visible {
      outline: 2px solid var(--accent);
      outline-offset: 2px;
    }
    #app-shell {
      display: grid;
      grid-template-columns: 240px 1fr 320px;
      height: 100vh;
      gap: 0;
    }
    #sidebar {
      background: var(--bg-panel);
      border-right: var(--border-width) solid var(--border);
      padding: var(--space-md);
      overflow-y: auto;
      display: flex;
      flex-direction: column;
    }
    .sidebar-top-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: var(--space-sm);
    }
    .theme-select {
      font-family: var(--font-ui);
      font-size: var(--size-xs);
      background: var(--bg-surface);
      color: var(--text-primary);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-sm);
      padding: 0.2rem 0.4rem;
      cursor: pointer;
    }
    #main-workspace {
      display: flex;
      flex-direction: column;
      background: var(--bg-app);
    }
    #messages {
      flex: 1;
      overflow-y: auto;
      padding: var(--space-md);
    }
    #composer {
      padding: var(--space-md);
      border-top: var(--border-width) solid var(--border);
      background: var(--bg-panel);
    }
    #details-panel {
      background: var(--bg-panel);
      border-left: var(--border-width) solid var(--border);
      padding: var(--space-md);
      overflow-y: auto;
    }
    .panel-header {
      font-size: var(--size-sm);
      font-weight: var(--weight-bold);
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      margin-bottom: 0.75rem;
    }
    .empty-state {
      color: var(--text-muted);
      font-size: var(--size-sm);
      line-height: 1.5;
    }
    .composer-input {
      width: 100%;
      min-height: 80px;
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      background: var(--bg-surface);
      color: var(--text-primary);
      resize: vertical;
      font-family: var(--font-ui);
    }
    .composer-actions {
      display: flex;
      justify-content: flex-end;
      gap: var(--space-sm);
      margin-top: var(--space-sm);
    }
    .btn {
      border: none;
      border-radius: var(--radius-md);
      padding: 0.5rem 1rem;
      font-weight: var(--weight-medium);
      cursor: pointer;
      background: var(--accent);
      color: white;
      transition: opacity var(--duration-fast) var(--easing-standard);
    }
    .btn:disabled {
      opacity: 0.55;
      cursor: not-allowed;
    }
    .btn-secondary {
      background: var(--bg-surface);
      color: var(--text-primary);
      border: var(--border-width) solid var(--border);
    }
    .icon-btn {
      background: transparent;
      color: var(--text-primary);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-sm);
      padding: 0.25rem 0.5rem;
      cursor: pointer;
    }
    .card {
      background: var(--bg-surface);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
    }
    .badge, .status-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding: 0.125rem 0.5rem;
      border-radius: 9999px;
      font-size: var(--size-xs);
      font-weight: var(--weight-bold);
    }
    .status-running { background: var(--accent); color: white; }
    .status-waiting { background: #f08c00; color: white; }
    .status-completed { background: var(--success); color: white; }
    .status-failed { background: var(--danger); color: white; }
    .approval-card {
      background: var(--bg-surface);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      margin-bottom: 0.75rem;
    }
    .approval-card .action {
      font-weight: var(--weight-bold);
      margin-bottom: 0.25rem;
    }
    .approval-card .detail {
      font-size: var(--size-sm);
      color: var(--text-muted);
      word-break: break-all;
      font-family: var(--font-mono);
    }
    .approval-actions {
      display: flex;
      gap: var(--space-sm);
      margin-top: var(--space-sm);
    }
    .tool-call-card {
      background: var(--bg-surface);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      margin-bottom: var(--space-sm);
      font-family: var(--font-mono);
      font-size: var(--size-xs);
    }
    .tool-call-card .tool-name {
      font-weight: var(--weight-bold);
      color: var(--accent);
    }
    .tool-call-card .tool-status {
      display: inline-block;
      padding: 0.125rem 0.375rem;
      border-radius: var(--radius-sm);
      font-size: 0.7rem;
      font-weight: var(--weight-bold);
      margin-left: var(--space-sm);
    }
    .tool-status-ok { background: var(--success); color: white; }
    .tool-status-error { background: var(--danger); color: white; }
    .tool-status-pending { background: #f08c00; color: white; }
    .message {
      margin-bottom: var(--space-md);
      line-height: 1.6;
    }
    .message.user {
      background: var(--bg-surface);
      border-radius: var(--radius-md);
      padding: 0.75rem 1rem;
    }
    .message.assistant {
      background: var(--bg-panel);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem 1rem;
    }
    .streaming-cursor::after {
      content: "▌";
      animation: blink 1s step-end infinite;
    }
    @keyframes blink {
      50% { opacity: 0; }
    }
    @media (prefers-reduced-motion: reduce) {
      *, *::before, *::after {
        animation-duration: 0.01ms !important;
        transition-duration: 0.01ms !important;
      }
      .streaming-cursor::after { animation: none; }
    }
    @media (max-width: 900px) {
      #app-shell {
        grid-template-columns: 1fr;
      }
      #sidebar, #details-panel {
        display: none;
      }
    }
  </style>
</head>
<body>
  <div id="app-shell">
    <aside id="sidebar" role="navigation" aria-label="Sidebar">
      <div class="sidebar-top-row">
        <div class="panel-header" style="margin-bottom:0;">Gaggle</div>
        <select id="theme-switcher" class="theme-select" aria-label="Color theme">
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>
      <button class="btn" style="width:100%;margin-bottom:1rem;" id="btn-new-chat">+ New Chat</button>
      <div class="empty-state" id="recent-chats">No recent conversations yet. Register a project to get started.</div>
    </aside>
    <main id="main-workspace" role="main" aria-label="Main Workspace">
      <div id="messages" role="log" aria-live="polite">
        <div class="empty-state">Welcome to Gaggle. Ask the assistant to inspect or modify your project.</div>
      </div>
      <div id="composer">
        <textarea class="composer-input" id="composer-text" placeholder="Describe a task for your project..." aria-label="Message composer"></textarea>
        <div class="composer-actions">
          <button class="btn btn-secondary" id="btn-stop" disabled>Stop</button>
          <button class="btn" id="btn-send">Send</button>
        </div>
      </div>
    </main>
    <aside id="details-panel" role="complementary" aria-label="Contextual Details">
      <div class="panel-header">Run Timeline</div>
      <div class="empty-state" id="run-timeline">No run in progress. Submit a message to start.</div>
      <div style="margin-top:1rem;" class="panel-header">Pending Approvals</div>
      <div id="approval-queue"></div>
      <div style="margin-top:1rem;" class="panel-header">Git Changes</div>
      <div id="git-changes" class="empty-state">No changes detected.</div>
    </aside>
  </div>
  <script>
    (function() {
      const API_BASE = window.location.origin;
      const messagesEl = document.getElementById('messages');
      const composerText = document.getElementById('composer-text');
      const btnSend = document.getElementById('btn-send');
      const btnStop = document.getElementById('btn-stop');
      const approvalQueue = document.getElementById('approval-queue');
      const themeSwitcher = document.getElementById('theme-switcher');

      let currentChatId = null;
      let currentRunId = null;
      let eventSource = null;

      // XSS-safe DOM element builder: never interprets strings as raw HTML
      function createEl(tag, attrs, children) {
        const node = document.createElement(tag);
        if (attrs) {
          Object.keys(attrs).forEach(function(key) {
            if (key === 'className') {
              node.className = attrs[key];
            } else if (key === 'textContent') {
              node.textContent = attrs[key];
            } else if (key.indexOf('on') === 0 && typeof attrs[key] === 'function') {
              node.addEventListener(key.slice(2).toLowerCase(), attrs[key]);
            } else if (attrs[key] !== undefined && attrs[key] !== null) {
              node.setAttribute(key, String(attrs[key]));
            }
          });
        }
        if (Array.isArray(children)) {
          children.forEach(function(child) {
            if (typeof child === 'string') {
              node.appendChild(document.createTextNode(child));
            } else if (child instanceof Node) {
              node.appendChild(child);
            }
          });
        }
        return node;
      }

      function applyTheme(theme) {
        const valid = (theme === 'light' || theme === 'dark' || theme === 'system') ? theme : 'system';
        document.documentElement.setAttribute('data-theme', valid);
        if (themeSwitcher && themeSwitcher.value !== valid) {
          themeSwitcher.value = valid;
        }
      }

      async function loadSavedTheme() {
        try {
          const res = await fetch(API_BASE + '/api/settings');
          if (!res.ok) return;
          const list = await res.json();
          const entry = list.find(function(item) { return item.key === 'theme'; });
          if (entry && entry.value) {
            const parsed = JSON.parse(entry.value);
            if (typeof parsed === 'string') {
              applyTheme(parsed);
            }
          }
        } catch (e) {
          console.error('Failed to load theme setting:', e);
        }
      }

      if (themeSwitcher) {
        themeSwitcher.addEventListener('change', async function() {
          const chosen = themeSwitcher.value;
          applyTheme(chosen);
          try {
            await fetch(API_BASE + '/api/settings', {
              method: 'PUT',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ key: 'theme', value: chosen })
            });
          } catch (e) {
            console.error('Failed to persist theme setting:', e);
          }
        });
      }

      loadSavedTheme();

      function appendMessage(role, text) {
        const div = createEl('div', {
          className: 'message ' + role,
          textContent: text
        });
        messagesEl.appendChild(div);
        messagesEl.scrollTop = messagesEl.scrollHeight;
      }

      function renderApprovalCard(approval) {
        const approvalId = String(approval.id || '');
        const summaryNode = createEl('div', {
          className: 'action',
          textContent: '⚠ ' + (approval.summary || 'Approval required')
        });
        const toolCallNode = createEl('div', {
          className: 'detail',
          textContent: 'Tool call: ' + (approval.tool_call_id || '—')
        });

        const children = [summaryNode, toolCallNode];
        if (approval.command) {
          children.push(createEl('div', {
            className: 'detail',
            textContent: 'Command: ' + String(approval.command)
          }));
        }

        const approveBtn = createEl('button', {
          className: 'btn',
          textContent: '✓ Approve',
          'aria-label': 'Approve tool call ' + approvalId,
          onclick: function() { window.__gaggle_decide(approvalId, true); }
        });
        const denyBtn = createEl('button', {
          className: 'btn btn-secondary',
          textContent: '✕ Deny',
          'aria-label': 'Deny tool call ' + approvalId,
          onclick: function() { window.__gaggle_decide(approvalId, false); }
        });

        children.push(createEl('div', { className: 'approval-actions' }, [approveBtn, denyBtn]));

        const card = createEl('div', {
          className: 'approval-card',
          id: 'approval-' + approvalId
        }, children);
        approvalQueue.appendChild(card);
      }

      window.__gaggle_decide = async function(approvalId, approved) {
        try {
          const res = await fetch(API_BASE + '/api/approvals/' + encodeURIComponent(approvalId) + '/decide', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ approved: approved })
          });
          if (!res.ok) throw new Error(await res.text());
          const card = document.getElementById('approval-' + approvalId);
          if (card) card.remove();
        } catch (e) {
          console.error('Approval decision failed:', e);
        }
      };

      function handleSSEEvent(event) {
        const data = JSON.parse(event.data);
        switch (event.type) {
          case 'message.delta':
            appendMessage('assistant', data.text);
            break;
          case 'tool.started':
            appendMessage('assistant', '[Tool: ' + data.tool_name + ']');
            break;
          case 'tool.completed':
            appendMessage('assistant', '[Tool completed in ' + data.duration_ms + 'ms]');
            break;
          case 'tool.failed':
            appendMessage('assistant', '[Tool failed: ' + (data.error || 'unknown') + ']');
            break;
          case 'tool.denied':
            appendMessage('assistant', '[Tool denied: ' + (data.reason || 'unknown') + ']');
            break;
          case 'approval.requested':
            renderApprovalCard(data);
            break;
          case 'run.completed':
            appendMessage('assistant', '[Run completed]');
            btnStop.disabled = true;
            break;
          case 'run.cancelled':
            appendMessage('assistant', '[Run cancelled]');
            btnStop.disabled = true;
            break;
          case 'run.failed':
            appendMessage('assistant', '[Run failed: ' + (data.error || 'unknown') + ']');
            btnStop.disabled = true;
            break;
        }
      }

      btnSend.addEventListener('click', async function() {
        const prompt = composerText.value.trim();
        if (!prompt) return;
        if (!currentChatId) {
          appendMessage('assistant', 'No active chat. Create a project and chat first.');
          return;
        }
        appendMessage('user', prompt);
        composerText.value = '';
        btnSend.disabled = true;
        btnStop.disabled = false;

        try {
          const res = await fetch(API_BASE + '/api/chats/' + encodeURIComponent(currentChatId) + '/messages', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ prompt: prompt, model_key: 'local-mock' })
          });
          if (!res.ok) throw new Error(await res.text());
          const outcome = await res.json();
          if (outcome.run && outcome.run.id) {
            currentRunId = outcome.run.id;
            if (eventSource) eventSource.close();
            eventSource = new EventSource(API_BASE + '/api/runs/' + encodeURIComponent(currentRunId) + '/events');
            eventSource.addEventListener('message.delta', handleSSEEvent);
            eventSource.addEventListener('tool.started', handleSSEEvent);
            eventSource.addEventListener('tool.completed', handleSSEEvent);
            eventSource.addEventListener('tool.failed', handleSSEEvent);
            eventSource.addEventListener('tool.denied', handleSSEEvent);
            eventSource.addEventListener('approval.requested', handleSSEEvent);
            eventSource.addEventListener('run.completed', handleSSEEvent);
            eventSource.addEventListener('run.cancelled', handleSSEEvent);
            eventSource.addEventListener('run.failed', handleSSEEvent);
          }
        } catch (e) {
          appendMessage('assistant', 'Error: ' + e.message);
          btnStop.disabled = true;
        } finally {
          btnSend.disabled = false;
        }
      });

      btnStop.addEventListener('click', async function() {
        if (!currentRunId) return;
        try {
          await fetch(API_BASE + '/api/runs/' + encodeURIComponent(currentRunId) + '/cancel', { method: 'POST' });
        } catch (e) {
          console.error('Cancel failed:', e);
        }
      });

      composerText.addEventListener('keydown', function(e) {
        if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
          e.preventDefault();
          btnSend.click();
        }
      });
    })();
  </script>
</body>
</html>"#,
    );
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_design_tokens_and_xss_safe_html_generation() {
        let html = render_index_html();

        // All DesignTokens categories must be emitted into CSS custom properties
        assert!(html.contains("--font-ui:"));
        assert!(html.contains("--font-mono:"));
        assert!(html.contains("--size-xs:"));
        assert!(html.contains("--space-md:"));
        assert!(html.contains("--radius-md:"));
        assert!(html.contains("--duration-fast:"));
        assert!(html.contains("[data-theme=\"dark\"]"));
        assert!(html.contains(":focus-visible"));

        // Ensure no unsafe innerHTML assignments exist in the generated UI script
        assert!(!html.contains(".innerHTML"));
        assert!(html.contains("function createEl("));
    }
}
