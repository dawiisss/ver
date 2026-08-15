use serde::{Deserialize, Serialize};

use crate::importers::ImporterError;
use crate::models::Connection;

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupPayload {
    pub version: String,
    pub timestamp: u64,
    pub connections: Vec<Connection>,
}

/// Exports connections list to a JSON string.
pub fn export_connections_json(connections: &[Connection]) -> Result<String, ImporterError> {
    let payload = BackupPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        connections: connections.to_vec(),
    };

    Ok(serde_json::to_string_pretty(&payload)?)
}

/// Imports connections from a JSON string (either formatted with BackupPayload or plain array).
pub fn import_connections_json(content: &str) -> Result<Vec<Connection>, ImporterError> {
    // Try BackupPayload envelope first
    if let Ok(payload) = serde_json::from_str::<BackupPayload>(content) {
        return Ok(payload.connections);
    }

    // Try plain array of Connection
    if let Ok(connections) = serde_json::from_str::<Vec<Connection>>(content) {
        return Ok(connections);
    }

    // Try single Connection
    if let Ok(single) = serde_json::from_str::<Connection>(content) {
        return Ok(vec![single]);
    }

    Err(ImporterError::InvalidFormat(
        "Invalid JSON structure for connection backup".to_string(),
    ))
}
