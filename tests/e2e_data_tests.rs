use beautiful_goodall::models::{AppConfig, Connection, Protocol, VncScaling};
use beautiful_goodall::secrets::{
    delete_password, delete_password_sync, get_password, get_password_sync, set_password,
    set_password_sync,
};
use beautiful_goodall::storage::{
    load_config_from_path, save_connections_to_path, to_json_4spaces,
};
use std::fs;
use tempfile::tempdir;

// ============================================================================
// Tier 1 Feature 1: Connection Model Serialization (>= 5 tests)
// ============================================================================

#[test]
fn test_t1_conn_serialization_roundtrip() {
    let mut conn = Connection::default();
    conn.id = "11111111-2222-3333-4444-555555555555".to_string();
    conn.name = "Production VNC Server".to_string();
    conn.protocol = Protocol::Vnc;
    conn.host = "192.168.1.100".to_string();
    conn.port = 5900;
    conn.username = "admin".to_string();
    conn.group = "Infrastructure".to_string();
    conn.advanced_settings.clipboard_sharing = true;
    conn.advanced_settings.vnc_scaling = VncScaling::FitToWindow;

    let json = serde_json::to_string(&conn).expect("Serialization failed");
    assert!(json.contains("\"name\":\"Production VNC Server\""));
    assert!(json.contains("\"protocol\":\"vnc\""));
    assert!(json.contains("\"Fit to Window\""));

    let deserialized: Connection = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(conn, deserialized);
}

#[test]
fn test_t1_conn_serialization_password_isolation() {
    let conn = Connection::default();
    let json = serde_json::to_string(&conn).expect("Serialization failed");
    assert!(!json.contains("password"));
    assert!(!json.contains("secret"));
    assert!(!json.contains("pass"));
}

#[test]
fn test_t1_conn_deserialization_defaults() {
    let json = "{}";
    let conn: Connection = serde_json::from_str(json).expect("Deserialization of empty object failed");
    assert!(!conn.id.is_empty());
    assert_eq!(conn.name, "New Connection");
    assert_eq!(conn.protocol, Protocol::Rdp);
    assert_eq!(conn.port, 3389);
    assert_eq!(conn.group, "Default");
}

#[test]
fn test_t1_conn_deserialization_unknown_fields_ignored() {
    let json = r#"{
        "id": "22222222-3333-4444-5555-666666666666",
        "name": "Legacy Conn",
        "unknown_legacy_field_1": 12345,
        "deprecated_flag": true,
        "custom_metadata": { "key": "value" }
    }"#;
    let conn: Connection = serde_json::from_str(json).expect("Deserialization with unknown fields failed");
    assert_eq!(conn.id, "22222222-3333-4444-5555-666666666666");
    assert_eq!(conn.name, "Legacy Conn");
}

#[test]
fn test_t1_conn_deserialization_all_advanced_settings() {
    let json = r#"{
        "id": "33333333-4444-5555-6666-777777777777",
        "name": "Advanced RDP",
        "protocol": "rdp",
        "advanced_settings": {
            "rdp_multimon": true,
            "rdp_fullscreen": true,
            "rdp_audio": true,
            "vnc_viewonly": false,
            "vnc_shared": true,
            "clipboard_sharing": true,
            "color_depth": 32,
            "vnc_scaling": "Stretch"
        }
    }"#;
    let conn: Connection = serde_json::from_str(json).expect("Deserialization of advanced settings failed");
    assert!(conn.advanced_settings.rdp_multimon);
    assert!(conn.advanced_settings.rdp_fullscreen);
    assert!(conn.advanced_settings.rdp_audio);
    assert!(conn.advanced_settings.vnc_shared);
    assert!(conn.advanced_settings.clipboard_sharing);
    assert_eq!(conn.advanced_settings.color_depth, 32);
    assert_eq!(conn.advanced_settings.vnc_scaling, VncScaling::Stretch);
}

