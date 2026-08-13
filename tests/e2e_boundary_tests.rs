use beautiful_goodall::models::{AdvancedSettings, Connection, Protocol};
use beautiful_goodall::network::build_wol_packet;
use beautiful_goodall::secrets;
use beautiful_goodall::storage::load_connections_from_path;
use std::fs;
use tempfile::tempdir;

// ============================================================================
// Tier 2: Boundary & Corner Cases
// ============================================================================

#[test]
fn test_t2_boundary_empty_json_file() {
    let dir = tempdir().expect("Failed to create temp dir");
    let empty_file = dir.path().join("empty.json");
    fs::write(&empty_file, "").unwrap();

    let loaded =
        load_connections_from_path(&empty_file).expect("Loading empty file must return empty vec");
    assert!(loaded.is_empty());
}

#[test]
fn test_t2_boundary_corrupt_json_syntax() {
    let dir = tempdir().expect("Failed to create temp dir");
    let corrupt_file = dir.path().join("corrupt.json");
    fs::write(&corrupt_file, "{ invalid json syntax: [ ...").unwrap();

    let loaded = load_connections_from_path(&corrupt_file)
        .expect("Loading corrupt file must handle error gracefully");
    assert!(loaded.is_empty());

    let entries = fs::read_dir(dir.path()).unwrap();
    let backup_found = entries.filter_map(|e| e.ok()).any(|e| {
        e.file_name()
            .to_string_lossy()
            .contains("corrupt.json.corrupt.")
    });
    assert!(backup_found, "Corrupt backup file should be created");
}

#[test]
fn test_t2_boundary_missing_fields_defaults_injected() {
    let json = r#"{
        "id": "conn-001",
        "name": "Legacy Server",
        "protocol": "rdp",
        "host": "192.168.1.1"
    }"#;

    let conn: Connection = serde_json::from_str(json).expect("Should parse missing fields");
    assert_eq!(conn.name, "Legacy Server");
    assert_eq!(conn.port, 3389);
    assert_eq!(conn.group, "Default");
    assert_eq!(conn.advanced_settings, AdvancedSettings::default());
}

#[test]
fn test_t2_boundary_invalid_mac_address_formats() {
    assert!(build_wol_packet("").is_err());
    assert!(build_wol_packet("00:11").is_err());
    assert!(build_wol_packet("00:11:22:33:44:55:66:77").is_err());
    assert!(build_wol_packet("ZZ:YY:XX:WW:VV:UU").is_err());

    let mut conn = Connection::default();
    conn.mac_address = "invalid-mac".to_string();
    assert!(conn.validate_mac().is_err());

    conn.mac_address = "".to_string();
    assert_eq!(conn.validate_mac(), Ok(None));

    conn.mac_address = "00:11:22:33:44:55".to_string();
    assert_eq!(conn.validate_mac(), Ok(Some("001122334455".to_string())));
}

#[test]
fn test_t2_boundary_zero_port_resolution_and_sanitization() {
    let mut conn = Connection::default();
    conn.port = 0;
    conn.protocol = Protocol::Vnc;

    assert_eq!(conn.resolve_port(), 5900);

    let modified = conn.sanitize();
    assert!(modified);
    assert_eq!(conn.port, 5900);
}

#[test]
fn test_t2_boundary_unknown_protocol_strings_rejection() {
    let json = r#"{
        "id": "conn-999",
        "name": "Invalid Protocol",
        "protocol": "http"
    }"#;

    let res: Result<Connection, _> = serde_json::from_str(json);
    assert!(
        res.is_err(),
        "Unknown protocol string 'http' must fail deserialization"
    );
}

#[test]
fn test_t2_boundary_extreme_ports() {
    let mut conn_max = Connection::default();
    conn_max.port = 65535;
    let json_max = serde_json::to_string(&conn_max).unwrap();
    let deserialized_max: Connection = serde_json::from_str(&json_max).unwrap();
    assert_eq!(deserialized_max.port, 65535);
}

#[test]
fn test_t2_boundary_unicode_connection_fields() {
    let mut conn = Connection::default();
    conn.name = "服务器 🚀 Remote (Köln)".to_string();
    conn.group = "Testing 測試 Group".to_string();
    conn.username = "usr_ñandú_123".to_string();

    let json = serde_json::to_string(&conn).expect("Serialization failed");
    let deserialized: Connection = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(conn, deserialized);
}

#[test]
fn test_t2_boundary_missing_keyring_item_returns_none() {
    let result = secrets::get_password_sync("non-existent-uuid-999999").expect("Should not fail");
    assert_eq!(result, None);
}

#[test]
fn test_t2_boundary_connection_sanitize_invalid_uuid_and_whitespace() {
    let mut conn = Connection {
        id: "".to_string(),
        name: "   ".to_string(),
        group: "\t\n".to_string(),
        port: 0,
        protocol: Protocol::Ssh,
        advanced_settings: AdvancedSettings {
            color_depth: 99, // Invalid color depth
            ..Default::default()
        },
        ..Default::default()
    };

    let modified = conn.sanitize();
    assert!(modified);
    assert!(uuid::Uuid::parse_str(&conn.id).is_ok());
    assert_eq!(conn.name, "New Connection");
    assert_eq!(conn.group, "Default");
    assert_eq!(conn.port, 22);
    assert_eq!(conn.advanced_settings.color_depth, 0);
}
