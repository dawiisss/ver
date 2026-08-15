pub mod json_backup;
pub mod rdp_file;
pub mod remmina;
pub mod ssh_config;

use std::fmt;
use crate::models::Connection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportConflictStrategy {
    SkipDuplicates,
    Overwrite,
    RenameWithSuffix,
}

#[derive(Debug)]
pub enum ImporterError {
    Io(std::io::Error),
    Parse(String),
    InvalidFormat(String),
    Json(serde_json::Error),
}

impl fmt::Display for ImporterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImporterError::Io(e) => write!(f, "I/O error: {}", e),
            ImporterError::Parse(s) => write!(f, "Parse error: {}", s),
            ImporterError::InvalidFormat(s) => write!(f, "Invalid format: {}", s),
            ImporterError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for ImporterError {}

impl From<std::io::Error> for ImporterError {
    fn from(e: std::io::Error) -> Self {
        ImporterError::Io(e)
    }
}

impl From<serde_json::Error> for ImporterError {
    fn from(e: serde_json::Error) -> Self {
        ImporterError::Json(e)
    }
}

pub use json_backup::{export_connections_json, import_connections_json};
pub use rdp_file::{export_rdp_file, import_rdp_content, import_rdp_file};
pub use remmina::{import_remmina_content, import_remmina_file, scan_remmina_profiles};
pub use ssh_config::{import_ssh_config_content, import_ssh_config_file};

/// Merges imported connections into the existing list according to the specified conflict strategy.
/// Returns (added_count, updated_count, skipped_count).
pub fn merge_imported_connections(
    existing: &mut Vec<Connection>,
    imported: Vec<Connection>,
    strategy: ImportConflictStrategy,
) -> (usize, usize, usize) {
    let mut added = 0;
    let mut updated = 0;
    let mut skipped = 0;

    for mut new_conn in imported {
        let existing_idx = existing.iter().position(|c| {
            c.id == new_conn.id
                || (c.name.eq_ignore_ascii_case(&new_conn.name)
                    && c.host.eq_ignore_ascii_case(&new_conn.host)
                    && c.protocol == new_conn.protocol)
        });

        match existing_idx {
            Some(idx) => match strategy {
                ImportConflictStrategy::SkipDuplicates => {
                    skipped += 1;
                }
                ImportConflictStrategy::Overwrite => {
                    new_conn.id = existing[idx].id.clone();
                    existing[idx] = new_conn;
                    updated += 1;
                }
                ImportConflictStrategy::RenameWithSuffix => {
                    new_conn.id = uuid::Uuid::new_v4().to_string();
                    new_conn.name = format!("{} (Imported)", new_conn.name);
                    existing.push(new_conn);
                    added += 1;
                }
            },
            None => {
                existing.push(new_conn);
                added += 1;
            }
        }
    }

    (added, updated, skipped)
}