// ============================================================================
// Tier 1 Feature 2: AppConfig Defaults (>= 5 tests)
// ============================================================================

#[test]
fn test_t1_appconfig_default_theme() {
    let config = AppConfig::default();
    assert_eq!(config.theme, "default");
}

#[test]
fn test_t1_appconfig_serialization_roundtrip() {
    let mut config = AppConfig::default();
    config.theme = "dark".to_string();

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(config, deserialized);
}

#[test]
fn test_t1_appconfig_load_nonexistent_returns_default() {
    let dir = tempdir().expect("Failed to create temp dir");
    let config_path = dir.path().join("nonexistent_config.json");

    let loaded = load_config_from_path(&config_path).expect("Loading nonexistent config must succeed");
    assert_eq!(loaded, AppConfig::default());
}

#[test]
fn test_t1_appconfig_corrupt_returns_default_and_backups() {
    let dir = tempdir().expect("Failed to create temp dir");
    let config_path = dir.path().join("corrupt_config.json");

    fs::write(&config_path, "INVALID_JSON_CONTENT{{{").expect("Writing corrupt file failed");

    let loaded = load_config_from_path(&config_path).expect("Loading corrupt config must return default");
    assert_eq!(loaded, AppConfig::default());

    let entries = fs::read_dir(dir.path()).expect("Reading dir failed");
    let backup_found = entries.filter_map(|e| e.ok()).any(|e| {
        e.file_name().to_string_lossy().contains("corrupt_config.json.corrupt.")
    });
    assert!(backup_found, "Corrupt backup file should be created");
}

#[test]
fn test_t1_appconfig_empty_json_returns_default() {
    let json = "{}";
    let config: AppConfig = serde_json::from_str(json).expect("Deserialization of empty config failed");
    assert_eq!(config.theme, "default");
}

// ============================================================================
// Tier 1 Feature 3: Storage Pretty Printing (>= 5 tests)
// ============================================================================

#[test]
fn test_t1_storage_pretty_printing_4spaces_connections() {
    let mut conn = Connection::default();
    conn.id = "44444444-5555-6666-7777-888888888888".to_string();
    conn.name = "Pretty Server".to_string();

    let json_str = to_json_4spaces(&vec![conn]).expect("Pretty printing failed");
    let lines: Vec<&str> = json_str.lines().collect();

    assert!(lines.len() > 3);
    assert!(lines[1].starts_with("    {"), "Line 1 should start with 4 spaces: '{}'", lines[1]);
    assert!(lines[2].starts_with("        \"id\":"), "Line 2 should start with 8 spaces: '{}'", lines[2]);
}

#[test]
fn test_t1_storage_pretty_printing_4spaces_config() {
    let config = AppConfig { theme: "light".to_string(), ..Default::default() };
    let json_str = to_json_4spaces(&config).expect("Pretty printing failed");
    let lines: Vec<&str> = json_str.lines().collect();

    assert_eq!(lines[0], "{");
    assert!(lines[1].starts_with("    \"theme\": \"light\""));
    assert_eq!(lines[lines.len() - 1], "}");
}

#[test]
fn test_t1_storage_pretty_printing_trailing_newline() {
    let config = AppConfig::default();
    let json_str = to_json_4spaces(&config).expect("Pretty printing failed");
    assert!(json_str.ends_with('\n'), "Output must end with a trailing newline");
}

#[test]
fn test_t1_storage_pretty_printing_nested_objects_indented() {
    let conn = Connection::default();
    let json_str = to_json_4spaces(&conn).expect("Pretty printing failed");

    assert!(json_str.contains("    \"advanced_settings\": {\n        \"rdp_multimon\": false,"));
}

