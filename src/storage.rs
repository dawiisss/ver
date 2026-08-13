use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::ser::PrettyFormatter;
use serde_json::Serializer;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{AppConfig, Connection};

/// Returns the configuration directory path (~/.config/ver).
pub fn get_config_dir() -> Result<PathBuf> {
    if let Some(config_base) = dirs::config_dir() {
        Ok(config_base.join("ver"))
    } else {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config").join("ver"))
    }
}

/// Returns the full path to connections.json (~/.config/ver/connections.json).
pub fn get_connections_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("connections.json"))
}

/// Returns the full path to config.json (~/.config/ver/config.json).
pub fn get_config_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

/// Helper function to format serializable data to JSON string with 4-space indentation.
pub fn to_json_4spaces<T: Serialize + ?Sized>(data: &T) -> Result<String> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(b"    ");
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);
    data.serialize(&mut serializer)
        .context("Failed to serialize data to JSON with 4-space indent")?;
    buf.push(b'\n');
    let json_str = String::from_utf8(buf).context("Serialized JSON is not valid UTF-8")?;
    Ok(json_str)
}

fn backup_corrupt_file(path: &Path) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_path = PathBuf::from(format!("{}.corrupt.{}", path.display(), timestamp));
    if let Err(e) = fs::copy(path, &backup_path) {
        eprintln!("Failed to backup corrupt file {:?}: {}", path, e);
    } else {
        eprintln!("Corrupt file backed up to {:?}", backup_path);
    }
}

/// Loads connections from a specific file path.
/// Returns an empty vector if the file does not exist.
/// Automatically backs up corrupt JSON or non-UTF8 files and returns an empty vector.
pub fn load_connections_from_path(path: &Path) -> Result<Vec<Connection>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read connections file at {:?}: {}", path, e);
            return Ok(Vec::new());
        }
    };
    let content = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Non-UTF8 connections file at {:?}: {}", path, e);
            backup_corrupt_file(path);
            return Ok(Vec::new());
        }
    };
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<Vec<Connection>>(&content) {
        Ok(mut connections) => {
            for conn in &mut connections {
                conn.sanitize();
            }
            Ok(connections)
        }
        Err(e) => {
            eprintln!("Corrupt connections JSON at {:?}: {}", path, e);
            backup_corrupt_file(path);
            Ok(Vec::new())
        }
    }
}

/// Saves connections to a specific file path atomically using 4-space indentation.
/// Automatically creates parent directories if needed.
pub fn save_connections_to_path(path: &Path, connections: &[Connection]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory structure for {:?}", parent))?;
    let json_str = to_json_4spaces(connections)?;
    let mut temp_file = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {:?}", parent))?;
    temp_file
        .write_all(json_str.as_bytes())
        .with_context(|| format!("Failed to write to temp file for {:?}", path))?;
    temp_file
        .persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Failed to persist temp file to {:?}", path))?;
    Ok(())
}

/// Loads connections from standard location (~/.config/ver/connections.json).
pub fn load_connections() -> Result<Vec<Connection>> {
    let path = get_connections_file_path()?;
    load_connections_from_path(&path)
}

/// Saves connections to standard location (~/.config/ver/connections.json).
pub fn save_connections(connections: &[Connection]) -> Result<()> {
    let path = get_connections_file_path()?;
    save_connections_to_path(&path, connections)
}

/// Loads app configuration from a specific file path.
/// Returns default AppConfig if the file does not exist.
/// Automatically backs up corrupt JSON or non-UTF8 files and returns default AppConfig.
pub fn load_config_from_path(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read config file at {:?}: {}", path, e);
            return Ok(AppConfig::default());
        }
    };
    let content = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Non-UTF8 config file at {:?}: {}", path, e);
            backup_corrupt_file(path);
            return Ok(AppConfig::default());
        }
    };
    if content.trim().is_empty() {
        return Ok(AppConfig::default());
    }
    match serde_json::from_str::<AppConfig>(&content) {
        Ok(config) => Ok(config),
        Err(e) => {
            eprintln!("Corrupt config JSON at {:?}: {}", path, e);
            backup_corrupt_file(path);
            Ok(AppConfig::default())
        }
    }
}

/// Saves app configuration to a specific file path atomically using 4-space indentation.
/// Automatically creates parent directories if needed.
pub fn save_config_to_path(path: &Path, config: &AppConfig) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory structure for {:?}", parent))?;
    let json_str = to_json_4spaces(config)?;
    let mut temp_file = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {:?}", parent))?;
    temp_file
        .write_all(json_str.as_bytes())
        .with_context(|| format!("Failed to write to temp file for {:?}", path))?;
    temp_file
        .persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Failed to persist temp file to {:?}", path))?;
    Ok(())
}

