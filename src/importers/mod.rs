pub mod json_backup;
pub mod rdp_file;
pub mod remmina;
pub mod ssh_config;

use crate::models::Connection;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportConflictStrategy {
    SkipDuplicates,
    Overwrite,
    RenameWithSuffix,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImporterError {
    Io(String),
    Parse(String),
    InvalidFormat(String),
    Json(String),
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
        ImporterError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for ImporterError {
    fn from(e: serde_json::Error) -> Self {
        ImporterError::Json(e.to_string())
    }
}

pub use json_backup::{export_connections_json, import_connections_json};
pub use rdp_file::{export_rdp_file, import_rdp_content, import_rdp_file};
pub use remmina::{import_remmina_content, import_remmina_file, scan_remmina_profiles};
pub use ssh_config::{import_ssh_config_content, import_ssh_config_file};

/// Parses `host:port` string handling IPv6 bracketed notations (`[::1]:port`).
pub fn parse_host_port(server: &str, default_port: u16) -> (String, u16) {
    let trimmed = server.trim();
    if trimmed.is_empty() {
        return (String::new(), default_port);
    }

    if trimmed.starts_with('[') {
        if let Some(close_idx) = trimmed.find(']') {
            let host = trimmed[1..close_idx].to_string();
            let remainder = &trimmed[close_idx + 1..];
            if let Some(port_str) = remainder.strip_prefix(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    return (host, port);
                }
            }
            return (host, default_port);
        }
    }

    if let Some((h, p)) = trimmed.rsplit_once(':') {
        if !h.contains(':') {
            if let Ok(port) = p.parse::<u16>() {
                return (h.to_string(), port);
            }
        }
    }

    (trimmed.to_string(), default_port)
}

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
