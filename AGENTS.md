# Gaggle Project Instructions

Panduan ini otomatis dimuat oleh OpenCode pada setiap sesi di repositori **Gaggle**.

## Arsitektur Workspace (`gaggle`)
- `crates/gaggle-db`: Koneksi SQLite global (`gaggle.db`), PRAGMA (`WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`, `auto_vacuum=INCREMENTAL`), migrasi `sqlx`, 37 model tabel ERD, FTS5 (`memories_fts`, `messages_fts`), dan repository per domain.
- `crates/gaggle-core`: Logika inti agent, pemulihan durable run (`RunEngine`), 7-step `PermissionEngine`, context compaction, dan konfigurasi sandbox Code Mode.
- `crates/gaggle-tools`: Extension bawaan (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) dan diff generator (`similar`).
- `crates/gaggle-mcp`: Klien MCP & extension registry (menjaga batas rekomendasi <= 25 active tools per chat).
- `crates/gaggle-recipes`: Parser recipe, validasi parameter wajib/opsional, render template `minijinja`, dan scheduler (`croner`).
- `crates/gaggle-server`: Server HTTP/SSE berbasis `axum`, REST API, ekspor/impor chat JSON, dan penyajian UI.
- `crates/gaggle-cli`: Binary CLI `gaggle`.
- `crates/gaggle-ui`: Shell UI web Gaggle.

## Aturan Wajib Sebelum Mengubah Kode
1. **Perubahan Database / SQL / Repository**: Wajib memuat skill `@gaggle-db-schema` atau mengikuti aturan konvensi di `docs/ERD.md` (UUIDv7 `TEXT`, waktu epoch ms `INTEGER`, `CHECK(json_valid(...))`, `CHECK(col IN (...))`, dan `secret_ref` untuk rahasia di OS Keychain).
2. **Perubahan Permission / Tool Execution / Durable Run**: Wajib memuat skill `@gaggle-architecture` agar urutan 7 langkah keputusan izin (`chat_only` -> `command_rules` deny -> `full` -> `tool_permissions` -> `is_read_only` -> `smart` `tool_risk_cache` -> `approvals`) tetap terjaga.
3. **Verifikasi**: Selalu jalankan `cargo test --workspace` setelah mengubah kode Rust atau SQL.
