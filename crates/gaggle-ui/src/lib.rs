/// Returns the embedded index HTML shell for Gaggle's web interface.
pub fn render_index_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Gaggle — Self-Hosted AI Workspace Agent</title>
</head>
<body>
  <main id="app">
    <h1>Gaggle</h1>
    <p>Self-hosted AI agent workspace powered by Rust &amp; SQLite.</p>
  </main>
</body>
</html>"#
}
