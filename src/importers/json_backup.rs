use serde::{Deserialize, Serialize};

use crate::importers::ImporterError;
use crate::models::{sanitize_connections, Connection};

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupPayload {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub version: String,
    pub timestamp: u64,
    pub connections: Vec<Connection>,
}

fn default_schema_version() -> u32 {
    1
}

/// Exports connections list to a JSON string.
pub fn export_connections_json(connections: &[Connection]) -> Result<String, ImporterError> {
    let payload = BackupPayload {
        schema_version: default_schema_version(),
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
    let value: serde_json::Value = serde_json::from_str(content)?;
    // An invalid envelope must not fall through into Connection's permissive defaults.
    let mut connections = if value.is_array() {
        serde_json::from_value::<Vec<Connection>>(value)?
    } else if value.get("connections").is_some()
        || value.get("version").is_some()
        || value.get("timestamp").is_some()
        || value.get("schema_version").is_some()
    {
        let payload = serde_json::from_value::<BackupPayload>(value)?;
        if payload.schema_version != default_schema_version() {
            return Err(ImporterError::InvalidFormat(format!(
                "Unsupported backup schema version: {}",
                payload.schema_version
            )));
        }
        payload.connections
    } else if value.get("host").is_some() {
        vec![serde_json::from_value::<Connection>(value)?]
    } else {
        return Err(ImporterError::InvalidFormat(
            "Expected a connection or connection backup".into(),
        ));
    };
    sanitize_connections(&mut connections);
    for conn in &connections {
        conn.validate_fields()
            .map_err(|e| ImporterError::InvalidFormat(format!("{}: {}", conn.name, e)))?;
    }
    Ok(connections)
}
