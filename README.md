# Gaggle

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![Database: SQLite](https://img.shields.io/badge/Database-SQLite_WAL_%2B_FTS5-003B57.svg)](https://www.sqlite.org/)

**Gaggle** adalah asisten dan agent AI pribadi yang *self-hosted*, dirancang untuk bekerja langsung di dalam workspace dan proyek nyata Anda. Seluruh aplikasi ditulis dalam **Rust** dengan satu database **SQLite** global per perangkat.

Desain arsitektur Gaggle terinspirasi dari **Goose** (agent AI open-source dari Block), namun diperluas dengan konsep **Project sebagai entitas utama**, isolasi **Git Worktree per percakapan**, **panel Changes & Review Comments**, **Durable Runs** yang tahan terhadap penutupan tab maupun restart server, **Code Mode sandbox**, **Terminal PTY**, serta **Browser Sandbox**.

---

## Fitur Utama

- **Project-Centric Workspace**: Setiap percakapan (`chats`), memori (`memories`), aturan perintah (`command_rules`), worktree, jadwal (`schedules`), dan sesi terminal (`terminal_sessions`) dikelompokkan rapi di bawah entitas `projects`.
- **Durable Runs & Event Replay**: Eksekusi agent berjalan di latar belakang (`runs`). Seluruh kejadian disimpan secara berurutan di `run_events` (satu-satunya sumber kebenaran / *single source of truth*) untuk *replay* SSE, serta `run_checkpoints` untuk memulihkan pekerjaan secara otomatis saat server di-restart.
- **Mode Git Worktree & Panel Changes**: Isolasikan perubahan kode tiap percakapan di Git Worktree terpisah (`worktrees`), pantau *unified diff* per file di `file_changes`, dan berikan komentar review baris-demi-baris (`review_comments`) yang dapat dikirim kembali ke agent.
- **Sistem Izin Berlapis (7-Step Permission Engine)**:
  - Empat mode izin per chat: `ask` (*default* aman), `smart` (*LLM risk judge cache*), `full` (*autonomous*), dan `chat_only`.
  - Aturan perintah shell (`command_rules` dengan aksi `allow`, `ask`, `deny` — di mana `deny` selalu berlaku mutlak termasuk pada mode `full`).
  - Izin per tool (`always_allow`, `ask_before`, `never_allow`) baik di tingkat global maupun per proyek.
- **Unified Extension & MCP Architecture**: Seluruh kemampuan bawaan (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) maupun server MCP eksternal (`stdio`, `streamable_http`, `sse`, `wasm`) dimodelkan seragam sebagai `extensions`.
- **Recipe, Sub-Recipe, & Cron Scheduler**: Bungkus instruksi, prompt berparameter (`minijinja`), dan daftar extension menjadi `recipes` yang dapat dijalankan ulang, didelegasikan ke *subagent* (`parent_chat_id`), atau dijadwalkan otomatis dengan ekspresi cron (`schedules`).
- **Pencarian Cepat dengan SQLite FTS5**: Pencarian teks penuh (*full-text search*) bawaan pada riwayat pesan (`messages_fts`) dan memori proyek/global (`memories_fts`), dilengkapi dukungan *vector embedding* opsional (`memory_embeddings`).
- **Keamanan Kredensial**: API key dan token OAuth **tidak pernah** disimpan di dalam database. Database hanya menyimpan `secret_ref` yang merujuk ke OS Keychain.

---

## Arsitektur Workspace

Gaggle menggunakan susunan Cargo Workspace multi-crate agar batas tanggung jawab setiap domain tetap bersih dan modular:

```text
gaggle/
├─ Cargo.toml                # Konfigurasi Cargo Workspace
├─ LICENSE                   # Lisensi Apache-2.0
├─ README.md                 # Dokumentasi utama proyek
├─ CONTRIBUTING.md           # Panduan kontribusi
├─ CODE_OF_CONDUCT.md        # Kode etik komunitas
├─ SECURITY.md               # Kebijakan keamanan & pelaporan celah
├─ docs/
│  └─ ERD.md                 # Dokumentasi detail skema & ERD database
└─ crates/
   ├─ gaggle-core/           # Agent loop, durable run recovery, compaction, 7-step permission engine, Code Mode
   ├─ gaggle-db/             # Koneksi SQLite (WAL, FK), migrasi sqlx, 37 model tabel ERD, repository per domain
   ├─ gaggle-tools/          # Extension bawaan (fs, shell, memory, web, browser) & unified diff generator
   ├─ gaggle-mcp/            # Klien MCP, registry extension, dan penjaga batas tool aktif (~25 tools)
   ├─ gaggle-recipes/        # Parser recipe, validasi parameter wajib/opsional, render template minijinja, scheduler
   ├─ gaggle-server/         # Server HTTP/SSE berbasis Axum, REST API, ekspor/impor chat JSON, penyajian UI
   ├─ gaggle-cli/            # Binary CLI `gaggle` (`serve`, `project add/list`, `db-path`)
   └─ gaggle-ui/             # Antarmuka web yang disajikan langsung oleh gaggle-server
```

---

## Penyimpanan Data & Konvensi Database

Gaggle menggunakan **satu file SQLite global** untuk seluruh proyek agar direktori kerja proyek Anda tetap bersih (tanpa folder `.gaggle` yang mengotori repositori atau menggandakan database saat memakai Git Worktree).

Lokasi default database ditentukan secara otomatis menggunakan standar direktori sistem operasi:
- **macOS**: `~/Library/Application Support/gaggle/gaggle.db`
- **Linux**: `~/.local/share/gaggle/gaggle.db`
- **Windows**: `%APPDATA%\gaggle\data\gaggle.db`

Konvensi skema SQLite yang diterapkan di `gaggle-db`:
- **Primary Key**: UUIDv7 (`TEXT`), terurut berdasarkan waktu pembuatan.
- **Timestamp**: `INTEGER`, Unix epoch dalam satuan milidetik.
- **Validasi JSON & Enum**: Dijaga ketat di level database menggunakan constraint `CHECK(json_valid(...))` dan `CHECK(kolom IN (...))`.
- **PRAGMA Koneksi**: `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`, dan `auto_vacuum=INCREMENTAL`.

Lihat dokumentasi lengkap skema dan diagram relasi di [`docs/ERD.md`](./docs/ERD.md).

---

## Memulai (Getting Started)

### Prasyarat

- **Rust Toolchain**: Versi stabil terbaru (mendukung Rust 2024 Edition, mis. `rustc >= 1.85`).
- **Git**: Diperlukan untuk fitur manajemen repositori dan mode Git Worktree.

### Instalasi & Build dari Source

1. Kloning repositori ini:
   ```bash
   git clone https://github.com/frengky/gaggle.git
   cd gaggle
   ```

2. Jalankan kompilasi seluruh workspace:
   ```bash
   cargo build --workspace
   ```

3. Jalankan seluruh pengujian (termasuk pengujian migrasi skema SQLite, FTS5, dan *Permission Engine*):
   ```bash
   cargo test --workspace
   ```

---

## Penggunaan CLI (`gaggle`)

Binary utama tersedia melalui crate `gaggle-cli` dengan nama perintah `gaggle`.

### 1. Melihat Lokasi Database Global
```bash
cargo run -p gaggle-cli -- db-path
```

### 2. Mendaftarkan Proyek Baru
```bash
cargo run -p gaggle-cli -- project add "Gaggle Workspace" "/path/to/your/project" --vcs git
```

### 3. Melihat Daftar Proyek Aktif
```bash
cargo run -p gaggle-cli -- project list
```

### 4. Menjalankan Server & Web UI
```bash
cargo run -p gaggle-cli -- serve --addr 127.0.0.1:4321
```
Saat server dijalankan, Gaggle secara otomatis:
1. Membuka koneksi SQLite global dengan mode WAL.
2. Menjalankan migrasi database (`sqlx::migrate!`).
3. Mendaftarkan seluruh *built-in extensions* (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`).
4. Memulihkan *durable runs* yang sebelumnya berstatus `running` atau `waiting_approval`.

### Endpoint API Dasar
- `GET /` — Antarmuka Web Gaggle.
- `GET /api/health` — Pemeriksaan status layanan (`{"status": "ok", "service": "gaggle"}`).
- `GET /api/projects` — Daftar proyek aktif.
- `POST /api/projects` — Mendaftarkan proyek baru.
- `POST /api/chats` — Membuat sesi percakapan baru pada proyek.
- `GET /api/chats/{id}/export` — Mengekspor seluruh riwayat chat beserta bagian pesannya (`message_parts`) ke format JSON.

---

## Kontribusi & Komunitas

Kami sangat terbuka terhadap kontribusi dari komunitas! Sebelum mengirimkan *Pull Request* atau membuka *Issue*, silakan baca dokumen berikut:

- [Panduan Kontribusi (CONTRIBUTING.md)](./CONTRIBUTING.md)
- [Kode Etik Komunitas (CODE_OF_CONDUCT.md)](./CODE_OF_CONDUCT.md)
- [Kebijakan Keamanan (SECURITY.md)](./SECURITY.md)

---

## Lisensi

Proyek ini dilisensikan di bawah **Apache License, Version 2.0**. Lihat file [`LICENSE`](./LICENSE) untuk teks lisensi selengkapnya.
