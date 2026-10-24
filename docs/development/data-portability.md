# Data Portability: Export, Import, Backup, and Restore

Gaggle keeps all chats, memories, and attachments in a single local data
directory. This guide covers moving a single chat between projects or devices,
sharing memories through Git, and creating or restoring a full local backup.

> **Supported platforms:** macOS and Linux. Archive paths use `/` separators and
> restore staging happens on the same filesystem as the target database so
> replacement stays atomic on both platforms.

---

## 1. Data locations

| Data | Location |
| :--- | :--- |
| SQLite database | macOS: `~/Library/Application Support/gaggle/gaggle.db`<br>Linux: `~/.local/share/gaggle/gaggle.db` |
| Attachments | `<database directory>/attachments/` (optional; created only when attachments exist) |
| WAL/SHM sidecars | `<database path>-wal` and `<database path>-shm` (managed by SQLite) |
| Exported memories | `<project root>/.pal/memory/*.md` |
| Chat bundles | Any path supplied to `--output`; nothing is written outside it |

Use `gaggle db-path` to print the resolved database path for the current
machine. All commands accept `--db <path>` before the subcommand to target a
non-default database.

Close the Gaggle server (`gaggle serve`) and desktop app before restoring a
backup. SQLite handles concurrent readers, but replacing the database file while
another long-lived process has it open leaves that process reading the old
inode.

---

## 2. Export a chat bundle

```bash
gaggle export chat <chat-id> --output chat.json
```

The output is a versioned JSON bundle:

```json
{
  "format": "gaggle.chat-bundle",
  "version": 1,
  "exported_at": 1730000000000,
  "chat": { "...": "..." },
  "messages": [ "..." ],
  "message_parts": [ "..." ]
}
```

- The command prints a JSON summary containing `chat_id`, `output`, `messages`,
  and `message_parts` counts.
- The file is written atomically: an interrupted export never leaves a truncated
  bundle at the destination.
- The export reflects the database as stored. It never includes credential
  values; provider rows only ever hold `secret_ref` identifiers.

`GET /api/chats/{id}/export` returns the same `chat`, `messages`, and
`message_parts` payload for the Web UI.

---

## 3. Import a chat bundle

```bash
gaggle import chat chat.json --project <project-id>
```

Import always creates a **new chat**. It never edits, merges into, or overwrites
an existing chat.

Validation happens before any row is written:

- Bundle `format` must be `gaggle.chat-bundle` and `version` must be `1`.
- Chat title, enum fields (`chat_type`, `mode`, `permission_mode`), and
  `compact_threshold` must be valid.
- Chat, message, and message-part ids must be UUIDs and unique.
- Every message must belong to the bundle chat with a unique `seq >= 1`.
- Every message part must reference a bundled message with a unique
  `(message_id, idx)` pair.
- Bundles are limited to 20,000 messages, 100,000 parts, 8 MiB per part, and
  64 MiB on disk.

The target project must exist, be active, and not be archived.

Everything is inserted in one short transaction with fresh UUIDv7 ids. References
that cannot exist in the target database are intentionally dropped instead of
creating dangling foreign keys:

| Reference | Import behavior |
| :--- | :--- |
| `run_id` | Set to `NULL` (runs and events are not part of a chat bundle). |
| `parent_id` | Remapped when the parent is also in the bundle; otherwise dropped. |
| `tool_call_id` | Set to `NULL` (tool-call audit rows are not bundled). |
| `attachment_id` | Set to `NULL`; use a full backup to move attachment files. |
| `model_id` / `recipe_id` | Preserved only when that row already exists in the target database. |
| `worktree_id`, `parent_chat_id`, `schedule_id` | Set to `NULL` (machine- and schedule-specific). |
| Token counters, `archived_at` | Reset so the imported chat starts active. |

The command prints the imported chat plus `message_count` and `part_count`.

---

## 4. Export memories for Git

```bash
# Default: <project root>/.pal/memory/
gaggle export memory --project <project-id>

# Optional explicit directory (must stay inside the project workspace)
gaggle export memory --project <project-id> --output .pal/memory
```

One Markdown file is written per active project and global memory, named
`<kind>-<title-slug>-<id-prefix>.md`. File names contain only lowercase ASCII
letters, digits, `-`, `_`, and `.`, so they are safe on macOS and Linux.

- Existing memory files are replaced atomically; unrelated files in the
  directory are never deleted.
- Obvious credentials (`api_key=...`, `Authorization: ...`, private-key blocks,
  common token prefixes) are redacted before writing.
- Stored Markdown is escaped so a memory body cannot break the document
  structure.
- The output directory is resolved through the project workspace policy, so
  `..` traversal and symlinks that escape the project root are rejected.
- Commit `.pal/memory/` to share durable project knowledge with the team.

Soft-deleted memories are excluded from both chat-bundle and memory exports.

---

## 5. Create a full backup

```bash
gaggle backup --output backup.tar.gz
```

The archive contains:

