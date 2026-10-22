---
name: Gaggle DB Schema
description: Gunakan skill ini setiap kali membuat atau mengubah tabel SQLite, migrasi SQL, indeks, FTS5, struct model, atau repository di crates/gaggle-db sesuai ERD Gaggle.
---

# Skill: Gaggle DB Schema & SQLite Conventions

Gunakan panduan ini saat bekerja di dalam `crates/gaggle-db` atau ketika menambahkan tabel, kolom, indeks, maupun query baru.

## 1. Konvensi Kolom & Tipe Data Wajib
- **Primary Key**: Selalu bertipe `TEXT` berisi **UUIDv7** yang dibuat dari aplikasi (`gaggle_db::connection::new_id()`).
- **Timestamp / Waktu**: Selalu bertipe `INTEGER` berisi Unix epoch dalam milidetik (`gaggle_db::connection::now_ms()`).
- **JSON**: Disimpan sebagai `TEXT` dan **wajib** memiliki constraint `CHECK(json_valid(kolom))` (atau `CHECK(kolom IS NULL OR json_valid(kolom))` bila nullable).
- **Boolean**: Disimpan sebagai `INTEGER` (`0` atau `1`) dengan `CHECK(kolom IN (0, 1))`.
- **Enum**: Disimpan sebagai `TEXT` dengan `CHECK(kolom IN (...))` sesuai daftar nilai di `docs/ERD.md`.
- **Rahasia / API Key / OAuth Token**: Dilarang disimpan di database. Simpan hanya kunci referensi di kolom `secret_ref` yang menunjuk ke OS Keychain.
- **Soft Delete**: Gunakan kolom `archived_at` atau `deleted_at` (`NULL` berarti masih aktif).

## 2. Referensi Melingkar & Indeks Khusus
- `messages.run_id` <-> `runs.trigger_message_id` dan `chats.worktree_id` <-> `worktrees.chat_id` menggunakan `DEFERRABLE INITIALLY DEFERRED`.
- `tool_permissions` menggunakan indeks ekspresi `UNIQUE(COALESCE(project_id, ''), extension_tool_id)` karena `NULL` dianggap berbeda oleh `UNIQUE` standar SQLite.
- Tabel FTS5 (`memories_fts` dan `messages_fts`) disinkronkan lewat trigger `AFTER INSERT`, `AFTER DELETE`, dan `AFTER UPDATE`.

## 3. Pengujian
- Gunakan `gaggle_db::connect_in_memory().await?` pada integration test di `crates/gaggle-db/tests/`.
- Jalankan `cargo test -p gaggle-db` untuk memvalidasi migrasi dan query.
