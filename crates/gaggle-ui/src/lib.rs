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

    let mut html = String::with_capacity(24576);
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
      grid-template-columns: 260px 1fr 340px;
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
      gap: var(--space-sm);
    }
    .sidebar-top-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .theme-select, .select-control, .text-input {
      font-family: var(--font-ui);
      font-size: var(--size-xs);
      background: var(--bg-surface);
      color: var(--text-primary);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-sm);
      padding: 0.35rem 0.5rem;
    }
    .select-control, .text-input {
      width: 100%;
      font-size: var(--size-sm);
    }
    .workspace-switcher-box {
      display: flex;
      flex-direction: column;
      gap: var(--space-xs);
      padding-bottom: var(--space-sm);
      border-bottom: var(--border-width) solid var(--border);
    }
    .workspace-switcher-row {
      display: flex;
      gap: var(--space-xs);
    }
    .mode-selectors-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: var(--space-xs);
    }
    .field-label {
      font-size: var(--size-xs);
      color: var(--text-muted);
      font-weight: var(--weight-medium);
      display: block;
      margin-bottom: 0.15rem;
    }
    #add-project-modal {
      background: var(--bg-surface);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: var(--space-sm);
      display: none;
      flex-direction: column;
      gap: var(--space-xs);
    }
    #add-project-modal.open {
      display: flex;
    }
    .error-banner, .warning-banner {
      font-size: var(--size-xs);
      padding: 0.4rem 0.6rem;
      border-radius: var(--radius-sm);
      border: var(--border-width) solid var(--danger);
      background: rgba(224, 49, 49, 0.12);
      color: var(--danger);
      display: none;
    }
    .error-banner.visible, .warning-banner.visible {
      display: block;
    }
    .chat-list {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }
    .chat-item {
      text-align: left;
      width: 100%;
      background: transparent;
      color: var(--text-primary);
      border: var(--border-width) solid transparent;
      border-radius: var(--radius-sm);
      padding: 0.45rem 0.6rem;
      cursor: pointer;
      font-size: var(--size-sm);
      display: flex;
      flex-direction: column;
      gap: 0.15rem;
    }
    .chat-item:hover {
      background: var(--bg-surface);
    }
    .chat-item[aria-current="page"] {
      background: var(--bg-surface);
      border-color: var(--accent);
      font-weight: var(--weight-medium);
    }
    .chat-item-meta {
      font-size: var(--size-xs);
      color: var(--text-muted);
      font-family: var(--font-mono);
    }
    #main-workspace {
      display: flex;
      flex-direction: column;
      background: var(--bg-app);
      min-width: 0;
    }
    #workspace-header {
      background: var(--bg-panel);
      border-bottom: var(--border-width) solid var(--border);
      padding: 0.65rem var(--space-md);
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: var(--space-sm);
    }
    .workspace-header-left {
      display: flex;
      flex-direction: column;
      gap: 0.15rem;
      min-width: 0;
    }
    #active-project-name {
      font-weight: var(--weight-bold);
      font-size: var(--size-md);
    }
    #active-project-path {
      font-family: var(--font-mono);
      font-size: var(--size-xs);
      color: var(--text-muted);
      word-break: break-all;
    }
    .workspace-header-badges {
      display: flex;
      flex-wrap: wrap;
      gap: var(--space-xs);
      align-items: center;
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
    .file-list {
      display: flex;
      flex-direction: column;
      gap: 0.2rem;
      font-family: var(--font-mono);
      font-size: var(--size-xs);
    }
    .file-entry {
      display: flex;
      justify-content: space-between;
      padding: 0.2rem 0.35rem;
      border-radius: var(--radius-sm);
      background: var(--bg-surface);
    }
    .panel-header {
      font-size: var(--size-sm);
      font-weight: var(--weight-bold);
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      margin-bottom: 0.5rem;
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
      padding: 0.45rem 0.85rem;
      font-weight: var(--weight-medium);
      font-size: var(--size-sm);
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
      padding: 0.15rem 0.5rem;
      border-radius: 9999px;
      font-size: var(--size-xs);
      font-weight: var(--weight-bold);
      background: var(--bg-surface);
      color: var(--text-primary);
      border: var(--border-width) solid var(--border);
    }
    .status-running { background: var(--accent); color: white; border-color: var(--accent); }
    .status-waiting { background: #f08c00; color: white; border-color: #f08c00; }
    .status-completed { background: var(--success); color: white; border-color: var(--success); }
    .status-failed { background: var(--danger); color: white; border-color: var(--danger); }
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
        <button id="btn-toggle-sidebar" class="icon-btn" type="button" aria-label="Toggle sidebar panel" aria-expanded="true">☰</button>
        <div class="panel-header" style="margin-bottom:0;">Gaggle</div>
        <select id="theme-switcher" class="theme-select" aria-label="Color theme">
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>

      <div class="workspace-switcher-box" aria-label="Workspace Switcher">
        <label class="field-label" for="project-switcher">Workspace Project</label>
        <div class="workspace-switcher-row">
          <select id="project-switcher" class="select-control" aria-label="Select active project">
            <option value="">No project selected</option>
          </select>
          <button class="btn btn-secondary" id="btn-open-add-project" type="button" aria-label="Add project">+ Add</button>
        </div>

        <form id="add-project-modal" aria-label="Register new project">
          <label class="field-label" for="input-project-name">Project Name</label>
          <input id="input-project-name" class="text-input" type="text" placeholder="e.g. gaggle" required />
          <label class="field-label" for="input-project-path">Root Directory Path</label>
          <input id="input-project-path" class="text-input" type="text" placeholder="/Users/name/projects/repo" required />
          <label class="field-label" for="select-project-vcs">Version Control</label>
          <select id="select-project-vcs" class="select-control">
            <option value="git">git</option>
            <option value="none">none</option>
          </select>
          <div id="add-project-error" class="error-banner" role="alert"></div>
          <div class="workspace-switcher-row" style="margin-top:0.25rem;">
            <button class="btn" id="btn-submit-project" type="submit" style="flex:1;">Register</button>
            <button class="btn btn-secondary" id="btn-cancel-project" type="button">Cancel</button>
          </div>
        </form>
      </div>

      <div class="workspace-switcher-box" aria-label="New Chat Configuration">
        <div class="mode-selectors-grid">
          <div>
            <label class="field-label" for="select-chat-mode">Execution Mode</label>
            <select id="select-chat-mode" class="select-control" aria-label="Execution mode">
              <option value="local">local</option>
              <option value="worktree">worktree</option>
              <option value="readonly">readonly</option>
            </select>
          </div>
          <div>
            <label class="field-label" for="select-permission-mode">Permission Mode</label>
            <select id="select-permission-mode" class="select-control" aria-label="Permission mode">
              <option value="ask">ask</option>
              <option value="smart">smart</option>
              <option value="full">full</option>
              <option value="chat_only">chat_only</option>
            </select>
          </div>
        </div>
        <button class="btn" style="width:100%;margin-top:0.25rem;" id="btn-new-chat" type="button">+ New Chat</button>
      </div>

      <div class="panel-header" style="margin-bottom:0.25rem;">Conversations</div>
      <div id="recent-chats" class="chat-list">
        <div class="empty-state">No recent conversations yet. Register a project to get started.</div>
      </div>
    </aside>

    <main id="main-workspace" role="main" aria-label="Main Workspace">
      <div id="reconnect-banner" class="warning-banner" role="alert" aria-live="assertive" style="display:none;">
        Reconnecting to local Gaggle server...
      </div>
      <div id="connection-status" class="badge" data-state="connected" aria-live="polite" style="align-self:flex-end;margin:0.5rem 0.5rem 0 0;">Connected</div>
      <header id="workspace-header" aria-label="Active Workspace Context">
        <div class="workspace-header-left">
          <div id="active-project-name">No Project Selected</div>
          <div id="active-project-path">Register or select a workspace folder in the sidebar</div>
          <div id="workspace-path-warning" class="warning-banner" role="alert">
            ⚠ Workspace root directory is missing or inaccessible on disk.
          </div>
        </div>
        <div class="workspace-header-badges">
          <button id="btn-rename-chat" class="btn btn-secondary" type="button" aria-label="Rename conversation" title="Rename conversation">✏</button>
          <span id="badge-vcs-kind" class="badge">vcs: —</span>
          <span id="badge-exec-mode" class="badge">mode: local</span>
          <span id="badge-perm-mode" class="badge">access: ask</span>
        </div>
      </header>

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

    <aside id="details-panel" role="complementary" aria-label="Contextual Details" aria-hidden="false">
      <div class="workspace-header-row">
        <div class="panel-header" style="margin-bottom:0;">Inspector</div>
        <button id="btn-toggle-inspector" class="icon-btn" type="button" aria-label="Toggle contextual details panel" aria-expanded="true">⚙</button>
      </div>
      <div class="panel-header">Workspace Files</div>
      <div id="workspace-files" class="file-list">
        <div class="empty-state">Select a project to inspect workspace files.</div>
      </div>
      <div style="margin-top:1rem;" class="panel-header">Run Timeline</div>
      <div class="empty-state" id="run-timeline">No run in progress. Submit a message to start.</div>
      <button id="btn-retry" class="btn btn-secondary" type="button" aria-label="Retry failed run" style="display:none;">Retry</button>
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

      const projectSwitcher = document.getElementById('project-switcher');
      const btnOpenAddProject = document.getElementById('btn-open-add-project');
      const addProjectModal = document.getElementById('add-project-modal');
      const inputProjectName = document.getElementById('input-project-name');
      const inputProjectPath = document.getElementById('input-project-path');
      const selectProjectVcs = document.getElementById('select-project-vcs');
      const addProjectError = document.getElementById('add-project-error');
      const btnCancelProject = document.getElementById('btn-cancel-project');

      const selectChatMode = document.getElementById('select-chat-mode');
      const selectPermissionMode = document.getElementById('select-permission-mode');
      const btnNewChat = document.getElementById('btn-new-chat');
      const recentChatsEl = document.getElementById('recent-chats');

      const activeProjectNameEl = document.getElementById('active-project-name');
      const activeProjectPathEl = document.getElementById('active-project-path');
      const workspacePathWarningEl = document.getElementById('workspace-path-warning');
      const badgeVcsKindEl = document.getElementById('badge-vcs-kind');
      const badgeExecModeEl = document.getElementById('badge-exec-mode');
      const badgePermModeEl = document.getElementById('badge-perm-mode');
      const workspaceFilesEl = document.getElementById('workspace-files');
      const runTimelineEl = document.getElementById('run-timeline');
      const gitChangesEl = document.getElementById('git-changes');
      const btnToggleSidebar = document.getElementById('btn-toggle-sidebar');
      const btnToggleInspector = document.getElementById('btn-toggle-inspector');
      const connectionStatusEl = document.getElementById('connection-status');
      const reconnectBannerEl = document.getElementById('reconnect-banner');

      let projectsById = {};
      let chatsById = {};
      let worktreesByChatId = {};
      let currentProjectId = null;
      let currentChatId = null;
      let currentRunId = null;
      let eventSource = null;
      let activeAssistantMessageEl = null;
      let lastUserPrompt = '';
      let userScrolledUpDuringStream = false;
      let lastMessageSeq = 0;

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

      function escapeHtml(value) {
        return String(value || '')
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;')
          .replace(/"/g, '&quot;')
          .replace(/'/g, '&#39;');
      }

      function formatInlineMarkdownSafe(text) {
        const escaped = escapeHtml(text);
        return escaped
          .replace(/`([^`]+)`/g, '<code>$1</code>')
          .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
      }

      function renderMarkdownSafe(text) {
        const normalized = String(text || '');
        const fencedRegex = /```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g;
        let lastIndex = 0;
        const fragments = [];
        let match;

        while ((match = fencedRegex.exec(normalized)) !== null) {
          const before = normalized.slice(lastIndex, match.index);
          if (before.trim()) {
            fragments.push('<p>' + formatInlineMarkdownSafe(before).replace(/\n/g, '<br/>') + '</p>');
          }
          const language = match[1] || 'text';
          const code = match[2].replace(/\n$/, '');
          const escapedCode = escapeHtml(code);
          fragments.push(
            '<div class="code-block">' +
            '<div class="code-header">' +
            '<span class="code-language">' + escapeHtml(language) + '</span>' +
            '<button class="btn btn-secondary code-copy-btn" type="button" aria-label="Copy code to clipboard" data-code="' + encodeURIComponent(code) + '">Copy</button>' +
            '</div>' +
            '<pre><code class="language-' + escapeHtml(language) + '">' + escapedCode + '</code></pre>' +
            '</div>'
          );
          lastIndex = match.index + match[0].length;
        }

        const tail = normalized.slice(lastIndex);
        if (tail.trim()) {
          fragments.push('<p>' + formatInlineMarkdownSafe(tail).replace(/\n/g, '<br/>') + '</p>');
        }

        return fragments.join('') || '<p></p>';
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

      function updateWorkspaceHeader() {
        const proj = currentProjectId ? projectsById[currentProjectId] : null;
        const chat = currentChatId ? chatsById[currentChatId] : null;

        if (!proj) {
          activeProjectNameEl.textContent = 'No Project Selected';
          activeProjectPathEl.textContent = 'Register or select a workspace folder in the sidebar';
          workspacePathWarningEl.classList.remove('visible');
          badgeVcsKindEl.textContent = 'vcs: —';
          badgeExecModeEl.textContent = 'mode: ' + selectChatMode.value;
          badgePermModeEl.textContent = 'access: ' + selectPermissionMode.value;
          return;
        }

        activeProjectNameEl.textContent = proj.name + (chat ? ' — ' + chat.title : '');
        activeProjectPathEl.textContent = proj.root_path;

        if (proj.path_exists === false) {
          workspacePathWarningEl.classList.add('visible');
        } else {
          workspacePathWarningEl.classList.remove('visible');
        }

        badgeVcsKindEl.textContent = 'vcs: ' + (proj.vcs_kind || 'git');

        const activeMode = chat ? chat.mode : selectChatMode.value;
        const wt = (chat && worktreesByChatId[chat.id]) ? worktreesByChatId[chat.id] : null;
        if (activeMode === 'worktree' && wt && wt.branch) {
          badgeExecModeEl.textContent = 'mode: worktree (' + wt.branch + ')';
        } else {
          badgeExecModeEl.textContent = 'mode: ' + activeMode;
        }

        const activePerm = chat ? chat.permission_mode : selectPermissionMode.value;
        badgePermModeEl.textContent = 'access: ' + activePerm;
      }

      async function loadWorkspaceFiles(projectId) {
        workspaceFilesEl.textContent = '';
        if (!projectId) {
          workspaceFilesEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: 'Select a project to inspect workspace files.'
          }));
          return;
        }
        try {
          const res = await fetch(API_BASE + '/api/projects/' + encodeURIComponent(projectId) + '/files');
          if (!res.ok) {
            const errText = await res.text();
            workspaceFilesEl.appendChild(createEl('div', {
              className: 'empty-state',
              textContent: 'Unable to list files: ' + errText
            }));
            return;
          }
          const entries = await res.json();
          if (!entries.length) {
            workspaceFilesEl.appendChild(createEl('div', {
              className: 'empty-state',
              textContent: 'Workspace directory is empty.'
            }));
            return;
          }
          entries.forEach(function(entry) {
            const label = (entry.is_dir ? '📁 ' : '📄 ') + entry.relative_path;
            const sizeLabel = entry.is_dir ? 'dir' : (entry.size_bytes + ' B');
            workspaceFilesEl.appendChild(createEl('div', { className: 'file-entry' }, [
              createEl('span', { textContent: label }),
              createEl('span', { textContent: sizeLabel })
            ]));
          });
        } catch (e) {
          workspaceFilesEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: 'Error loading files: ' + e.message
          }));
        }
      }

      async function loadProjectWorktrees(projectId) {
        worktreesByChatId = {};
        if (!projectId) return;
        try {
          const res = await fetch(API_BASE + '/api/projects/' + encodeURIComponent(projectId) + '/worktrees');
          if (!res.ok) return;
          const list = await res.json();
          list.forEach(function(wt) {
            worktreesByChatId[wt.chat_id] = wt;
          });
        } catch (e) {
          console.error('Failed to load worktrees:', e);
        }
      }

      function renderChatList(chats) {
        recentChatsEl.textContent = '';
        if (!chats.length) {
          recentChatsEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: 'No conversations in this project yet. Click + New Chat to start.'
          }));
          return;
        }
        chats.forEach(function(chat) {
          const wt = worktreesByChatId[chat.id];
          const modeLabel = (chat.mode === 'worktree' && wt) ? ('worktree: ' + wt.branch) : chat.mode;
          const btn = createEl('button', {
            type: 'button',
            className: 'chat-item',
            'aria-current': chat.id === currentChatId ? 'page' : 'false',
            onclick: function() { selectChat(chat.id); }
          }, [
            createEl('span', { textContent: chat.title }),
            createEl('span', { className: 'chat-item-meta', textContent: modeLabel + ' • ' + chat.permission_mode })
          ]);
          recentChatsEl.appendChild(btn);
        });
      }

      function selectChat(chatId) {
        currentChatId = chatId;
        const chatList = Object.keys(chatsById).map(function(id) { return chatsById[id]; });
        renderChatList(chatList);
        updateWorkspaceHeader();
        hydrateChatHistory(chatId);
      }

      async function hydrateChatHistory(chatId) {
        messagesEl.textContent = '';
        lastMessageSeq = 0;
        if (!chatId) return;
        try {
          const res = await fetch(API_BASE + '/api/chats/' + encodeURIComponent(chatId) + '/messages');
          if (!res.ok) return;
          const data = await res.json();
          (data.messages || []).forEach(function(msg) {
            const parts = (data.message_parts || []).filter(function(p) { return p.message_id === msg.id; });
            const textContent = parts.map(function(p) { return p.content || ''; }).join('\n');
            appendMessage(msg.role, textContent, false);
          });
          try {
            const approvalsRes = await fetch(API_BASE + '/api/approvals/pending');
            if (approvalsRes.ok) {
              const pending = await approvalsRes.json();
              pending.forEach(renderApprovalCard);
            }
          } catch (e) {
            console.error('Failed to hydrate approvals:', e);
          }
        } catch (e) {
          console.error('Failed to hydrate chat history:', e);
        }
      }

      async function selectProject(projectId) {
        // Strictly isolate context when switching between projects
        if (eventSource) {
          eventSource.close();
          eventSource = null;
        }
        currentProjectId = projectId || null;
        currentChatId = null;
        currentRunId = null;
        chatsById = {};
        messagesEl.textContent = '';

        if (projectSwitcher && projectSwitcher.value !== (projectId || '')) {
          projectSwitcher.value = projectId || '';
        }

        if (!currentProjectId) {
          updateWorkspaceHeader();
          renderChatList([]);
          await loadWorkspaceFiles(null);
          return;
        }

        const proj = projectsById[currentProjectId];
        if (proj) {
          selectChatMode.value = proj.default_mode || 'local';
          selectPermissionMode.value = proj.default_access || 'ask';
        }

        await loadProjectWorktrees(currentProjectId);

        try {
          const res = await fetch(API_BASE + '/api/projects/' + encodeURIComponent(currentProjectId) + '/chats');
          if (res.ok) {
            const chats = await res.json();
            chats.forEach(function(c) { chatsById[c.id] = c; });
            if (chats.length > 0) {
              currentChatId = chats[0].id;
            }
            renderChatList(chats);
          }
        } catch (e) {
          console.error('Failed to load project chats:', e);
        }

        updateWorkspaceHeader();
        if (currentChatId) {
          await hydrateChatHistory(currentChatId);
        }
        await loadWorkspaceFiles(currentProjectId);
      }

      async function loadProjects(preferredProjectId) {
        try {
          const res = await fetch(API_BASE + '/api/projects');
          if (!res.ok) return;
          const projects = await res.json();
          projectsById = {};
          projectSwitcher.textContent = '';

          if (projects.length === 0) {
            projectSwitcher.appendChild(createEl('option', {
              value: '',
              textContent: 'No projects registered'
            }));
            await selectProject(null);
            return;
          }

          projects.forEach(function(p) {
            projectsById[p.id] = p;
            projectSwitcher.appendChild(createEl('option', {
              value: p.id,
              textContent: p.name + ' (' + p.vcs_kind + ')'
            }));
          });

          const targetId = (preferredProjectId && projectsById[preferredProjectId])
            ? preferredProjectId
            : projects[0].id;
          await selectProject(targetId);
        } catch (e) {
          console.error('Failed to load projects:', e);
        }
      }

      projectSwitcher.addEventListener('change', function() {
        selectProject(projectSwitcher.value);
      });

      btnOpenAddProject.addEventListener('click', function() {
        addProjectError.textContent = '';
        addProjectError.classList.remove('visible');
        addProjectModal.classList.toggle('open');
        if (addProjectModal.classList.contains('open')) {
          inputProjectName.focus();
        }
      });

      btnCancelProject.addEventListener('click', function() {
        addProjectModal.classList.remove('open');
        addProjectError.textContent = '';
        addProjectError.classList.remove('visible');
      });

      addProjectModal.addEventListener('submit', async function(e) {
        e.preventDefault();
        addProjectError.textContent = '';
        addProjectError.classList.remove('visible');

        const payload = {
          name: inputProjectName.value.trim(),
          root_path: inputProjectPath.value.trim(),
          vcs_kind: selectProjectVcs.value
        };

        try {
          const res = await fetch(API_BASE + '/api/projects', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
          });
          if (!res.ok) {
            const errMsg = await res.text();
            addProjectError.textContent = errMsg || 'Failed to register project';
            addProjectError.classList.add('visible');
            return;
          }
          const created = await res.json();
          inputProjectName.value = '';
          inputProjectPath.value = '';
          addProjectModal.classList.remove('open');
          await loadProjects(created.id);
        } catch (err) {
          addProjectError.textContent = err.message;
          addProjectError.classList.add('visible');
        }
      });

      btnNewChat.addEventListener('click', async function() {
        if (!currentProjectId) {
          addProjectModal.classList.add('open');
          addProjectError.textContent = 'Register or select a project first before creating a chat.';
          addProjectError.classList.add('visible');
          return;
        }
        try {
          const res = await fetch(API_BASE + '/api/chats', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              project_id: currentProjectId,
              title: 'Chat ' + (Object.keys(chatsById).length + 1),
              mode: selectChatMode.value,
              permission_mode: selectPermissionMode.value
            })
          });
          if (!res.ok) {
            appendMessage('assistant', 'Failed to create chat: ' + (await res.text()));
            return;
          }
          const createdChat = await res.json();
          chatsById[createdChat.id] = createdChat;
          await loadProjectWorktrees(currentProjectId);
          selectChat(createdChat.id);
        } catch (e) {
          appendMessage('assistant', 'Error creating chat: ' + e.message);
        }
      });

      selectChatMode.addEventListener('change', updateWorkspaceHeader);
      selectPermissionMode.addEventListener('change', updateWorkspaceHeader);

      loadSavedTheme();
      loadProjects(null);

      function appendMessage(role, text, autoScroll) {
        if (autoScroll === undefined) autoScroll = true;
        const div = createEl('div', { className: 'message ' + role });
        if (role === 'assistant' && text === '') {
          div.classList.add('streaming-cursor');
        }
        div.innerHTML = renderMarkdownSafe(text);
        div.querySelectorAll('.code-copy-btn').forEach(function(btn) {
          btn.addEventListener('click', function() {
            const code = decodeURIComponent(btn.getAttribute('data-code') || '');
            navigator.clipboard.writeText(code).then(function() {
              const original = btn.textContent;
              btn.textContent = 'Copied!';
              setTimeout(function() { btn.textContent = original; }, 1500);
            });
          });
        });
        messagesEl.appendChild(div);
        if (autoScroll) {
          scrollToBottomIfNeeded();
        }
        return div;
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
            if (!activeAssistantMessageEl) {
              activeAssistantMessageEl = appendMessage('assistant', '', true);
            }
            activeAssistantMessageEl.innerHTML = renderMarkdownSafe(
              (activeAssistantMessageEl.dataset.rawText || '') + (data.text || '')
            );
            activeAssistantMessageEl.dataset.rawText = (activeAssistantMessageEl.dataset.rawText || '') + (data.text || '');
            activeAssistantMessageEl.querySelectorAll('.code-copy-btn').forEach(function(btn) {
              btn.addEventListener('click', function() {
                const code = decodeURIComponent(btn.getAttribute('data-code') || '');
                navigator.clipboard.writeText(code).then(function() {
                  const original = btn.textContent;
                  btn.textContent = 'Copied!';
                  setTimeout(function() { btn.textContent = original; }, 1500);
                });
              });
            });
            scrollToBottomIfNeeded();
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
            if (activeAssistantMessageEl) {
              activeAssistantMessageEl.classList.remove('streaming-cursor');
              activeAssistantMessageEl = null;
            }
            appendMessage('assistant', '[Run completed]');
            btnStop.disabled = true;
            break;
          case 'run.cancelled':
            if (activeAssistantMessageEl) {
              activeAssistantMessageEl.classList.remove('streaming-cursor');
              activeAssistantMessageEl = null;
            }
            appendMessage('assistant', '[Run cancelled]');
            btnStop.disabled = true;
            break;
          case 'run.failed':
            if (activeAssistantMessageEl) {
              activeAssistantMessageEl.classList.remove('streaming-cursor');
              activeAssistantMessageEl = null;
            }
            appendMessage('assistant', '[Run failed: ' + (data.error || 'unknown') + ']');
            btnStop.disabled = true;
            break;
        }
      }

      function isNearBottom() {
        return messagesEl.scrollHeight - messagesEl.scrollTop - messagesEl.clientHeight < 80;
      }

      function scrollToBottomIfNeeded() {
        if (userScrolledUpDuringStream) return;
        if (isNearBottom()) {
          messagesEl.scrollTop = messagesEl.scrollHeight;
        }
      }

      btnSend.addEventListener('click', async function() {
        const prompt = composerText.value.trim();
        if (!prompt) return;
        if (!currentChatId) {
          appendMessage('assistant', 'No active chat. Create a project and chat first.');
          return;
        }
        lastUserPrompt = prompt;
        appendMessage('user', prompt);
        composerText.value = '';
        btnSend.disabled = true;
        btnStop.disabled = false;
        userScrolledUpDuringStream = false;
        activeAssistantMessageEl = null;

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
          // Preserve the user's prompt for retry on transient errors
          if (!composerText.value) {
            composerText.value = lastUserPrompt;
          }
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

      messagesEl.addEventListener('scroll', function() {
        if (messagesEl.scrollHeight - messagesEl.scrollTop - messagesEl.clientHeight > 100) {
          userScrolledUpDuringStream = true;
        } else {
          userScrolledUpDuringStream = false;
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
        assert!(!html.contains("innerHTML = \""));
        assert!(html.contains("function createEl("));
    }

    #[test]
    fn test_workspace_switcher_add_project_modal_and_mode_selectors_present() {
        let html = render_index_html();

        // Sidebar workspace switcher & + Add Project modal controls
        assert!(html.contains("id=\"project-switcher\""));
        assert!(html.contains("id=\"btn-open-add-project\""));
        assert!(html.contains("id=\"add-project-modal\""));
        assert!(html.contains("id=\"input-project-name\""));
        assert!(html.contains("id=\"input-project-path\""));
        assert!(html.contains("id=\"select-project-vcs\""));
        assert!(html.contains("id=\"add-project-error\""));

        // Execution mode (local, worktree, readonly) & permission mode (ask, smart, full, chat_only)
        assert!(html.contains("id=\"select-chat-mode\""));
        assert!(html.contains("<option value=\"local\">local</option>"));
        assert!(html.contains("<option value=\"worktree\">worktree</option>"));
        assert!(html.contains("<option value=\"readonly\">readonly</option>"));

        assert!(html.contains("id=\"select-permission-mode\""));
        assert!(html.contains("<option value=\"ask\">ask</option>"));
        assert!(html.contains("<option value=\"smart\">smart</option>"));
        assert!(html.contains("<option value=\"full\">full</option>"));
        assert!(html.contains("<option value=\"chat_only\">chat_only</option>"));

        // Active workspace header & workspace files inspector
        assert!(html.contains("id=\"workspace-header\""));
        assert!(html.contains("id=\"active-project-name\""));
        assert!(html.contains("id=\"active-project-path\""));
        assert!(html.contains("id=\"workspace-path-warning\""));
        assert!(html.contains("id=\"workspace-files\""));
    }

    #[test]
    fn test_responsive_app_shell_drawer_toggles_and_connection_banner_present() {
        let html = render_index_html();

        // Header drawer toggle buttons for narrow viewports (<= 900px)
        assert!(html.contains("id=\"btn-toggle-sidebar\""));
        assert!(html.contains("id=\"btn-toggle-inspector\""));
        assert!(html.contains("aria-expanded"));

        // Connection health indicator and reconnection banner
        assert!(html.contains("id=\"connection-status\""));
        assert!(html.contains("id=\"reconnect-banner\""));

        // App shell structure with 3 panels
        assert!(html.contains("id=\"app-shell\""));
        assert!(html.contains("id=\"sidebar\""));
        assert!(html.contains("id=\"main-workspace\""));
        assert!(html.contains("id=\"details-panel\""));

        // Responsive media query for narrow windows
        assert!(html.contains("@media (max-width: 900px)"));
    }

    #[test]
    fn test_conversation_streaming_and_code_block_rendering() {
        let html = render_index_html();

        // SSE EventSource connection setup
        assert!(html.contains("new EventSource("));
        assert!(html.contains("addEventListener('message.delta'"));

        // XSS-safe Markdown rendering helper
        assert!(html.contains("function escapeHtml("));
        assert!(html.contains("function renderMarkdownSafe("));

        // Fenced code block with copy button
        assert!(html.contains("code-copy-btn"));
        assert!(html.contains("navigator.clipboard.writeText"));

        // Smart scroll lock
        assert!(html.contains("isNearBottom"));
        assert!(html.contains("userScrolledUpDuringStream"));

        // Chat rename and retry
        assert!(html.contains("btn-rename-chat"));
        assert!(html.contains("btn-retry"));
    }
}
