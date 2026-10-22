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
  </style>
</head>
<body>
  <div id="app-shell">
    <aside id="sidebar" role="navigation" aria-label="Sidebar">
      <div class="panel-header">Gaggle</div>
      <button class="btn" style="width:100%;margin-bottom:1rem;">+ New Chat</button>
      <div class="empty-state">No recent conversations yet.<br/>Register a project to get started.</div>
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
      <div class="empty-state">No run in progress. Submit a message to start.</div>
      <div style="margin-top:1rem;" class="panel-header">Pending Approvals</div>
      <div class="empty-state">No pending approvals.</div>
      <div style="margin-top:1rem;" class="panel-header">Git Changes</div>
      <div class="empty-state">No changes detected.</div>
    </aside>
  </div>
</body>
</html>"#
}
