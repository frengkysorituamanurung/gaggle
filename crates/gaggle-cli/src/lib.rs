//! Library entry point for the `gaggle` command-line interface.
//!
//! Keeping the export/import/backup logic in a library crate makes the CLI
//! behavior directly testable without spawning a process.

pub mod backup;
pub mod chat_bundle;
pub mod goose_import;
pub mod memory_export;
