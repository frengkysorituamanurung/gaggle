# Panduan Kontribusi (Contributing to Gaggle)

Terima kasih atas minat Anda untuk berkontribusi pada **Gaggle**! Dokumen ini berisi panduan alur kerja, konvensi arsitektur, standar kode Rust, serta aturan skema database SQLite agar kontribusi Anda dapat ditinjau dan digabungkan (*merged*) dengan lancar.

Dengan berpartisipasi dalam proyek ini, Anda setuju untuk mematuhi [Kode Etik Komunitas (CODE_OF_CONDUCT.md)](./CODE_OF_CONDUCT.md) kami.

---

## Daftar Isi

1. [Cara Berkontribusi](#1-cara-berkontribusi)
2. [Persiapan Lingkungan Pengembangan](#2-persiapan-lingkungan-pengembangan)
3. [Struktur Workspace & Tanggung Jawab Crate](#3-struktur-workspace--tanggung-jawab-crate)
4. [Aturan Skema Database & Migrasi SQLite](#4-aturan-skema-database--migrasi-sqlite)
5. [Standar Penulisan Kode Rust](#5-standar-penulisan-kode-rust)
6. [Pengujian (Testing)](#6-pengujian-testing)
7. [Alur Pull Request & Konvensi Commit](#7-alur-pull-request--konvensi-commit)
8. [Lisensi Kontribusi](#8-lisensi-kontribusi)

---

## 1. Cara Berkontribusi

Anda dapat berkontribusi dalam berbagai bentuk:
- **Melaporkan Bug**: Buka *Issue* dengan langkah reproduksi yang jelas, versi OS, versi Rust (`rustc --version`), dan log terkait.
- **Mengusulkan Fitur / Perubahan Arsitektur**: Diskusikan terlebih dahulu melalui *Issue* sebelum menulis perubahan berskala besar, terutama jika menyangkut perubahan skema database (`gaggle-db`) atau *permission engine* (`gaggle-core`).
- **Mengirimkan Pull Request**: Perbaikan *bug*, penambahan pengujian, peningkatan performa, maupun perbaikan dokumentasi.
- **Melaporkan Celah Keamanan**: Ikuti prosedur privat pada [SECURITY.md](./SECURITY.md).

---

## 2. Persiapan Lingkungan Pengembangan

1. Pastikan Anda memiliki toolchain **Rust stabil terbaru** (mendukung `edition = "2024"`), `rustfmt`, dan `clippy`:
   ```bash
   rustup update stable
   rustup component add rustfmt clippy
   ```

2. Kloning repositori dan verifikasi bahwa seluruh crate dapat dikompilasi dan lulus pengujian:
   ```bash
   git clone https://github.com/frengky/gaggle.git
   cd gaggle
   cargo build --workspace
   cargo test --workspace
   ```

---

## 3. Struktur Workspace & Tanggung Jawab Crate

Tempatkan kode Anda pada crate yang sesuai dengan batas domainnya:

| Crate | Tanggung Jawab Domain |
| :--- | :--- |
| `crates/gaggle-db` | Koneksi SQLite, pengaturan PRAGMA, file migrasi SQL (`migrations/`), struct model tabel ERD, serta *repository* query per domain. Tidak boleh bergantung pada crate Gaggle lain. |
| `crates/gaggle-core` | Logika inti agent: *7-step Permission Engine*, pemulihan *durable runs* (`RunEngine`), perhitungan *context compaction*, dan konfigurasi sandbox *Code Mode*. |
| `crates/gaggle-tools` | Implementasi *built-in extensions* (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) dan pembuatan *unified diff* (`similar`). |
| `crates/gaggle-mcp` | Integrasi klien MCP, manajemen *extension registry*, serta validasi batas jumlah tool aktif per percakapan (~25 tools). |
| `crates/gaggle-recipes` | Parser definisi recipe (YAML/JSON), validasi parameter wajib vs opsional, rendering template `minijinja`, dan penjadwalan cron (`croner`). |
| `crates/gaggle-server` | Server HTTP/SSE berbasis `axum`, endpoint REST API, ekspor/impor sesi percakapan JSON, serta penyajian UI web. |
| `crates/gaggle-cli` | Titik masuk binari CLI `gaggle` (`serve`, `project`, `db-path`, dll.). |
| `crates/gaggle-ui` | Aset dan komponen antarmuka web Gaggle. |

---

## 4. Aturan Skema Database & Migrasi SQLite

Setiap perubahan pada skema database di `crates/gaggle-db/migrations/` **wajib** mematuhi konvensi ERD ([`docs/ERD.md`](./docs/ERD.md)):

1. **Primary Key**: Gunakan UUIDv7 yang disimpan sebagai `TEXT` (`gaggle_db::connection::new_id()`).
2. **Timestamp**: Gunakan `INTEGER` (Unix epoch dalam milidetik, `gaggle_db::connection::now_ms()`).
3. **Kolom JSON**: Disimpan sebagai `TEXT` dan wajib dilengkapi constraint `CHECK(json_valid(nama_kolom))` (atau `CHECK(nama_kolom IS NULL OR json_valid(nama_kolom))` untuk kolom nullable).
4. **Kolom Boolean**: Gunakan `INTEGER` (`0` atau `1`) dengan `CHECK(nama_kolom IN (0, 1))`.
5. **Kolom Enum**: Gunakan `TEXT` dengan `CHECK(nama_kolom IN (...))` sesuai daftar nilai yang diizinkan di ERD.
6. **Rahasia & Kredensial**: Jangan pernah membuat kolom untuk menyimpan API key atau token OAuth secara langsung; gunakan kolom `secret_ref` yang merujuk ke OS Keychain.
7. **Migrasi Baru**: Jangan mengubah file migrasi yang sudah dirilis kecuali dalam tahap pre-release awal; tambahkan file migrasi bernomor urut berikutnya (mis. `0002_nama_perubahan.sql`).

---

## 5. Standar Penulisan Kode Rust

- **Formatting**: Seluruh kode wajib diformat menggunakan `cargo fmt --all`.
- **Linting**: Pastikan tidak ada peringatan dari `cargo clippy --workspace --all-targets`.
- **Penanganan Error**:
  - Gunakan `thiserror` untuk tipe error domain terstruktur dan `anyhow::Result` beserta `.context(...)` untuk batas aplikasi/repositori.
  - Hindari penggunaan `.unwrap()` atau `.expect(...)` pada jalur kode produksi di `gaggle-server` dan `gaggle-core` (kecuali pada pengujian atau invarian statis yang terbukti aman).
- **Logging**: Gunakan makro dari crate `tracing` (`info!`, `warn!`, `error!`, `debug!`) dengan *structured fields* (mis. `info!(run_id = %run.id, "...")`), bukan `println!` di dalam crate pustaka.

---

## 6. Pengujian (Testing)

Setiap fitur baru atau perbaikan bug harus disertai dengan pengujian otomatis:

- Untuk pengujian database atau repository di `gaggle-db` dan `gaggle-core`, gunakan helper `gaggle_db::connect_in_memory().await?` agar pengujian berjalan cepat, terisolasi, dan otomatis menjalankan seluruh migrasi SQLite serta FTS5.
- Jalankan seluruh suite pengujian sebelum membuka Pull Request:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets
  cargo test --workspace
  ```

---

## 7. Alur Pull Request & Konvensi Commit

1. Buat cabang (*branch*) baru dari `main` dengan nama yang deskriptif (mis. `feat/worktree-manager` atau `fix/permission-deny-rule`).
2. Gunakan pesan *commit* mengikuti gaya **Conventional Commits**:
   - `feat(core): add smart permission LLM risk cache lookup`
   - `fix(db): enforce json_valid check on project_settings.env`
   - `docs(readme): update CLI usage examples`
   - `test(db): add FTS5 trigger update test`
3. Pastikan seluruh *test* lulus dan deskripsikan perubahan serta konteks pengujian pada deskripsi *Pull Request*.

---

## 8. Lisensi Kontribusi

Sesuai dengan Bagian 5 dari [Apache License, Version 2.0](./LICENSE), setiap kontribusi yang Anda kirimkan secara sengaja untuk dimasukkan ke dalam proyek **Gaggle** akan berada di bawah ketentuan **Apache License, Version 2.0**, tanpa syarat atau ketentuan tambahan.