/// Loads app configuration from standard location (~/.config/ver/config.json).
pub fn load_config() -> Result<AppConfig> {
    let path = get_config_file_path()?;
    load_config_from_path(&path)
}

/// Saves app configuration to standard location (~/.config/ver/config.json).
pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = get_config_file_path()?;
    save_config_to_path(&path, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Connection, Protocol};
    use tempfile::tempdir;

    #[test]
    fn test_python_4space_indent_formatting() {
        let mut conn = Connection::default();
        conn.id = "11111111-2222-3333-4444-555555555555".to_string();
        conn.name = "Test Server".to_string();
        conn.protocol = Protocol::Vnc;
        conn.host = "192.168.1.100".to_string();
        conn.port = 5900;
        conn.username = "admin".to_string();
        conn.group = "Servers".to_string();

        let connections = vec![conn];
        let json_str = to_json_4spaces(&connections).expect("JSON serialization failed");

        let lines: Vec<&str> = json_str.lines().collect();
        assert!(lines.len() > 5);
        assert!(
            lines[1].starts_with("    {"),
            "Line 1 must start with 4 spaces: '{}'",
            lines[1]
        );
        assert!(
            lines[2].starts_with("        \"id\":"),
            "Line 2 must start with 8 spaces: '{}'",
            lines[2]
        );
    }

    #[test]
    fn test_roundtrip_storage_save_load() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("connections.json");

        let mut conn1 = Connection::default();
        conn1.id = "11111111-1111-1111-1111-111111111111".to_string();
        conn1.name = "Server 1".to_string();
        conn1.protocol = Protocol::Rdp;

        let mut conn2 = Connection::default();
        conn2.id = "22222222-2222-2222-2222-222222222222".to_string();
        conn2.name = "Server 2".to_string();
        conn2.protocol = Protocol::Vnc;
        conn2.advanced_settings.vnc_shared = true;

        let original = vec![conn1, conn2];

        save_connections_to_path(&file_path, &original).expect("Save must succeed");
        assert!(file_path.exists());

        let loaded = load_connections_from_path(&file_path).expect("Load must succeed");
        assert_eq!(
            original, loaded,
            "Loaded connections must match original exactly"
        );
    }

    #[test]
    fn test_load_nonexistent_file_returns_empty_vec() {
        let dir = tempdir().expect("Failed to create temp dir");
        let non_existent_path = dir.path().join("does_not_exist.json");

        let result = load_connections_from_path(&non_existent_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_load_corrupted_json_backs_up_and_returns_empty() {
        let dir = tempdir().expect("Failed to create temp dir");
        let corrupted_path = dir.path().join("corrupted.json");

        fs::write(&corrupted_path, "{ invalid json content ...").unwrap();

        let loaded =
            load_connections_from_path(&corrupted_path).expect("Should recover gracefully");
        assert!(loaded.is_empty());

        let entries = fs::read_dir(dir.path()).unwrap();
        let backup_exists = entries.filter_map(|e| e.ok()).any(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("corrupted.json.corrupt.")
        });
        assert!(
            backup_exists,
            "Corrupt backup file should have been created"
        );
    }

    #[test]
    fn test_load_save_config_roundtrip() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("config.json");

        let config = AppConfig {
            theme: "dark".to_string(),
            ..Default::default()
        };

        save_config_to_path(&config_path, &config).expect("Save config must succeed");
        let loaded = load_config_from_path(&config_path).expect("Load config must succeed");
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_dir_autocreate_on_save() {
        let dir = tempdir().expect("Failed to create temp dir");
        let nested_path = dir
            .path()
            .join("sub")
            .join("folder")
            .join("connections.json");

        let conn = Connection::default();
        save_connections_to_path(&nested_path, &[conn]).expect("Save to nested path must succeed");
        assert!(nested_path.exists());
    }

    #[test]
    fn test_load_non_utf8_binary_backs_up_and_returns_empty() {
        let dir = tempdir().expect("Failed to create temp dir");
        let bin_path = dir.path().join("binary.json");

        fs::write(&bin_path, &[0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01]).unwrap();

        let loaded = load_connections_from_path(&bin_path)
            .expect("Should recover gracefully from non-UTF8 binary");
        assert!(loaded.is_empty());

        let entries = fs::read_dir(dir.path()).unwrap();
        let backup_exists = entries.filter_map(|e| e.ok()).any(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("binary.json.corrupt.")
        });
        assert!(
            backup_exists,
            "Corrupt backup file should have been created for non-UTF8 binary"
        );
    }
}
