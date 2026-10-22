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
            text_muted: "#6c757d",
            accent: "#228be6",
            danger: "#fa5252",
            success: "#40c057",
            border: "#dee2e6",
        },
        typography: TypographyTokens {
            font_ui: "Inter, -apple-system, BlinkMacSystemFont, sans-serif",
            font_mono: "JetBrains Mono, monospace",
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
            text_primary: "#c1c2c5",
            text_muted: "#909296",
            accent: "#4dabf7",
            danger: "#ff6b6b",
            success: "#51cf66",
            border: "#373a40",
        },
        typography: TypographyTokens {
            font_ui: "Inter, -apple-system, BlinkMacSystemFont, sans-serif",
            font_mono: "JetBrains Mono, monospace",
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

pub fn render_index_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Gaggle — Self-Hosted AI Workspace Agent</title>
  <style>
    :root {
      --bg-app: #f8f9fa;
      --bg-panel: #ffffff;
      --bg-surface: #f1f3f5;
      --text-primary: #212529;
      --text-muted: #6c757d;
      --accent: #228be6;
      --danger: #fa5252;
      --success: #40c057;
      --border: #dee2e6;
      --radius-md: 0.5rem;
      --space-md: 1rem;
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --bg-app: #1a1b1e;
        --bg-panel: #25262b;
        --bg-surface: #2c2e33;
        --text-primary: #c1c2c5;
        --text-muted: #909296;
        --accent: #4dabf7;
        --danger: #ff6b6b;
        --success: #51cf66;
        --border: #373a40;
      }
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: Inter, -apple-system, BlinkMacSystemFont, sans-serif;
      background: var(--bg-app);
      color: var(--text-primary);
      height: 100vh;
      overflow: hidden;
    }
    #app-shell {
      display: grid;
      grid-template-columns: 240px 1fr 320px;
      height: 100vh;
      gap: 0;
    }
    #sidebar {
      background: var(--bg-panel);
      border-right: 1px solid var(--border);
      padding: var(--space-md);
      overflow-y: auto;
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
      border-top: 1px solid var(--border);
      background: var(--bg-panel);
    }
    #details-panel {
      background: var(--bg-panel);
      border-left: 1px solid var(--border);
      padding: var(--space-md);
      overflow-y: auto;
    }
    .panel-header {
      font-size: 0.875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      margin-bottom: 0.75rem;
    }
    .empty-state {
      color: var(--text-muted);
      font-size: 0.875rem;
      line-height: 1.5;
    }
    .composer-input {
      width: 100%;
      min-height: 80px;
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      background: var(--bg-surface);
      color: var(--text-primary);
      resize: vertical;
      font-family: inherit;
    }
    .composer-actions {
      display: flex;
      justify-content: flex-end;
      gap: 0.5rem;
      margin-top: 0.5rem;
    }
    .btn {
      border: none;
      border-radius: var(--radius-md);
      padding: 0.5rem 1rem;
      font-weight: 500;
      cursor: pointer;
      background: var(--accent);
      color: white;
    }
    .btn-secondary {
      background: var(--bg-surface);
      color: var(--text-primary);
      border: 1px solid var(--border);
    }
    .status-badge {
      display: inline-block;
      padding: 0.125rem 0.5rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 600;
    }
    .status-running { background: var(--accent); color: white; }
    .status-waiting { background: #f08c00; color: white; }
    .status-completed { background: var(--success); color: white; }
    .status-failed { background: var(--danger); color: white; }
    .approval-card {
      background: var(--bg-surface);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      margin-bottom: 0.75rem;
    }
    .approval-card .action {
      font-weight: 600;
      margin-bottom: 0.25rem;
    }
    .approval-card .detail {
      font-size: 0.875rem;
      color: var(--text-muted);
      word-break: break-all;
    }
    .approval-actions {
      display: flex;
      gap: 0.5rem;
      margin-top: 0.5rem;
    }
    .tool-call-card {
      background: var(--bg-surface);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      padding: 0.75rem;
      margin-bottom: 0.5rem;
      font-family: var(--font-mono);
      font-size: 0.8rem;
    }
    .tool-call-card .tool-name {
      font-weight: 600;
      color: var(--accent);
    }
    .tool-call-card .tool-status {
      display: inline-block;
      padding: 0.125rem 0.375rem;
      border-radius: 0.25rem;
      font-size: 0.7rem;
      font-weight: 600;
      margin-left: 0.5rem;
    }
    .tool-status-ok { background: var(--success); color: white; }
    .tool-status-error { background: var(--danger); color: white; }
    .tool-status-pending { background: #f08c00; color: white; }
    .message {
      margin-bottom: 1rem;
      line-height: 1.6;
    }
    .message.user {
      background: var(--bg-surface);
      border-radius: var(--radius-md);
      padding: 0.75rem 1rem;
    }
    .message.assistant {
      background: var(--bg-panel);
      border: 1px solid var(--border);
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
      <div class="panel-header">Gaggle</div>
      <button class="btn" style="width:100%;margin-bottom:1rem;" id="btn-new-chat">+ New Chat</button>
      <div class="empty-state" id="recent-chats">No recent conversations yet.<br/>Register a project to get started.</div>
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
      const runTimeline = document.getElementById('run-timeline');
      const gitChanges = document.getElementById('git-changes');

      let currentChatId = null;
      let currentRunId = null;
      let eventSource = null;

      function appendMessage(role, text) {
        const div = document.createElement('div');
        div.className = 'message ' + role;
        div.textContent = text;
        messagesEl.appendChild(div);
        messagesEl.scrollTop = messagesEl.scrollHeight;
      }

      function renderApprovalCard(approval) {
        const card = document.createElement('div');
        card.className = 'approval-card';
        card.id = 'approval-' + approval.id;
        card.innerHTML = '<div class="action">' + (approval.summary || 'Approval required') + '</div>'
          + '<div class="detail">Tool call: ' + (approval.tool_call_id || '—') + '</div>'
          + (approval.command ? '<div class="detail">Command: ' + approval.command + '</div>' : '')
          + '<div class="approval-actions">'
          + '<button class="btn" onclick="window.__gaggle_decide(\'' + approval.id + '\', true)">Approve</button>'
          + '<button class="btn btn-secondary" onclick="window.__gaggle_decide(\'' + approval.id + '\', false)">Deny</button>'
          + '</div>';
        approvalQueue.appendChild(card);
      }

      window.__gaggle_decide = async function(approvalId, approved) {
        try {
          const res = await fetch(API_BASE + '/api/approvals/' + approvalId + '/decide', {
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
          const res = await fetch(API_BASE + '/api/chats/' + currentChatId + '/messages', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ prompt: prompt, model_key: 'local-mock' })
          });
          if (!res.ok) throw new Error(await res.text());
          const outcome = await res.json();
          if (outcome.run && outcome.run.id) {
            currentRunId = outcome.run.id;
            if (eventSource) eventSource.close();
            eventSource = new EventSource(API_BASE + '/api/runs/' + currentRunId + '/events');
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
          await fetch(API_BASE + '/api/runs/' + currentRunId + '/cancel', { method: 'POST' });
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
</html>"#
}