| Entry | Contents |
| :--- | :--- |
| `gaggle.db` | Consistent SQLite snapshot produced with `VACUUM INTO`. |
| `attachments/...` | Every regular file below `<data dir>/attachments/`, when present. |
| `manifest.json` | Format version, creation time, database integrity/count summary, and a SHA-256 checksum plus size for every other file. |

Properties:

- **WAL-safe:** `VACUUM INTO` reads a consistent snapshot, including content
  still in the WAL. `-wal`/`-shm` files are not copied, so the archive can be
  opened directly.
- **Verified at creation:** the snapshot runs `PRAGMA integrity_check` and must
  contain applied Gaggle migrations before the archive is written.
- **Checksummed:** SHA-256 and byte size are recorded for each file, and the
  command prints the archive's own SHA-256.
- **No secrets:** the database stores only `secret_ref` identifiers for
  credentials, and the manifest contains only paths, sizes, and hashes.
- **Symlinks skipped:** symlinked files inside `attachments/` are not archived;
  a warning is logged with the skipped path.
- **Atomic output:** the archive is written to a temporary file in the output
  directory and renamed into place. `--force` is required to overwrite an
  existing archive.
- Full backups only. There are no incremental or cloud backups.

Example:

```bash
gaggle backup --output ~/Backups/gaggle-$(date +%Y%m%d).tar.gz
```

---

## 6. Verify and restore a backup

```bash
# Verify checksums and database integrity without changing anything
gaggle restore backup.tar.gz --dry-run

# Replace existing data after the archive has been verified
gaggle restore backup.tar.gz --force
```

Restore is deliberately conservative:

1. The archive is extracted into a staging directory **inside** the data
   directory. Entry paths must be relative, must not contain `..` or backslashes,
   and must match the allowed layout (`gaggle.db`, `attachments/...`).
   Absolute paths, symlinks, hard links, device files, duplicate entries, and
   undeclared entries are rejected.
2. Every extracted file is hashed while being written and compared against
   `manifest.json`. A size or SHA-256 mismatch aborts before existing data is
   touched.
3. The staged database is opened read-only and must pass
   `PRAGMA integrity_check`, contain applied Gaggle migrations, and match the
   manifest's database summary.
4. If `gaggle.db`, `gaggle.db-wal`, `gaggle.db-shm`, or `attachments/` already
   exist, restore refuses to continue unless `--force` is passed. `--dry-run`
   reports what would be replaced without modifying anything.
5. Existing data is moved to
   `<data dir>/gaggle-restore-previous-<unix-ms>/`, then the verified snapshot and
   attachments are renamed into place. If installation fails, the moves are
   reversed and the new files are removed.

The restore report includes `database`, `attachments_restored`,
`replaced_existing`, `dry_run`, and `previous_data_dir`. The previous data
directory is **kept** after a successful restore so you can roll back manually:

```bash
# Roll back by hand after stopping the server
mv "$DATA_DIR/gaggle-restore-previous-<ts>/gaggle.db" "$DATA_DIR/gaggle.db"
```

Verify a restore with:

```bash
gaggle doctor
gaggle project list
```

Delete old `gaggle-restore-previous-*` directories once you are satisfied; they
contain a full copy of the replaced database and attachments.

### Restore safety checklist

- Close `gaggle serve` and the desktop app first.
- Run `--dry-run` and confirm the archive SHA-256 matches the value printed by
  `gaggle backup`.
- Keep at least one recent backup before restoring an older one.
- Restore does not modify provider credentials: those live in the OS credential
  store, not in SQLite. A backup moved to another machine still needs its
  providers authenticated there.

### Backup confidentiality

A backup is a complete copy of local Gaggle data: chats, messages, memories,
project settings, and attachments. Provider credentials are never written to
SQLite (only `secret_ref` identifiers), so they are not in the archive, but
anything you typed into a chat or stored in project settings is. Treat backup
archives as sensitive:

- Store them in a location with appropriate filesystem permissions.
- Do not commit `backup.tar.gz` or `.pal/memory/` output that contains private
  context to a public repository.
- Prefer an encrypted backup volume if backups leave the machine.

Chat bundles and memory Markdown files redact obvious credential patterns, but
redaction is best-effort; review exported content before sharing it.

---

## 7. Troubleshooting

| Symptom | Cause / fix |
| :--- | :--- |
| `Refusing to replace existing Gaggle data without confirmation` | Re-run with `--force` after a `--dry-run`, or move the existing database aside. |
| `Checksum verification failed for ...` | Archive is corrupt or was modified. Use a different backup. |
| `Database snapshot failed integrity_check` | The database or snapshot is corrupt; do not restore it over working data. |
| `Backup archive contains an unsafe path component` | The archive is malformed or malicious; discard it. |
| `Chat bundle ... is not valid JSON` | Wrong file or truncated export; re-export from the source machine. |
| `Refusing to export memories outside project workspace` | Pass a relative path inside the project root or use the default. |
| `Backup output ... already exists` | Pass `--force` to replace the previous archive. |
