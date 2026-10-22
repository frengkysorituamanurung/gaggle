# Gaggle — Architecture & ERD Database

Skema database dan arsitektur workspace untuk **Gaggle**, asisten AI pribadi yang self-hosted dan bekerja di workspace/proyek nyata. Seluruh aplikasi ditulis dengan **Rust**, dengan satu database **SQLite** lokal per perangkat (`gaggle.db`). Desainnya terinspirasi dari **Goose** (agent open-source dari Block, ditulis dengan Rust).

## Susunan Workspace Cargo

```text
gaggle/
├─ Cargo.toml                # [workspace]
├─ docs/
│  └─ ERD.md                 # Dokumentasi ERD & arsitektur Gaggle
└─ crates/
   ├─ gaggle-core/           # agent loop, run engine, compaction, permission engine, Code Mode
   ├─ gaggle-db/             # sqlx, migrasi SQLite + FTS5, models & repository per domain
   ├─ gaggle-tools/          # extension bawaan: fs, shell, memory, web, browser
   ├─ gaggle-mcp/            # klien MCP, registry extension & batas tool aktif
   ├─ gaggle-recipes/        # parser recipe, validasi parameter, template minijinja, scheduler
   ├─ gaggle-server/         # axum: REST + SSE, menyajikan UI & ekspor chat
   ├─ gaggle-cli/            # binary `gaggle` (gaggle serve, gaggle project, gaggle db-path)
   └─ gaggle-ui/             # web UI shell yang disajikan oleh gaggle-server
```

## Konvensi Database

| Aspek | Aturan |
|---|---|
| Lokasi DB Global | `~/.local/share/gaggle/gaggle.db` (Linux) atau `~/Library/Application Support/gaggle/gaggle.db` (macOS) via crate `directories` |
| Primary key | UUIDv7 disimpan sebagai `TEXT` (terurut waktu, dibuat di sisi aplikasi) |
| Waktu | `INTEGER`, unix epoch dalam milidetik |
| JSON | Disimpan sebagai `TEXT` dengan `CHECK(json_valid(kolom))` |
| Boolean | `INTEGER` 0/1 dengan `CHECK(kolom IN (0, 1))` |
| Enum | `TEXT` dengan `CHECK(kolom IN (...))` |
| PRAGMA Koneksi | `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`, `auto_vacuum=INCREMENTAL` |
| Migrasi | Otomatis dijalankan saat start lewat `sqlx::migrate!("./migrations")` di `crates/gaggle-db/migrations/0001_initial_schema.sql` |
