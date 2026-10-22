---
name: Gaggle Architecture
description: Gunakan skill ini saat mengerjakan agent loop, durable runs, 7-step permission engine, built-in extensions, MCP, recipes, atau endpoint server di Gaggle.
---

# Skill: Gaggle Core Architecture & Workflows

Gunakan panduan ini saat bekerja di `crates/gaggle-core`, `crates/gaggle-tools`, `crates/gaggle-mcp`, `crates/gaggle-recipes`, atau `crates/gaggle-server`.

## 1. Urutan 7 Langkah Keputusan Izin (`PermissionEngine`)
Sebelum `tool_calls` dieksekusi, urutan evaluasi di `crates/gaggle-core/src/permission_engine.rs` wajib dipertahankan:
1. `chat_only`: Tolak semua tool tanpa terkecuali.
2. `command_rules` (khusus shell): Aksi `deny` **selalu menolak**, termasuk ketika chat berada di mode `full`.
3. `full`: Langsung jalankan tool.
4. `tool_permissions`: `never_allow` menolak, `always_allow` menjalankan, `ask_before` meminta approval.
5. Anotasi Read-Only (`extension_tools.is_read_only == 1`): Jalankan tanpa approval.
6. Mode `smart`: Cek `tool_risk_cache` berdasarkan `(tool_name, args_fingerprint)`.
7. Default: Buat baris `approvals` berstatus `pending` dan ubah `tool_calls.status` menjadi `awaiting_approval`.

## 2. Durable Runs & Pemulihan Restart (`RunEngine`)
- Semua kejadian selama run wajib dicatat berurutan di `run_events` (`UNIQUE(run_id, seq)`) sebagai satu-satunya sumber kebenaran untuk replay SSE.
- Saat server start (`gaggle_server::initialize_server_state`), semua `runs` berstatus `running` atau `waiting_approval` dipulihkan dari `run_checkpoints` terakhir dan `resume_count` ditambah 1.

## 3. Unified Extension & Batas Tool Aktif
- Seluruh tool bawaan (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) didaftarkan sebagai `extensions` dengan `kind = 'builtin'` dan `source = 'bundled'`.
- Jaga jumlah tool aktif per chat di bawah `RECOMMENDED_MAX_ACTIVE_TOOLS` (25 tool) di `gaggle-mcp`.

## 4. Recipe & Subagent
- Parameter recipe yang `required = 1` tidak boleh memiliki `default_value`; parameter `required = 0` wajib memiliki `default_value`.
- Sub-recipe dan subagent berjalan sebagai chat anak (`chats.parent_chat_id` dan `chats.parent_tool_call_id` dengan `chat_type = 'subagent'`).
