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
    .hidden { display: none !important; }
    #onboarding-card {
      margin: var(--space-md);
      border: var(--border-width) solid var(--accent);
      border-radius: var(--radius-md);
      background: var(--bg-panel);
      padding: var(--space-md);
    }
    .onboarding-header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: var(--space-sm);
    }
    .onboarding-steps {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: var(--space-sm);
      margin-top: var(--space-sm);
    }
    .onboarding-step {
      background: var(--bg-surface);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      padding: var(--space-sm);
      display: flex;
      flex-direction: column;
      gap: var(--space-xs);
    }
    .privacy-note {
      font-size: var(--size-xs);
      color: var(--text-muted);
      line-height: 1.55;
      margin: var(--space-sm) 0 0;
    }
    .composer-toolbar {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: var(--space-sm);
      margin-bottom: var(--space-sm);
    }
    .composer-toolbar .select-control {
      width: auto;
      min-width: 240px;
      max-width: 100%;
    }
    .modal-overlay {
      position: fixed;
      inset: 0;
      z-index: 1000;
      display: none;
      align-items: center;
      justify-content: center;
      padding: var(--space-md);
      background: rgba(0, 0, 0, 0.55);
    }
    .modal-overlay.open { display: flex; }
    .modal-dialog {
      width: min(940px, 100%);
      max-height: 88vh;
      display: flex;
      flex-direction: column;
      background: var(--bg-panel);
      color: var(--text-primary);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-lg);
      box-shadow: 0 18px 60px rgba(0, 0, 0, 0.35);
      overflow: hidden;
    }
    .modal-header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: var(--space-sm);
      padding: var(--space-md);
      border-bottom: var(--border-width) solid var(--border);
    }
    .modal-body {
      display: flex;
      flex: 1;
      min-height: 0;
    }
    .settings-tabs {
      display: flex;
      flex-direction: column;
      gap: 0.15rem;
      min-width: 190px;
      padding: var(--space-sm);
      border-right: var(--border-width) solid var(--border);
      background: var(--bg-app);
    }
    .settings-tab {
      text-align: left;
      background: transparent;
      color: var(--text-primary);
      border: var(--border-width) solid transparent;
      border-radius: var(--radius-sm);
      padding: 0.5rem 0.65rem;
      font-size: var(--size-sm);
      cursor: pointer;
    }
    .settings-tab[aria-selected="true"] {
      background: var(--bg-surface);
      border-color: var(--accent);
      font-weight: var(--weight-bold);
    }
    .settings-panels {
      flex: 1;
      min-width: 0;
      overflow-y: auto;
      padding: var(--space-md);
    }
    .settings-panel {
      display: none;
      flex-direction: column;
      gap: var(--space-md);
    }
    .settings-panel.active { display: flex; }
    .settings-section {
      display: flex;
      flex-direction: column;
      gap: var(--space-sm);
      padding: var(--space-sm);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      background: var(--bg-surface);
    }
    .settings-section h3 {
      margin: 0;
      font-size: var(--size-sm);
      text-transform: uppercase;
      letter-spacing: 0.04em;
      color: var(--text-muted);
    }
    .form-row {
      display: flex;
      flex-wrap: wrap;
      gap: var(--space-sm);
      align-items: flex-end;
    }
    .form-row > div {
      flex: 1 1 170px;
      min-width: 0;
    }
    .settings-note {
      margin: 0;
      font-size: var(--size-xs);
      color: var(--text-muted);
      line-height: 1.55;
    }
    .data-table {
      width: 100%;
      border-collapse: collapse;
      font-size: var(--size-xs);
    }
    .data-table th, .data-table td {
      text-align: left;
      padding: 0.35rem 0.45rem;
      border-bottom: var(--border-width) solid var(--border);
      vertical-align: top;
      word-break: break-word;
    }
    .data-table th {
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.04em;
      font-size: 0.7rem;
    }
    .data-table tr:last-child td { border-bottom: none; }
    .status-ok { background: var(--success); color: white; border-color: var(--success); }
    .status-warn { background: #f08c00; color: white; border-color: #f08c00; }
    .status-danger { background: var(--danger); color: white; border-color: var(--danger); }
    .provider-card {
      display: flex;
      flex-direction: column;
      gap: 0.35rem;
      padding: var(--space-sm);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-md);
      background: var(--bg-panel);
    }
    .provider-card + .provider-card { margin-top: var(--space-sm); }
    .provider-card-head {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: var(--space-xs);
    }
    .mono { font-family: var(--font-mono); word-break: break-all; }
    .inline-actions {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: var(--space-sm);
    }
    .settings-toolbar {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: var(--space-sm);
    }
    .extension-row {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: var(--space-sm);
      padding: 0.4rem 0;
      border-bottom: var(--border-width) solid var(--border);
    }
    .extension-row:last-child { border-bottom: none; }
    .extension-row-main { display: flex; flex-direction: column; gap: 0.1rem; min-width: 0; }
    .toggle-label { display: inline-flex; align-items: center; gap: 0.35rem; font-size: var(--size-xs); }
    .toast {
      position: fixed;
      right: 1rem;
      bottom: 1rem;
      z-index: 1100;
      max-width: min(420px, calc(100vw - 2rem));
      padding: 0.6rem 0.8rem;
      border-radius: var(--radius-md);
      border: var(--border-width) solid var(--border);
      background: var(--bg-panel);
      color: var(--text-primary);
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
      font-size: var(--size-sm);
      display: none;
    }
    .toast.visible { display: block; }
    .toast.error { border-color: var(--danger); color: var(--danger); }
    @media (max-width: 760px) {
      .modal-body { flex-direction: column; }
      .settings-tabs {
        flex-direction: row;
        min-width: 0;
        overflow-x: auto;
        border-right: none;
        border-bottom: var(--border-width) solid var(--border);
      }
      .settings-tab { white-space: nowrap; }
      .onboarding-steps { grid-template-columns: 1fr; }
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
        <button id="btn-open-settings" class="icon-btn" type="button" aria-label="Open settings and diagnostics" aria-expanded="false" title="Settings (Cmd/Ctrl + ,)">⚙</button>
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

      <section id="onboarding-card" class="hidden" aria-labelledby="onboarding-title">
        <div class="onboarding-header">
          <div>
            <div id="onboarding-title" class="panel-header" style="margin-bottom:0.25rem;">Welcome to Gaggle</div>
            <p class="settings-note">Two optional steps to get productive. You can skip provider setup and keep using the built-in <span class="mono">local-mock</span> model offline.</p>
          </div>
          <button id="btn-skip-onboarding" class="btn btn-secondary" type="button">Skip for now</button>
        </div>
        <div class="onboarding-steps">
          <div class="onboarding-step">
            <strong>1. Register your first project</strong>
            <p class="settings-note">Pick a local directory. Gaggle only reads and writes inside validated project roots.</p>
            <button id="btn-onboarding-add-project" class="btn" type="button">Register project</button>
          </div>
          <div class="onboarding-step">
            <strong>2. Configure an AI provider (optional)</strong>
            <p class="settings-note">Add OpenAI, Anthropic, OpenRouter, DeepSeek, Ollama, or an OpenAI-compatible endpoint, then choose a model in the composer.</p>
            <button id="btn-onboarding-provider" class="btn btn-secondary" type="button">Open provider settings</button>
          </div>
        </div>
        <p class="privacy-note">
          <strong>What stays local:</strong> <span class="mono">gaggle.db</span>, your workspace files, run history, and approvals never leave this machine by default.
          API keys are stored in the server credential store and referenced from SQLite only by <span class="mono">secret_ref</span>; the key value is never written to the database or rendered in the UI.
          <strong>What leaves this machine:</strong> when a remote provider is active, the prompts and workspace context included in a chat turn are sent to that provider for inference.
        </p>
      </section>

      <div id="messages" role="log" aria-live="polite">
        <div class="empty-state">Welcome to Gaggle. Ask the assistant to inspect or modify your project.</div>
      </div>
      <div id="composer">
        <div class="composer-toolbar">
          <label class="field-label" for="select-model" style="margin:0;">Model</label>
          <select id="select-model" class="select-control" aria-label="Active model used for chat submissions">
            <option value="local-mock">local-mock (offline built-in)</option>
          </select>
          <span id="model-status-note" class="settings-note">No remote model configured. Chat runs against the offline built-in model.</span>
        </div>
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

  <div id="settings-modal" class="modal-overlay">
    <div class="modal-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <div class="modal-header">
        <div>
          <div id="settings-title" class="panel-header" style="margin:0;">Settings &amp; Integrations</div>
          <div class="settings-note">Cmd/Ctrl + , opens this dialog · Esc closes it</div>
        </div>
        <button id="btn-close-settings" class="icon-btn" type="button" aria-label="Close settings">✕</button>
      </div>
      <div class="modal-body">
        <div class="settings-tabs" role="tablist" aria-label="Settings sections" aria-orientation="vertical">
          <button class="settings-tab" id="tab-providers" type="button" role="tab" data-settings-tab="providers" aria-selected="true" aria-controls="panel-providers">Providers &amp; Models</button>
          <button class="settings-tab" id="tab-permissions" type="button" role="tab" data-settings-tab="permissions" aria-selected="false" aria-controls="panel-permissions">Permissions &amp; Rules</button>
          <button class="settings-tab" id="tab-extensions" type="button" role="tab" data-settings-tab="extensions" aria-selected="false" aria-controls="panel-extensions">Extensions</button>
          <button class="settings-tab" id="tab-memory" type="button" role="tab" data-settings-tab="memory" aria-selected="false" aria-controls="panel-memory">Memory</button>
          <button class="settings-tab" id="tab-appearance" type="button" role="tab" data-settings-tab="appearance" aria-selected="false" aria-controls="panel-appearance">Appearance</button>
          <button class="settings-tab" id="tab-diagnostics" type="button" role="tab" data-settings-tab="diagnostics" aria-selected="false" aria-controls="panel-diagnostics">Diagnostics</button>
        </div>
        <div class="settings-panels">
          <section class="settings-panel active" id="panel-providers" role="tabpanel" aria-labelledby="tab-providers">
            <div class="settings-section">
              <h3>Add provider</h3>
              <form id="form-add-provider" class="form-row" autocomplete="off">
                <div>
                  <label class="field-label" for="input-provider-kind">Kind</label>
                  <select id="input-provider-kind" class="select-control">
                    <option value="openai">openai</option>
                    <option value="anthropic">anthropic</option>
                    <option value="chatgpt">chatgpt</option>
                    <option value="openrouter">openrouter</option>
                    <option value="deepseek">deepseek</option>
                    <option value="ollama">ollama</option>
                    <option value="openai_compat">openai_compat</option>
                  </select>
                </div>
                <div>
                  <label class="field-label" for="input-provider-name">Display name</label>
                  <input id="input-provider-name" class="text-input" type="text" placeholder="e.g. OpenAI production" required />
                </div>
                <div>
                  <label class="field-label" for="input-provider-base-url">Base URL (optional)</label>
                  <input id="input-provider-base-url" class="text-input" type="url" placeholder="https://api.openai.com/v1" autocomplete="off" />
                </div>
                <div>
                  <label class="field-label" for="input-provider-auth-kind">Auth kind</label>
                  <select id="input-provider-auth-kind" class="select-control">
                    <option value="api_key">api_key</option>
                    <option value="oauth">oauth</option>
                    <option value="none">none</option>
                  </select>
                </div>
                <div>
                  <label class="field-label" for="input-provider-api-key">API key</label>
                  <input id="input-provider-api-key" class="text-input" type="password" placeholder="Never stored in SQLite" autocomplete="off" spellcheck="false" />
                </div>
                <div>
                  <button id="btn-add-provider" class="btn" type="submit" style="width:100%;">Add provider</button>
                </div>
              </form>
              <div id="provider-form-status" class="error-banner" role="alert"></div>
              <p class="settings-note">
                The key is sent once to the local server, stored via the <span class="mono">CredentialStore</span>, and cleared from this form immediately.
                Only <span class="mono">secret_ref</span> and a boolean credential status are persisted. When a remote provider is selected, the prompts and
                workspace context included in a chat turn are sent to that provider for inference.
              </p>
            </div>
            <div class="settings-section">
              <div class="settings-toolbar">
                <h3>Configured providers</h3>
                <button id="btn-refresh-providers" class="btn btn-secondary" type="button">Refresh</button>
              </div>
              <div id="provider-list"><div class="empty-state">Loading providers…</div></div>
            </div>
            <div class="settings-section">
              <h3>Add model to provider</h3>
              <form id="form-add-model" class="form-row" autocomplete="off">
                <div>
                  <label class="field-label" for="input-model-provider">Provider</label>
                  <select id="input-model-provider" class="select-control"></select>
                </div>
                <div>
                  <label class="field-label" for="input-model-key">Model key</label>
                  <input id="input-model-key" class="text-input" type="text" placeholder="gpt-4o-mini" required />
                </div>
                <div>
                  <label class="field-label" for="input-model-name">Display name</label>
                  <input id="input-model-name" class="text-input" type="text" placeholder="GPT-4o mini" required />
                </div>
                <div>
                  <label class="field-label" for="input-model-context">Context window</label>
                  <input id="input-model-context" class="text-input" type="number" min="1" value="128000" required />
                </div>
                <div>
                  <label class="toggle-label" for="input-model-supports-tools">
                    <input id="input-model-supports-tools" type="checkbox" checked /> Supports tools
                  </label>
                </div>
                <div>
                  <button id="btn-add-model" class="btn" type="submit" style="width:100%;">Add model</button>
                </div>
              </form>
              <div id="model-form-status" class="error-banner" role="alert"></div>
              <div id="model-list"></div>
            </div>
            <div class="settings-section">
              <h3>Active chat model</h3>
              <select id="settings-active-model" class="select-control" aria-label="Active chat model"></select>
              <p class="settings-note">The composer uses this model key. If no remote model is configured, Gaggle falls back to <span class="mono">local-mock</span>.</p>
            </div>
            <div class="settings-section">
              <h3>ChatGPT OAuth sign-in</h3>
              <p class="settings-note">Create a provider of kind <span class="mono">chatgpt</span> first, then authorize it with OAuth + PKCE. Tokens are stored only in the server credential store and referenced by <span class="mono">secret_ref</span>.</p>
              <div class="form-row">
                <div>
                  <label class="field-label" for="input-chatgpt-client-id">OAuth client ID</label>
                  <input id="input-chatgpt-client-id" class="text-input" type="text" placeholder="OAuth client ID" autocomplete="off" spellcheck="false" />
                </div>
                <div>
                  <button id="btn-chatgpt-oauth" class="btn" type="button" style="width:100%;">Sign in with ChatGPT</button>
                </div>
              </div>
              <div id="chatgpt-oauth-status" class="settings-note" role="status"></div>
            </div>
          </section>

          <section class="settings-panel" id="panel-permissions" role="tabpanel" aria-labelledby="tab-permissions">
            <div class="settings-section">
              <h3>Project access</h3>
              <p class="settings-note">Default access for new chats in the active project. A <span class="mono">deny</span> command rule always wins, even in <span class="mono">full</span> access.</p>
              <div class="form-row">
                <div>
                  <label class="field-label" for="settings-project-access">Default access</label>
                  <select id="settings-project-access" class="select-control">
                    <option value="ask">ask — approve every mutating tool call</option>
                    <option value="full">full — execute without approval</option>
                  </select>
                </div>
                <div>
                  <button id="btn-save-project-access" class="btn" type="button" style="width:100%;">Save access</button>
                </div>
              </div>
              <div id="project-access-status" class="settings-note" role="status"></div>
            </div>
            <div class="settings-section">
              <h3>Tool permissions</h3>
              <p class="settings-note">Per-tool overrides for the active project. Leaving a tool on <span class="mono">default</span> delegates to the permission engine.</p>
              <div id="tool-permission-list"><div class="empty-state">Select a project to manage tool permissions.</div></div>
            </div>
            <div class="settings-section">
              <h3>Command rules</h3>
              <form id="form-add-command-rule" class="form-row" autocomplete="off">
                <div>
                  <label class="field-label" for="input-rule-pattern">Pattern</label>
                  <input id="input-rule-pattern" class="text-input" type="text" placeholder="rm -rf" required />
                </div>
                <div>
                  <label class="field-label" for="input-rule-match-kind">Match kind</label>
                  <select id="input-rule-match-kind" class="select-control">
                    <option value="exact">exact</option>
                    <option value="prefix">prefix</option>
                    <option value="regex">regex</option>
                  </select>
                </div>
                <div>
                  <label class="field-label" for="input-rule-action">Action</label>
                  <select id="input-rule-action" class="select-control">
                    <option value="deny">deny</option>
                    <option value="ask">ask</option>
                    <option value="allow">allow</option>
                  </select>
                </div>
                <div>
                  <button id="btn-add-command-rule" class="btn" type="submit" style="width:100%;">Add rule</button>
                </div>
              </form>
              <div id="rule-form-status" class="error-banner" role="alert"></div>
              <div id="command-rule-list"></div>
            </div>
          </section>

          <section class="settings-panel" id="panel-extensions" role="tabpanel" aria-labelledby="tab-extensions">
            <div class="settings-section">
              <div class="settings-toolbar">
                <h3>Extensions &amp; tool budget</h3>
                <button id="btn-refresh-extensions" class="btn btn-secondary" type="button">Refresh</button>
              </div>
              <div id="extension-warning" class="warning-banner" role="alert">
                Active tools exceed the recommended limit of 25. Disable extensions or tools to keep model tool selection accurate.
              </div>
              <p class="settings-note">
                Enabled tools: <strong id="extension-active-count">—</strong> / recommended <strong id="extension-max-count">25</strong>.
                Disabling a bundled extension removes its tools from future runs until re-enabled.
              </p>
              <div id="extension-list"><div class="empty-state">Loading extensions…</div></div>
            </div>
          </section>

          <section class="settings-panel" id="panel-memory" role="tabpanel" aria-labelledby="tab-memory">
            <div class="settings-section">
              <h3>Project &amp; global memory</h3>
              <p class="settings-note">
                Memories are scoped to the active project plus global entries. Pinned memories are prioritized during context assembly.
                Memory content is treated as untrusted data by the agent, and obvious credentials are redacted before storage.
              </p>
              <form id="form-add-memory" class="form-row" autocomplete="off">
                <div>
                  <label class="field-label" for="input-memory-kind">Kind</label>
                  <select id="input-memory-kind" class="select-control">
                    <option value="fact">fact</option>
                    <option value="preference">preference</option>
                    <option value="decision">decision</option>
                    <option value="todo">todo</option>
                    <option value="summary">summary</option>
                  </select>
                </div>
                <div>
                  <label class="field-label" for="input-memory-title">Title</label>
                  <input id="input-memory-title" class="text-input" type="text" placeholder="Short descriptive title" required />
                </div>
                <div>
                  <label class="field-label" for="input-memory-content">Content</label>
                  <textarea id="input-memory-content" class="text-input" rows="3" placeholder="What should the agent remember?" required></textarea>
                </div>
                <div>
                  <label class="field-label" for="input-memory-tags">Tags (comma separated)</label>
                  <input id="input-memory-tags" class="text-input" type="text" placeholder="architecture, security" />
                </div>
                <div>
                  <label class="toggle-label" for="input-memory-pinned">
                    <input id="input-memory-pinned" type="checkbox" /> Pin to context
                  </label>
                </div>
                <div>
                  <button id="btn-add-memory" class="btn" type="submit" style="width:100%;">Save memory</button>
                </div>
              </form>
              <div id="memory-form-status" class="error-banner" role="alert"></div>
            </div>
            <div class="settings-section">
              <div class="settings-toolbar">
                <h3>Saved memories</h3>
                <button id="btn-refresh-memories" class="btn btn-secondary" type="button">Refresh</button>
              </div>
              <div class="form-row">
                <div>
                  <label class="field-label" for="input-memory-search">Search</label>
                  <input id="input-memory-search" class="text-input" type="search" placeholder="FTS5 query, e.g. auth token" />
                </div>
                <div>
                  <button id="btn-search-memories" class="btn btn-secondary" type="button" style="width:100%;">Search</button>
                </div>
              </div>
              <div id="memory-list"><div class="empty-state">Loading memories…</div></div>
            </div>
          </section>

          <section class="settings-panel" id="panel-appearance" role="tabpanel" aria-labelledby="tab-appearance">
            <div class="settings-section">
              <h3>Theme</h3>
              <label class="field-label" for="settings-theme-select">Color theme</label>
              <select id="settings-theme-select" class="select-control">
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
              <p class="settings-note">Theme changes are persisted in <span class="mono">app_settings</span> and shared with the sidebar switcher. Gaggle respects reduced-motion preferences.</p>
            </div>
          </section>

          <section class="settings-panel" id="panel-diagnostics" role="tabpanel" aria-labelledby="tab-diagnostics">
            <div class="settings-section">
              <div class="settings-toolbar">
                <h3>Gaggle doctor</h3>
                <div class="inline-actions">
                  <button id="btn-refresh-diagnostics" class="btn btn-secondary" type="button">Refresh</button>
                  <button id="btn-copy-diagnostics" class="btn btn-secondary" type="button">Copy Redacted Diagnostics</button>
                  <button id="btn-export-diagnostics" class="btn" type="button">Export Redacted Diagnostics</button>
                </div>
              </div>
              <p class="settings-note">Live report from <span class="mono">GET /api/diagnostics</span>. Export calls <span class="mono">GET /api/diagnostics/export</span> only when you click it. It contains no API keys, prompts, source files, or raw tool output, but database paths can reveal local directory names.</p>
              <div id="diagnostics-status" class="settings-note" role="status"></div>
              <div id="diagnostics-output"><div class="empty-state">Loading diagnostics…</div></div>
            </div>
          </section>
        </div>
      </div>
    </div>
  </div>

  <div id="app-toast" class="toast" role="status" aria-live="polite"></div>

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

      const selectModelEl = document.getElementById('select-model');
      const modelStatusNoteEl = document.getElementById('model-status-note');
      const onboardingCardEl = document.getElementById('onboarding-card');
      const btnSkipOnboarding = document.getElementById('btn-skip-onboarding');
      const btnOnboardingAddProject = document.getElementById('btn-onboarding-add-project');
      const btnOnboardingProvider = document.getElementById('btn-onboarding-provider');
      const btnOpenSettings = document.getElementById('btn-open-settings');
      const settingsModalEl = document.getElementById('settings-modal');
      const btnCloseSettings = document.getElementById('btn-close-settings');
      const settingsTabButtons = Array.prototype.slice.call(document.querySelectorAll('[data-settings-tab]'));
      const tabProviders = document.getElementById('tab-providers');
      const providerForm = document.getElementById('form-add-provider');
      const inputProviderKind = document.getElementById('input-provider-kind');
      const inputProviderName = document.getElementById('input-provider-name');
      const inputProviderBaseUrl = document.getElementById('input-provider-base-url');
      const inputProviderAuthKind = document.getElementById('input-provider-auth-kind');
      const inputProviderApiKey = document.getElementById('input-provider-api-key');
      const providerFormStatus = document.getElementById('provider-form-status');
      const providerListEl = document.getElementById('provider-list');
      const btnRefreshProviders = document.getElementById('btn-refresh-providers');
      const modelForm = document.getElementById('form-add-model');
      const inputModelProvider = document.getElementById('input-model-provider');
      const inputModelKey = document.getElementById('input-model-key');
      const inputModelName = document.getElementById('input-model-name');
      const inputModelContext = document.getElementById('input-model-context');
      const inputModelSupportsTools = document.getElementById('input-model-supports-tools');
      const modelFormStatus = document.getElementById('model-form-status');
      const modelListEl = document.getElementById('model-list');
      const settingsActiveModel = document.getElementById('settings-active-model');
      const settingsProjectAccess = document.getElementById('settings-project-access');
      const btnSaveProjectAccess = document.getElementById('btn-save-project-access');
      const projectAccessStatus = document.getElementById('project-access-status');
      const toolPermissionListEl = document.getElementById('tool-permission-list');
      const commandRuleForm = document.getElementById('form-add-command-rule');
      const inputRulePattern = document.getElementById('input-rule-pattern');
      const inputRuleMatchKind = document.getElementById('input-rule-match-kind');
      const inputRuleAction = document.getElementById('input-rule-action');
      const ruleFormStatus = document.getElementById('rule-form-status');
      const commandRuleListEl = document.getElementById('command-rule-list');
      const btnRefreshExtensions = document.getElementById('btn-refresh-extensions');
      const extensionWarningEl = document.getElementById('extension-warning');
      const extensionActiveCountEl = document.getElementById('extension-active-count');
      const extensionMaxCountEl = document.getElementById('extension-max-count');
      const extensionListEl = document.getElementById('extension-list');
      const settingsThemeSelect = document.getElementById('settings-theme-select');
      const btnRefreshDiagnostics = document.getElementById('btn-refresh-diagnostics');
      const btnCopyDiagnostics = document.getElementById('btn-copy-diagnostics');
      const btnExportDiagnostics = document.getElementById('btn-export-diagnostics');
      const diagnosticsStatusEl = document.getElementById('diagnostics-status');
      const diagnosticsOutputEl = document.getElementById('diagnostics-output');
      const memoryForm = document.getElementById('form-add-memory');
      const inputMemoryKind = document.getElementById('input-memory-kind');
      const inputMemoryTitle = document.getElementById('input-memory-title');
      const inputMemoryContent = document.getElementById('input-memory-content');
      const inputMemoryTags = document.getElementById('input-memory-tags');
      const inputMemoryPinned = document.getElementById('input-memory-pinned');
      const memoryFormStatus = document.getElementById('memory-form-status');
      const inputMemorySearch = document.getElementById('input-memory-search');
      const btnRefreshMemories = document.getElementById('btn-refresh-memories');
      const btnSearchMemories = document.getElementById('btn-search-memories');
      const memoryListEl = document.getElementById('memory-list');
      const inputChatGptClientId = document.getElementById('input-chatgpt-client-id');
      const btnChatGptOAuth = document.getElementById('btn-chatgpt-oauth');
      const chatGptOAuthStatus = document.getElementById('chatgpt-oauth-status');
      const appToastEl = document.getElementById('app-toast');

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

      let providers = [];
      let modelsByProviderId = {};
      let selectedModelKey = 'local-mock';
      let registeredProjectCount = 0;
      let onboardingDismissed = false;
      let currentSettingsTab = 'providers';
      let diagnosticsReport = null;
      let lastFocusedBeforeSettings = null;

      const PROVIDER_KINDS = ['openai', 'anthropic', 'chatgpt', 'openrouter', 'deepseek', 'ollama', 'openai_compat'];

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

      function showToast(message, isError) {
        appToastEl.textContent = message;
        appToastEl.classList.toggle('error', Boolean(isError));
        appToastEl.classList.add('visible');
        window.clearTimeout(showToast._timer);
        showToast._timer = window.setTimeout(function() {
          appToastEl.classList.remove('visible');
        }, 4000);
      }

      function setRequestStatus(el, message, isError) {
        if (!el) return;
        el.textContent = message || '';
        if (el.classList.contains('error-banner')) {
          el.classList.toggle('visible', Boolean(message));
        }
        el.classList.toggle('status-error', Boolean(isError));
      }

      async function apiRequest(path, options) {
        const res = await fetch(API_BASE + path, options);
        if (!res.ok) {
          const text = await res.text();
          throw new Error(text || ('Request failed with HTTP ' + res.status));
        }
        const contentType = res.headers.get('content-type') || '';
        if (contentType.indexOf('application/json') === -1) return null;
        return res.json();
      }

      async function persistSetting(key, value) {
        return apiRequest('/api/settings', {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ key: key, value: value })
        });
      }

      async function copyToClipboard(text) {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(text);
          return;
        }
        const helper = createEl('textarea', {
          'aria-hidden': 'true',
          style: 'position:fixed;left:-9999px;top:0;'
        });
        helper.value = text;
        document.body.appendChild(helper);
        helper.select();
        document.execCommand('copy');
        helper.remove();
      }

      async function persistTheme(theme) {
        applyTheme(theme);
        try {
          await persistSetting('theme', theme);
        } catch (e) {
          showToast('Theme saved locally but not persisted: ' + e.message, true);
        }
      }

      function formatModelNote() {
        if (!selectedModelKey || selectedModelKey === 'local-mock') {
          modelStatusNoteEl.textContent = 'No remote model configured. Chat runs against the offline built-in model.';
          return;
        }
        modelStatusNoteEl.textContent = 'Remote inference: prompts and included workspace context are sent to the selected provider.';
      }

      function syncSelectValue(selectEl, value) {
        if (!selectEl) return;
        const exists = Array.prototype.some.call(selectEl.options, function(option) {
          return option.value === value;
        });
        selectEl.value = exists ? value : 'local-mock';
      }

      function buildModelOptions() {
        const options = [{ value: 'local-mock', label: 'local-mock (offline built-in)' }];
        providers.forEach(function(item) {
          const provider = item.provider || item;
          const models = modelsByProviderId[provider.id] || [];
          models.forEach(function(model) {
            options.push({
              value: model.model_key,
              label: provider.name + ' — ' + model.display_name + ' (' + model.model_key + ')'
            });
          });
        });
        return options;
      }

      function populateModelSelectors() {
        const options = buildModelOptions();
        const availableKeys = options.map(function(option) { return option.value; });
        if (availableKeys.indexOf(selectedModelKey) === -1) {
          selectedModelKey = 'local-mock';
        }
        [selectModelEl, settingsActiveModel].forEach(function(selectEl) {
          if (!selectEl) return;
          const previous = selectEl === selectModelEl ? selectedModelKey : (selectEl.value || selectedModelKey);
          selectEl.textContent = '';
          options.forEach(function(option) {
            selectEl.appendChild(createEl('option', { value: option.value, textContent: option.label }));
          });
          syncSelectValue(selectEl, previous || 'local-mock');
        });
        syncSelectValue(selectModelEl, selectedModelKey);
        syncSelectValue(settingsActiveModel, selectedModelKey);
        formatModelNote();
      }

      function updateOnboardingState() {
        if (!onboardingCardEl) return;
        const shouldShow = !onboardingDismissed && (registeredProjectCount === 0 || providers.length === 0);
        onboardingCardEl.classList.toggle('hidden', !shouldShow);
      }

      function openSettings(tab) {
        if (!settingsModalEl) return;
        lastFocusedBeforeSettings = document.activeElement;
        settingsModalEl.classList.add('open');
        btnOpenSettings.setAttribute('aria-expanded', 'true');
        switchSettingsTab(tab || currentSettingsTab);
        loadProviders();
        loadExtensions();
        loadDiagnostics();
        loadPermissionsForProject();
        loadMemories();
        window.setTimeout(function() {
          const target = document.getElementById('tab-' + (tab || currentSettingsTab));
          if (target) target.focus();
        }, 0);
      }

      function closeSettings() {
        if (!settingsModalEl) return;
        settingsModalEl.classList.remove('open');
        btnOpenSettings.setAttribute('aria-expanded', 'false');
        if (lastFocusedBeforeSettings && typeof lastFocusedBeforeSettings.focus === 'function') {
          lastFocusedBeforeSettings.focus();
        }
      }

      function switchSettingsTab(tab) {
        currentSettingsTab = tab || 'providers';
        settingsTabButtons.forEach(function(button) {
          const selected = button.getAttribute('data-settings-tab') === currentSettingsTab;
          button.setAttribute('aria-selected', selected ? 'true' : 'false');
          button.setAttribute('tabindex', selected ? '0' : '-1');
        });
        Array.prototype.forEach.call(document.querySelectorAll('.settings-panel'), function(panel) {
          panel.classList.toggle('active', panel.id === 'panel-' + currentSettingsTab);
        });
        if (currentSettingsTab === 'permissions') loadPermissionsForProject();
        if (currentSettingsTab === 'extensions') loadExtensions();
        if (currentSettingsTab === 'memory') loadMemories();
        if (currentSettingsTab === 'diagnostics') loadDiagnostics();
        if (currentSettingsTab === 'appearance' && settingsThemeSelect) {
          settingsThemeSelect.value = document.documentElement.getAttribute('data-theme') || 'system';
        }
      }

      function trapSettingsFocus(event) {
        if (event.key !== 'Tab' || !settingsModalEl.classList.contains('open')) return;
        const focusable = Array.prototype.filter.call(
          settingsModalEl.querySelectorAll('button, input, select, textarea, [href]'),
          function(node) { return !node.disabled && node.offsetParent !== null; }
        );
        if (!focusable.length) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
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

      if (themeSwitcher) {
        themeSwitcher.addEventListener('change', function() {
          persistTheme(themeSwitcher.value);
        });
      }

      if (settingsThemeSelect) {
        settingsThemeSelect.addEventListener('change', function() {
          themeSwitcher.value = settingsThemeSelect.value;
          persistTheme(settingsThemeSelect.value);
        });
      }

      async function loadAppPreferences() {
        try {
          const settings = await apiRequest('/api/settings');
          const values = {};
          (settings || []).forEach(function(setting) {
            try {
              values[setting.key] = JSON.parse(setting.value);
            } catch (e) {
              values[setting.key] = setting.value;
            }
          });
          if (typeof values.theme === 'string') {
            applyTheme(values.theme);
          }
          if (typeof values.active_model_key === 'string' && values.active_model_key) {
            selectedModelKey = values.active_model_key;
          }
          onboardingDismissed = values.onboarding_dismissed === true;
          formatModelNote();
          updateOnboardingState();
        } catch (e) {
          console.error('Failed to load application preferences:', e);
        }
      }

      async function loadProviders() {
        if (providerListEl) {
          providerListEl.textContent = '';
          providerListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Loading providers…' }));
        }
        try {
          providers = (await apiRequest('/api/providers')) || [];
          modelsByProviderId = {};
          await Promise.all(providers.map(async function(item) {
            const provider = item.provider || item;
            try {
              modelsByProviderId[provider.id] =
                (await apiRequest('/api/providers/' + encodeURIComponent(provider.id) + '/models')) || [];
            } catch (e) {
              modelsByProviderId[provider.id] = [];
            }
          }));
          renderProviderList();
          renderModelList();
          populateModelSelectors();
          updateOnboardingState();
        } catch (e) {
          if (providerListEl) {
            providerListEl.textContent = '';
            providerListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Unable to load providers: ' + e.message }));
          }
        }
      }

      function renderProviderList() {
        if (!providerListEl) return;
        providerListEl.textContent = '';
        inputModelProvider.textContent = '';

        if (!providers.length) {
          providerListEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: 'No providers configured yet. Add one above or keep using local-mock.'
          }));
          inputModelProvider.appendChild(createEl('option', { value: '', textContent: 'No providers configured' }));
          inputModelProvider.disabled = true;
          modelListEl.textContent = '';
          modelListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Add a provider before registering models.' }));
          return;
        }

        inputModelProvider.disabled = false;
        providers.forEach(function(item) {
          const provider = item.provider || item;
          const models = modelsByProviderId[provider.id] || [];
          inputModelProvider.appendChild(createEl('option', {
            value: provider.id,
            textContent: provider.name + ' (' + provider.kind + ')'
          }));

          const configured = item.has_configured_secret === true;
          const statusBadge = createEl('span', {
            className: 'badge ' + (configured ? 'status-ok' : 'status-warn'),
            textContent: configured ? 'Credential configured' : 'Credential not set'
          });
          const head = createEl('div', { className: 'provider-card-head' }, [
            createEl('strong', { textContent: provider.name }),
            createEl('div', { className: 'inline-actions' }, [
              createEl('span', { className: 'badge', textContent: provider.kind }),
              createEl('span', { className: 'badge', textContent: 'auth: ' + provider.auth_kind }),
              statusBadge
            ])
          ]);
          const details = [head];
          if (provider.base_url) {
            details.push(createEl('div', { className: 'settings-note mono', textContent: 'base_url: ' + provider.base_url }));
          }
          details.push(createEl('div', {
            className: 'settings-note mono',
            textContent: 'secret_ref: ' + (provider.secret_ref || '—') + ' (reference only; the key is never returned)'
          }));
          details.push(createEl('div', {
            className: 'settings-note',
            textContent: models.length + ' model(s): ' + (models.map(function(model) { return model.model_key; }).join(', ') || 'none')
          }));
          providerListEl.appendChild(createEl('div', { className: 'provider-card' }, details));
        });
      }

      function renderModelList() {
        if (!modelListEl) return;
        modelListEl.textContent = '';
        let total = 0;
        providers.forEach(function(item) {
          const provider = item.provider || item;
          const models = modelsByProviderId[provider.id] || [];
          models.forEach(function(model) {
            total += 1;
            modelListEl.appendChild(createEl('div', {
              className: 'settings-note mono',
              textContent: provider.name + ' · ' + model.display_name + ' · ' + model.model_key +
                ' · ctx ' + model.context_window + (model.supports_tools ? ' · tools' : '')
            }));
          });
        });
        if (!total) {
          modelListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'No models registered yet.' }));
        }
      }

      async function loadExtensions() {
        try {
          const inventory = await apiRequest('/api/extensions/inventory');
          extensionActiveCountEl.textContent = String(inventory.active_tool_count);
          extensionMaxCountEl.textContent = String(inventory.recommended_max_active_tools);
          extensionWarningEl.classList.toggle('visible', Boolean(inventory.exceeds_recommended_tool_limit));
          extensionListEl.textContent = '';

          if (!inventory.extensions.length) {
            extensionListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'No extensions registered.' }));
            return;
          }

          inventory.extensions.forEach(function(extension) {
            const checkbox = createEl('input', {
              type: 'checkbox',
              'aria-label': 'Enable extension ' + extension.name
            });
            checkbox.checked = extension.enabled === 1;
            checkbox.addEventListener('change', async function() {
              try {
                await apiRequest('/api/extensions/' + encodeURIComponent(extension.id), {
                  method: 'PATCH',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({ enabled: checkbox.checked })
                });
                showToast(extension.name + (checkbox.checked ? ' enabled' : ' disabled'));
                await loadExtensions();
              } catch (e) {
                checkbox.checked = !checkbox.checked;
                showToast('Failed to update extension: ' + e.message, true);
              }
            });

            const toolNames = (extension.tools || []).map(function(tool) {
              return tool.tool_name + (tool.is_read_only ? ' (ro)' : '');
            }).join(', ') || 'No tools registered';

            extensionListEl.appendChild(createEl('div', { className: 'extension-row' }, [
              createEl('div', { className: 'extension-row-main' }, [
                createEl('strong', { textContent: extension.name + ' · v' + extension.version }),
                createEl('span', {
                  className: 'settings-note',
                  textContent: extension.kind + ' · ' + extension.source + ' · ' + extension.tool_count + ' tool(s)'
                }),
                createEl('span', { className: 'settings-note mono', textContent: toolNames })
              ]),
              createEl('label', { className: 'toggle-label' }, [checkbox, ' Enabled'])
            ]));
          });
        } catch (e) {
          extensionListEl.textContent = '';
          extensionListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Unable to load extensions: ' + e.message }));
        }
      }

      function parseMemoryTags(raw) {
        return String(raw || '')
          .split(',')
          .map(function(tag) { return tag.trim(); })
          .filter(function(tag) { return tag.length > 0; });
      }

      async function loadMemories(search) {
        if (!memoryListEl) return;
        if (!currentProjectId) {
          memoryListEl.textContent = '';
          memoryListEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: 'Select a project to view and manage its memories.'
          }));
          return;
        }
        const query = typeof search === 'string' ? search.trim() : (inputMemorySearch ? inputMemorySearch.value.trim() : '');
        let path = '/api/memories?project_id=' + encodeURIComponent(currentProjectId) + '&limit=50';
        if (query) path += '&query=' + encodeURIComponent(query);
        memoryListEl.textContent = '';
        memoryListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Loading memories…' }));
        try {
          const memories = await apiRequest(path);
          renderMemoryList(memories || [], query);
        } catch (e) {
          memoryListEl.textContent = '';
          memoryListEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: 'Unable to load memories: ' + e.message
          }));
        }
      }

      function renderMemoryList(memories, query) {
        if (!memoryListEl) return;
        memoryListEl.textContent = '';
        if (!memories.length) {
          memoryListEl.appendChild(createEl('div', {
            className: 'empty-state',
            textContent: query ? 'No memories matched this search.' : 'No memories saved for this project yet.'
          }));
          return;
        }
        memories.forEach(function(entry) {
          const memory = entry.memory || entry;
          const tags = entry.tags || [];
          const pinButton = createEl('button', {
            className: 'btn btn-secondary',
            type: 'button',
            textContent: memory.pinned ? 'Unpin' : 'Pin'
          });
          pinButton.addEventListener('click', async function() {
            try {
              await apiRequest('/api/memories/' + encodeURIComponent(memory.id), {
                method: 'PATCH',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ pinned: !memory.pinned })
              });
              await loadMemories(query);
            } catch (e) {
              showToast('Failed to update memory: ' + e.message, true);
            }
          });
          const deleteButton = createEl('button', {
            className: 'btn btn-secondary',
            type: 'button',
            textContent: 'Delete'
          });
          deleteButton.addEventListener('click', async function() {
            if (!window.confirm('Delete this memory? It will be soft-deleted and excluded from context.')) return;
            try {
              await apiRequest('/api/memories/' + encodeURIComponent(memory.id), { method: 'DELETE' });
              showToast('Memory deleted');
              await loadMemories(query);
            } catch (e) {
              showToast('Failed to delete memory: ' + e.message, true);
            }
          });
          memoryListEl.appendChild(createEl('div', { className: 'provider-card' }, [
            createEl('div', { className: 'provider-card-head' }, [
              createEl('strong', { textContent: memory.title }),
              createEl('div', { className: 'inline-actions' }, [
                createEl('span', { className: 'badge', textContent: memory.kind }),
                memory.pinned ? createEl('span', { className: 'badge status-ok', textContent: 'pinned' }) : null,
                pinButton,
                deleteButton
              ].filter(Boolean))
            ]),
            createEl('div', { className: 'settings-note', textContent: memory.content }),
            tags.length ? createEl('div', {
              className: 'settings-note mono',
              textContent: 'tags: ' + tags.join(', ')
            }) : null
          ].filter(Boolean)));
        });
      }

      async function loadDiagnostics() {
        diagnosticsOutputEl.textContent = '';
        diagnosticsOutputEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Loading diagnostics…' }));
        diagnosticsStatusEl.textContent = '';
        try {
          const report = await apiRequest('/api/diagnostics');
          diagnosticsReport = report;
          const build = report.build || {};
          const health = report.core_health || { status: 'unknown', checks: [] };
          const capabilities = Array.isArray(report.capabilities) ? report.capabilities : [];
          const rows = [
            ['service', report.service],
            ['version', report.version],
            ['build_commit', build.commit || 'unknown'],
            ['build_profile', build.profile || 'unknown'],
            ['build_target', build.target || 'unknown'],
            ['rustc', build.rustc || 'unknown'],
            ['os', report.os],
            ['arch', report.arch],
            ['db_path', report.db_path],
            ['db_healthy', String(report.db_healthy)],
            ['core_health', health.status],
            ['journal_mode', report.journal_mode],
            ['foreign_keys_enabled', String(report.foreign_keys_enabled)],
            ['applied_migrations', String(report.applied_migrations)],
            ['active_projects_count', String(report.active_projects_count)],
            ['enabled_extensions_count', String(report.enabled_extensions_count)],
            ['pending_approvals_count', String(report.pending_approvals_count)]
          ];
          const diagnosticsTable = buildDataTable('Field', 'Value', rows.map(function(row) {
            return [row[0], row[1]];
          }));

          const healthRows = (health.checks || []).map(function(check) {
            return [check.name, check.status + ' — ' + check.detail];
          });
          const healthTable = buildDataTable('Core health check', 'Result', healthRows);

          const capabilityItems = capabilities.map(function(capability) {
            const item = createEl('div', { className: 'settings-section' }, [
              createEl('div', { className: 'inline-actions' }, [
                createEl('span', {
                  className: 'badge ' + (capability.available ? 'status-ok' : ''),
                  textContent: capability.state
                }),
                createEl('strong', { textContent: ' ' + capability.label + ' ' })
              ]),
              createEl('div', { className: 'settings-note', textContent: capability.reason }),
              capability.available || !capability.action ? null : createEl('div', {
                className: 'settings-note mono',
                textContent: 'action: ' + capability.action
              })
            ].filter(Boolean));
            return item;
          });
          const capabilitySection = createEl('div', { className: 'settings-section' }, [
            createEl('h3', { textContent: 'Optional capabilities' })
          ].concat(capabilityItems));

          diagnosticsOutputEl.textContent = '';
          diagnosticsOutputEl.appendChild(diagnosticsTable);
          diagnosticsOutputEl.appendChild(healthTable);
          diagnosticsOutputEl.appendChild(capabilitySection);
          diagnosticsStatusEl.textContent = 'Fetched live from the local server. Redaction removes secrets; paths are shown as-is. No telemetry is sent.';
        } catch (e) {
          diagnosticsOutputEl.textContent = '';
          diagnosticsOutputEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Unable to load diagnostics: ' + e.message }));
        }
      }

      function buildDataTable(headingA, headingB, rows) {
        const tbody = createEl('tbody');
        rows.forEach(function(row) {
          tbody.appendChild(createEl('tr', {}, [
            createEl('th', { textContent: row[0] }),
            createEl('td', { className: 'mono', textContent: String(row[1]) })
          ]));
        });
        return createEl('table', { className: 'data-table' }, [
          createEl('thead', {}, [createEl('tr', {}, [
            createEl('th', { textContent: headingA }),
            createEl('th', { textContent: headingB })
          ])]),
          tbody
        ]);
      }

      async function exportDiagnostics() {
        const bundle = await apiRequest('/api/diagnostics/export');
        const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const anchor = createEl('a', {
          href: url,
          download: 'gaggle-diagnostics-' + Date.now() + '.json'
        });
        document.body.appendChild(anchor);
        anchor.click();
        anchor.remove();
        window.setTimeout(function() { URL.revokeObjectURL(url); }, 0);
        diagnosticsStatusEl.textContent = 'Exported redacted diagnostics. Review the file before sharing it.';
        showToast('Redacted diagnostics exported.');
      }

      async function loadPermissionsForProject() {
        if (!settingsProjectAccess || !toolPermissionListEl || !commandRuleListEl) return;

        if (!currentProjectId) {
          settingsProjectAccess.disabled = true;
          btnSaveProjectAccess.disabled = true;
          projectAccessStatus.textContent = 'Register or select a project to configure permissions.';
          toolPermissionListEl.textContent = '';
          toolPermissionListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Select a project to manage tool permissions.' }));
          commandRuleListEl.textContent = '';
          commandRuleListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Select a project to manage command rules.' }));
          return;
        }

        const project = projectsById[currentProjectId];
        settingsProjectAccess.disabled = false;
        btnSaveProjectAccess.disabled = false;
        settingsProjectAccess.value = (project && project.default_access === 'full') ? 'full' : 'ask';
        projectAccessStatus.textContent = 'Editing permissions for ' + ((project && project.name) || 'the active project') + '.';

        toolPermissionListEl.textContent = '';
        toolPermissionListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Loading tool permissions…' }));
        commandRuleListEl.textContent = '';
        commandRuleListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Loading command rules…' }));

        try {
          const statuses = await apiRequest('/api/projects/' + encodeURIComponent(currentProjectId) + '/tool-permissions');
          toolPermissionListEl.textContent = '';
          if (!statuses.length) {
            toolPermissionListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'No enabled tools registered.' }));
          } else {
            const tbody = createEl('tbody');
            statuses.forEach(function(status) {
              const select = createEl('select', {
                className: 'select-control',
                'aria-label': 'Permission level for ' + status.tool_name
              });
              [['', 'default'], ['always_allow', 'always_allow'], ['ask_before', 'ask_before'], ['never_allow', 'never_allow']].forEach(function(pair) {
                select.appendChild(createEl('option', { value: pair[0], textContent: pair[1] }));
              });
              select.value = status.level || '';
              select.addEventListener('change', async function() {
                const previous = status.level || '';
                try {
                  await apiRequest('/api/projects/' + encodeURIComponent(currentProjectId) + '/tool-permissions', {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ extension_tool_id: status.extension_tool_id, level: select.value })
                  });
                  status.level = select.value || null;
                  showToast('Permission updated for ' + status.tool_name);
                } catch (e) {
                  select.value = previous;
                  showToast('Failed to update permission: ' + e.message, true);
                }
              });
              tbody.appendChild(createEl('tr', {}, [
                createEl('td', {}, [
                  createEl('div', { textContent: status.tool_name }),
                  createEl('div', { className: 'settings-note', textContent: status.extension_name })
                ]),
                createEl('td', {}, [
                  createEl('span', {
                    className: 'badge ' + (status.is_read_only ? 'status-ok' : 'status-warn'),
                    textContent: status.is_read_only ? 'read-only' : 'mutating'
                  })
                ]),
                createEl('td', {}, [select])
              ]));
            });
            toolPermissionListEl.appendChild(createEl('table', { className: 'data-table' }, [
              createEl('thead', {}, [createEl('tr', {}, [
                createEl('th', { textContent: 'Tool' }),
                createEl('th', { textContent: 'Risk' }),
                createEl('th', { textContent: 'Level' })
              ])]),
              tbody
            ]));
          }

          const rules = await apiRequest('/api/projects/' + encodeURIComponent(currentProjectId) + '/command-rules');
          commandRuleListEl.textContent = '';
          if (!rules.length) {
            commandRuleListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'No command rules. Add a deny rule for destructive commands.' }));
          } else {
            const tbody = createEl('tbody');
            rules.forEach(function(rule) {
              const badgeClass = rule.action === 'deny' ? 'status-danger' : (rule.action === 'allow' ? 'status-ok' : 'status-warn');
              tbody.appendChild(createEl('tr', {}, [
                createEl('td', { className: 'mono', textContent: rule.pattern }),
                createEl('td', { textContent: rule.match_kind }),
                createEl('td', {}, [createEl('span', { className: 'badge ' + badgeClass, textContent: rule.action })])
              ]));
            });
            commandRuleListEl.appendChild(createEl('table', { className: 'data-table' }, [
              createEl('thead', {}, [createEl('tr', {}, [
                createEl('th', { textContent: 'Pattern' }),
                createEl('th', { textContent: 'Match' }),
                createEl('th', { textContent: 'Action' })
              ])]),
              tbody
            ]));
          }
        } catch (e) {
          toolPermissionListEl.textContent = '';
          toolPermissionListEl.appendChild(createEl('div', { className: 'empty-state', textContent: 'Unable to load permissions: ' + e.message }));
          commandRuleListEl.textContent = '';
        }
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
        if (settingsModalEl && settingsModalEl.classList.contains('open')) {
          await loadPermissionsForProject();
        }
      }

      async function loadProjects(preferredProjectId) {
        try {
          const res = await fetch(API_BASE + '/api/projects');
          if (!res.ok) return;
          const projects = await res.json();
          projectsById = {};
          registeredProjectCount = projects.length;
          projectSwitcher.textContent = '';
          updateOnboardingState();

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

      if (btnOpenSettings) {
        btnOpenSettings.addEventListener('click', function() {
          openSettings('providers');
        });
      }
      if (btnCloseSettings) btnCloseSettings.addEventListener('click', closeSettings);
      if (settingsModalEl) {
        settingsModalEl.addEventListener('click', function(event) {
          if (event.target === settingsModalEl) closeSettings();
        });
      }
      settingsTabButtons.forEach(function(button) {
        button.addEventListener('click', function() {
          switchSettingsTab(button.getAttribute('data-settings-tab'));
        });
      });

      if (memoryForm) {
        memoryForm.addEventListener('submit', async function(event) {
          event.preventDefault();
          setRequestStatus(memoryFormStatus, '');
          if (!currentProjectId) {
            setRequestStatus(memoryFormStatus, 'Select a project before saving a memory.', true);
            return;
          }
          const payload = {
            project_id: currentProjectId,
            kind: inputMemoryKind.value,
            title: inputMemoryTitle.value.trim(),
            content: inputMemoryContent.value.trim(),
            tags: parseMemoryTags(inputMemoryTags.value),
            pinned: inputMemoryPinned.checked
          };
          if (!payload.title || !payload.content) {
            setRequestStatus(memoryFormStatus, 'Title and content are required.', true);
            return;
          }
          try {
            await apiRequest('/api/memories', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify(payload)
            });
            inputMemoryTitle.value = '';
            inputMemoryContent.value = '';
            inputMemoryTags.value = '';
            inputMemoryPinned.checked = false;
            showToast('Memory saved');
            await loadMemories('');
          } catch (e) {
            setRequestStatus(memoryFormStatus, 'Failed to save memory: ' + e.message, true);
          }
        });
      }
      if (btnRefreshMemories) {
        btnRefreshMemories.addEventListener('click', function() {
          if (inputMemorySearch) inputMemorySearch.value = '';
          loadMemories('');
        });
      }
      if (btnSearchMemories) {
        btnSearchMemories.addEventListener('click', function() {
          loadMemories();
        });
      }
      if (inputMemorySearch) {
        inputMemorySearch.addEventListener('keydown', function(event) {
          if (event.key === 'Enter') {
            event.preventDefault();
            loadMemories();
          }
        });
      }
      if (btnChatGptOAuth) {
        btnChatGptOAuth.addEventListener('click', async function() {
          if (chatGptOAuthStatus) chatGptOAuthStatus.textContent = '';
          const clientId = inputChatGptClientId ? inputChatGptClientId.value.trim() : '';
          if (!clientId) {
            if (chatGptOAuthStatus) chatGptOAuthStatus.textContent = 'Enter an OAuth client ID first.';
            return;
          }
          const match = providers.find(function(item) {
            return (item.provider || item).kind === 'chatgpt';
          });
          if (!match) {
            if (chatGptOAuthStatus) chatGptOAuthStatus.textContent = 'Add a provider of kind chatgpt first.';
            return;
          }
          const provider = match.provider || match;
          const popup = window.open('', 'gaggle-chatgpt-oauth', 'width=520,height=720');
          try {
            const started = await apiRequest(
              '/api/providers/' + encodeURIComponent(provider.id) + '/oauth/chatgpt/start',
              {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                  client_id: clientId,
                  redirect_uri: API_BASE + '/api/oauth/chatgpt/callback'
                })
              }
            );
            if (popup) {
              popup.location.href = started.authorization_url;
            }
            if (chatGptOAuthStatus) {
              chatGptOAuthStatus.textContent = popup
                ? 'Complete sign-in in the popup window.'
                : 'Popup blocked. Open the authorization URL manually: ' + started.authorization_url;
            }
            await loadProviders();
          } catch (e) {
            if (popup) popup.close();
            if (chatGptOAuthStatus) chatGptOAuthStatus.textContent = 'OAuth start failed: ' + e.message;
          }
        });
      }
      document.addEventListener('keydown', function(event) {
        if ((event.metaKey || event.ctrlKey) && event.key === ',') {
          event.preventDefault();
          openSettings(currentSettingsTab);
          return;
        }
        if (event.key === 'Escape' && settingsModalEl && settingsModalEl.classList.contains('open')) {
          event.preventDefault();
          closeSettings();
          return;
        }
        trapSettingsFocus(event);
      });

      if (btnSkipOnboarding) {
        btnSkipOnboarding.addEventListener('click', async function() {
          onboardingDismissed = true;
          updateOnboardingState();
          try {
            await persistSetting('onboarding_dismissed', true);
          } catch (e) {
            console.error('Failed to persist onboarding dismissal:', e);
          }
        });
      }
      if (btnOnboardingAddProject) {
        btnOnboardingAddProject.addEventListener('click', function() {
          btnOpenAddProject.click();
        });
      }
      if (btnOnboardingProvider) {
        btnOnboardingProvider.addEventListener('click', function() {
          openSettings('providers');
        });
      }

      providerForm.addEventListener('submit', async function(event) {
        event.preventDefault();
        setRequestStatus(providerFormStatus, '');
        // Clear the field before the network round-trip: the raw key is never
        // re-rendered, echoed back, or written to SQLite.
        const rawApiKey = inputProviderApiKey.value;
        inputProviderApiKey.value = '';

        const payload = {
          kind: inputProviderKind.value,
          name: inputProviderName.value.trim(),
          base_url: inputProviderBaseUrl.value.trim() || null,
          auth_kind: inputProviderAuthKind.value,
          api_key: rawApiKey ? rawApiKey : null
        };
        if (!payload.name) {
          setRequestStatus(providerFormStatus, 'Provider display name is required.', true);
          return;
        }
        if (PROVIDER_KINDS.indexOf(payload.kind) === -1) {
          setRequestStatus(providerFormStatus, 'Unsupported provider kind.', true);
          return;
        }

        try {
          await apiRequest('/api/providers', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
          });
          inputProviderName.value = '';
          inputProviderBaseUrl.value = '';
          showToast('Provider added. Credential stored by reference only.');
          await loadProviders();
        } catch (e) {
          setRequestStatus(providerFormStatus, 'Failed to add provider: ' + e.message, true);
        }
      });

      modelForm.addEventListener('submit', async function(event) {
        event.preventDefault();
        setRequestStatus(modelFormStatus, '');
        const providerId = inputModelProvider.value;
        if (!providerId) {
          setRequestStatus(modelFormStatus, 'Add a provider before registering a model.', true);
          return;
        }
        const payload = {
          model_key: inputModelKey.value.trim(),
          display_name: inputModelName.value.trim(),
          context_window: Number(inputModelContext.value || 0),
          supports_tools: inputModelSupportsTools.checked
        };
        if (!payload.model_key || !payload.display_name || payload.context_window <= 0) {
          setRequestStatus(modelFormStatus, 'Model key, display name, and a positive context window are required.', true);
          return;
        }
        try {
          const createdModel = await apiRequest('/api/providers/' + encodeURIComponent(providerId) + '/models', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
          });
          inputModelKey.value = '';
          inputModelName.value = '';
          showToast('Model registered and selected.');
          await loadProviders();
          handleModelSelection(createdModel.model_key);
          renderModelList();
        } catch (e) {
          setRequestStatus(modelFormStatus, 'Failed to add model: ' + e.message, true);
        }
      });

      function handleModelSelection(nextKey) {
        selectedModelKey = nextKey || 'local-mock';
        syncSelectValue(selectModelEl, selectedModelKey);
        syncSelectValue(settingsActiveModel, selectedModelKey);
        formatModelNote();
        persistSetting('active_model_key', selectedModelKey).catch(function(e) {
          showToast('Model selection not persisted: ' + e.message, true);
        });
      }

      selectModelEl.addEventListener('change', function() {
        handleModelSelection(selectModelEl.value);
      });
      settingsActiveModel.addEventListener('change', function() {
        handleModelSelection(settingsActiveModel.value);
      });

      btnRefreshProviders.addEventListener('click', function() {
        loadProviders();
      });

      btnSaveProjectAccess.addEventListener('click', async function() {
        if (!currentProjectId) return;
        try {
          const updated = await apiRequest('/api/projects/' + encodeURIComponent(currentProjectId), {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ default_access: settingsProjectAccess.value })
          });
          projectsById[updated.id] = updated;
          selectPermissionMode.value = updated.default_access;
          projectAccessStatus.textContent = 'Saved default access: ' + updated.default_access;
          showToast('Project access updated to ' + updated.default_access);
          updateWorkspaceHeader();
        } catch (e) {
          projectAccessStatus.textContent = 'Failed to save access: ' + e.message;
          showToast('Failed to save project access: ' + e.message, true);
        }
      });

      commandRuleForm.addEventListener('submit', async function(event) {
        event.preventDefault();
        setRequestStatus(ruleFormStatus, '');
        if (!currentProjectId) {
          setRequestStatus(ruleFormStatus, 'Select a project first.', true);
          return;
        }
        try {
          await apiRequest('/api/projects/' + encodeURIComponent(currentProjectId) + '/command-rules', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              pattern: inputRulePattern.value.trim(),
              match_kind: inputRuleMatchKind.value,
              action: inputRuleAction.value
            })
          });
          inputRulePattern.value = '';
          showToast('Command rule added.');
          await loadPermissionsForProject();
        } catch (e) {
          setRequestStatus(ruleFormStatus, 'Failed to add command rule: ' + e.message, true);
        }
      });

      btnRefreshExtensions.addEventListener('click', function() {
        loadExtensions();
      });
      btnRefreshDiagnostics.addEventListener('click', function() {
        loadDiagnostics();
      });
      btnCopyDiagnostics.addEventListener('click', async function() {
        try {
          if (!diagnosticsReport) await loadDiagnostics();
          if (!diagnosticsReport) throw new Error('No diagnostics report loaded');
          await copyToClipboard(JSON.stringify(diagnosticsReport, null, 2));
          diagnosticsStatusEl.textContent = 'Copied redacted diagnostics to the clipboard.';
          showToast('Redacted diagnostics copied.');
        } catch (e) {
          showToast('Copy failed: ' + e.message, true);
        }
      });
      btnExportDiagnostics.addEventListener('click', async function() {
        try {
          await exportDiagnostics();
        } catch (e) {
          diagnosticsStatusEl.textContent = 'Export failed: ' + e.message;
          showToast('Export failed: ' + e.message, true);
        }
      });

      (async function initializeUi() {
        await loadAppPreferences();
        await loadProviders();
        await loadProjects(null);
      })();

      function appendMessage(role, text, autoScroll) {
        if (autoScroll === undefined) autoScroll = true;
        const div = createEl('div', { className: 'message ' + role });
        if (role === 'assistant' && text === '') {
          div.classList.add('streaming-cursor');
        }
        // Safe XSS: renderMarkdownSafe escapes HTML before formatting
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

      function appendProblemNotice(errorText, retryGuidance, failureCategory) {
        const detailParts = [];
        if (failureCategory) detailParts.push('Category: ' + failureCategory);
        if (retryGuidance) detailParts.push('Next step: ' + retryGuidance);
        const exportButton = createEl('button', {
          className: 'btn btn-secondary',
          type: 'button',
          textContent: 'Copy redacted diagnostics',
          onclick: async function() {
            try {
              const bundle = await apiRequest('/api/diagnostics/export');
              await copyToClipboard(JSON.stringify(bundle, null, 2));
              showToast('Redacted diagnostics copied. Review before sharing.');
            } catch (e) {
              showToast('Could not copy diagnostics: ' + e.message, true);
            }
          }
        });
        const card = createEl('div', { className: 'action' }, [
          createEl('div', { textContent: '⚠ ' + (errorText || 'Run failed') }),
          detailParts.length ? createEl('div', {
            className: 'detail',
            textContent: detailParts.join(' · ')
          }) : null,
          createEl('div', { className: 'inline-actions' }, [exportButton])
        ].filter(Boolean));
        const div = createEl('div', { className: 'message assistant' }, [card]);
        messagesEl.appendChild(div);
        scrollToBottomIfNeeded();
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
        if (approval.cwd) {
          children.push(createEl('div', {
            className: 'detail',
            textContent: 'cwd: ' + String(approval.cwd)
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
            appendProblemNotice(data.error, data.retry_guidance, data.failure_category);
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
            body: JSON.stringify({ prompt: prompt, model_key: selectedModelKey || 'local-mock' })
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
          appendProblemNotice(e.message, 'Review the error, then retry the message. Your prompt was preserved in the composer.', 'request_failed');
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
    fn test_settings_first_run_onboarding_and_model_selection_present() {
        let html = render_index_html();

        // Progressive first-run onboarding card (skippable, inline)
        assert!(html.contains("id=\"onboarding-card\""));
        assert!(html.contains("id=\"btn-skip-onboarding\""));
        assert!(html.contains("id=\"btn-onboarding-add-project\""));
        assert!(html.contains("id=\"btn-onboarding-provider\""));
        assert!(html.contains("What stays local"));

        // Settings modal, entry point and keyboard shortcut
        assert!(html.contains("id=\"settings-modal\""));
        assert!(html.contains("id=\"btn-open-settings\""));
        assert!(html.contains("event.key === ','"));

        // Six settings tabs and their panels
        assert!(html.contains("data-settings-tab=\"providers\""));
        assert!(html.contains("data-settings-tab=\"permissions\""));
        assert!(html.contains("data-settings-tab=\"extensions\""));
        assert!(html.contains("data-settings-tab=\"memory\""));
        assert!(html.contains("data-settings-tab=\"appearance\""));
        assert!(html.contains("data-settings-tab=\"diagnostics\""));
        assert!(html.contains("id=\"panel-providers\""));
        assert!(html.contains("id=\"panel-permissions\""));
        assert!(html.contains("id=\"panel-extensions\""));
        assert!(html.contains("id=\"panel-memory\""));
        assert!(html.contains("id=\"panel-appearance\""));
        assert!(html.contains("id=\"panel-diagnostics\""));

        // Providers/models tab and credential status without secret exposure
        assert!(html.contains("id=\"form-add-provider\""));
        assert!(html.contains("type=\"password\""));
        assert!(html.contains("autocomplete=\"off\""));
        assert!(html.contains("has_configured_secret"));
        assert!(html.contains("id=\"provider-list\""));
        assert!(html.contains("id=\"form-add-model\""));

        // Permissions, extensions, diagnostics controls
        assert!(html.contains("id=\"settings-project-access\""));
        assert!(html.contains("id=\"tool-permission-list\""));
        assert!(html.contains("id=\"form-add-command-rule\""));
        assert!(html.contains("id=\"extension-warning\""));
        assert!(html.contains("exceeds_recommended_tool_limit"));
        assert!(html.contains("id=\"btn-copy-diagnostics\""));
        assert!(html.contains("id=\"btn-export-diagnostics\""));
        assert!(html.contains("/api/diagnostics/export"));
        assert!(html.contains("function exportDiagnostics("));
        assert!(html.contains("Optional capabilities"));
        assert!(html.contains("function appendProblemNotice("));
        assert!(html.contains("id=\"form-add-memory\""));
        assert!(html.contains("id=\"memory-list\""));
        assert!(html.contains("id=\"btn-chatgpt-oauth\""));
        assert!(html.contains("<option value=\"chatgpt\">chatgpt</option>"));

        // Composer model selector replaces the hardcoded model key
        assert!(html.contains("id=\"select-model\""));
        assert!(html.contains("selectedModelKey || 'local-mock'"));
        assert!(!html.contains("model_key: 'local-mock'"));
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