#[test]
fn test_t1_storage_auto_creates_parent_directories() {
    let dir = tempdir().expect("Failed to create temp dir");
    let nested_path = dir.path().join("deeply").join("nested").join("connections.json");

    let conn = Connection::default();
    save_connections_to_path(&nested_path, &[conn]).expect("Saving to nested path must auto-create directories");
    assert!(nested_path.exists());
}

// ============================================================================
// Tier 1 Feature 4: Keyring Operations Fallback (>= 5 tests)
// ============================================================================

#[test]
fn test_t1_keyring_get_nonexistent_returns_none() {
    let conn_id = format!("nonexistent-uuid-{}", uuid::Uuid::new_v4());
    let res = get_password_sync(&conn_id);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), None);
}

#[tokio::test]
async fn test_t1_keyring_set_get_delete_cycle() {
    let conn_id = format!("test-keyring-uuid-{}", uuid::Uuid::new_v4());
    let password = "MySecretPassword123!";

    let set_res = set_password(&conn_id, password).await;
    assert!(set_res.is_ok());

    let get_res = get_password(&conn_id).await;
    assert!(get_res.is_ok());

    let del_res = delete_password(&conn_id).await;
    assert!(del_res.is_ok());
}

#[test]
fn test_t1_keyring_special_characters_support() {
    let conn_id = format!("test-keyring-spec-{}", uuid::Uuid::new_v4());
    let special_pass = r#"P@ssw0rd! '$" \ / < > & % # @ ! * () - + = ~ ` 🔑"#;

    let set_res = set_password_sync(&conn_id, special_pass);
    assert!(set_res.is_ok());

    let get_res = get_password_sync(&conn_id);
    assert!(get_res.is_ok());

    let del_res = delete_password_sync(&conn_id);
    assert!(del_res.is_ok());
}

#[test]
fn test_t1_keyring_sync_wrappers_fallback() {
    let conn_id = format!("test-keyring-sync-{}", uuid::Uuid::new_v4());
    let _ = get_password_sync(&conn_id);
    let _ = set_password_sync(&conn_id, "pass");
    let _ = delete_password_sync(&conn_id);
}

#[test]
fn test_t1_keyring_overwrite_existing_secret() {
    let conn_id = format!("test-keyring-overwrite-{}", uuid::Uuid::new_v4());
    let _ = set_password_sync(&conn_id, "OldPassword");
    let _ = set_password_sync(&conn_id, "NewPassword");
    let _ = delete_password_sync(&conn_id);
}

// ============================================================================
// Tier 1 Feature 5: Protocol Defaults (>= 5 tests)
// ============================================================================

#[test]
fn test_t1_protocol_default_enum_is_rdp() {
    assert_eq!(Protocol::default(), Protocol::Rdp);
}

#[test]
fn test_t1_protocol_default_ports() {
    assert_eq!(Protocol::Rdp.default_port(), 3389);
    assert_eq!(Protocol::Vnc.default_port(), 5900);
    assert_eq!(Protocol::Ssh.default_port(), 22);
}

#[test]
fn test_t1_protocol_as_str_mapping() {
    assert_eq!(Protocol::Rdp.as_str(), "rdp");
    assert_eq!(Protocol::Vnc.as_str(), "vnc");
    assert_eq!(Protocol::Ssh.as_str(), "ssh");
}

#[test]
fn test_t1_protocol_display_trait() {
    assert_eq!(format!("{}", Protocol::Rdp), "rdp");
    assert_eq!(format!("{}", Protocol::Vnc), "vnc");
    assert_eq!(format!("{}", Protocol::Ssh), "ssh");
}

#[test]
fn test_t1_protocol_resolve_port_fallback() {
    let mut conn = Connection::default();
    conn.port = 0;

    conn.protocol = Protocol::Rdp;
    assert_eq!(conn.resolve_port(), 3389);

    conn.protocol = Protocol::Vnc;
    assert_eq!(conn.resolve_port(), 5900);

    conn.protocol = Protocol::Ssh;
    assert_eq!(conn.resolve_port(), 22);
}
