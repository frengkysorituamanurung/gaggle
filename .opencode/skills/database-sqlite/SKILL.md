---
name: database-sqlite
description: Design and maintain Gaggle's SQLite schema, SQLx migrations, indexes, transactions, search, backups, and recovery.
version: 1.0.0
---

# SQLite and SQLx Skill

## Mission
Keep local persistence correct, migratable, searchable, and resilient to interruption.

## Schema conventions
Follow repository conventions for UUIDv7 text IDs, Unix epoch milliseconds, JSON validation, booleans, foreign keys, and soft deletion. Do not introduce a competing convention without an ADR.

## Migration rules
- Use versioned SQLx migrations.
- Never edit a migration already released to users; add a new migration.
- Test migration from an empty database and from representative prior versions.
- Include rollback or recovery guidance when true rollback is not safe.
- Preserve existing data and document destructive transformations.
- Validate constraints and indexes against real query patterns.

## Runtime rules
- Enable and verify `PRAGMA foreign_keys=ON`.
- Define WAL, synchronous, and busy-timeout settings intentionally.
- Use transactions for related writes.
- Keep transactions short and avoid holding database locks across network/model calls.
- Handle `SQLITE_BUSY`, disk-full, read-only filesystem, corruption, and migration failures with actionable errors.
- Avoid N+1 query patterns and unbounded result reads.
- Use FTS5 only after confirming the SQLite build supports it on all target platforms.

## Durable events
- Enforce unique event sequence per run.
- Define event ordering and transaction boundaries.
- Make event append and state updates consistent.
- Do not treat exported JSONL as a second authoritative source if SQLite is the chosen source of truth.

## Data lifecycle
Define retention, deletion, export, backup, restore, and attachment cleanup. Ensure deleted records and files do not remain unintentionally linked. Do not promise secure erasure from SQLite/WAL/SSD without a validated process.

## Tests
Test constraints, concurrent writes, migration upgrades, crash interruption, backup restore, FTS behavior, and failure on read-only or full-disk scenarios.
