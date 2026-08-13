use beautiful_goodall::models::{AppConfig, Connection, Protocol, VncScaling};
use beautiful_goodall::ui::{ConnectionEditor, MainWindow};

#[test]
fn test_form_validation_boundary_invalid_ports() {
    // 1. Connection with port = 0
    let mut conn = Connection::default();
    conn.name = "Test Host".to_string();
    conn.host = "192.168.1.1".to_string();
    conn.port = 0;

    let editor = ConnectionEditor::new(conn.clone(), "pass".to_string());
    let err = editor.validate().expect_err("Port 0 must fail validation");
    assert_eq!(err, "Port must be a valid number between 1 and 65535");

    // 2. Connection::sanitize auto-corrects port 0 to default protocol port
    let mut conn_sanitize = conn.clone();
    assert!(conn_sanitize.sanitize());
    assert_eq!(conn_sanitize.port, 3389); // Default Rdp port

    let mut conn_vnc = Connection::new_with_protocol(Protocol::Vnc);
    conn_vnc.port = 0;
    assert!(conn_vnc.sanitize());
    assert_eq!(conn_vnc.port, 5900); // Default Vnc port

    let mut conn_ssh = Connection::new_with_protocol(Protocol::Ssh);
    conn_ssh.port = 0;
    assert!(conn_ssh.sanitize());
    assert_eq!(conn_ssh.port, 22); // Default Ssh port
}

#[test]
fn test_form_validation_boundary_empty_name_and_host() {
    // 1. Empty name
    let mut conn = Connection::default();
    conn.name = "".to_string();
    conn.host = "192.168.1.1".to_string();
    let editor_empty_name = ConnectionEditor::new(conn.clone(), "pass".to_string());
    let err_name = editor_empty_name.validate().expect_err("Empty name must fail validation");
    assert_eq!(err_name, "Connection name cannot be empty");

    // 2. Whitespace-only name
    conn.name = "   \t\n  ".to_string();
    let editor_ws_name = ConnectionEditor::new(conn.clone(), "pass".to_string());
    let err_ws_name = editor_ws_name.validate().expect_err("Whitespace name must fail validation");
    assert_eq!(err_ws_name, "Connection name cannot be empty");

    // 3. Empty host
    conn.name = "Valid Name".to_string();
    conn.host = "".to_string();
    let editor_empty_host = ConnectionEditor::new(conn.clone(), "pass".to_string());
    let err_host = editor_empty_host.validate().expect_err("Empty host must fail validation");
    assert_eq!(err_host, "Host address cannot be empty");

    // 4. Whitespace-only host
    conn.host = "   ".to_string();
    let editor_ws_host = ConnectionEditor::new(conn.clone(), "pass".to_string());
    let err_ws_host = editor_ws_host.validate().expect_err("Whitespace host must fail validation");
    assert_eq!(err_ws_host, "Host address cannot be empty");

    // 5. Valid name and host pass validation
    conn.host = "10.0.0.1".to_string();
    let editor_valid = ConnectionEditor::new(conn.clone(), "pass".to_string());
    assert!(editor_valid.validate().is_ok());

    // 6. Connection::sanitize resets empty/whitespace fields
    let mut conn_sanitize = Connection {
        name: "   ".to_string(),
        group: "   ".to_string(),
        ..Default::default()
    };
    assert!(conn_sanitize.sanitize());
    assert_eq!(conn_sanitize.name, "New Connection");
    assert_eq!(conn_sanitize.group, "Default");
}

#[test]
fn test_form_validation_boundary_malformed_macs() {
    let mut conn = Connection::default();
    conn.name = "Test Server".to_string();
    conn.host = "192.168.1.100".to_string();

    // 1. Empty MAC is valid (Ok(None))
    conn.mac_address = "".to_string();
    assert_eq!(conn.validate_mac(), Ok(None));
    let editor = ConnectionEditor::new(conn.clone(), "".to_string());
    assert!(editor.validate().is_ok());

    // 2. Whitespace MAC is valid (Ok(None))
    conn.mac_address = "   ".to_string();
    assert_eq!(conn.validate_mac(), Ok(None));

    // 3. Standard colon MAC -> normalized to 12 uppercase hex chars
    conn.mac_address = "00:11:22:33:44:55".to_string();
    assert_eq!(conn.validate_mac(), Ok(Some("001122334455".to_string())));

    // 4. Dash MAC -> normalized
    conn.mac_address = "aa-bb-cc-dd-ee-ff".to_string();
    assert_eq!(conn.validate_mac(), Ok(Some("AABBCCDDEEFF".to_string())));

    // 5. Malformed MAC: too short (10 hex chars)
    conn.mac_address = "00:11:22:33:44".to_string();
    let err_short = conn.validate_mac().expect_err("Too short MAC must fail");
    assert!(err_short.contains("Invalid MAC address format"));

    let editor_short_mac = ConnectionEditor::new(conn.clone(), "".to_string());
    assert!(editor_short_mac.validate().is_err());

    // 6. Malformed MAC: too long (14 hex chars)
    conn.mac_address = "00:11:22:33:44:55:66".to_string();
    assert!(conn.validate_mac().is_err());

    // 7. Malformed MAC: invalid hex characters ('G', 'Z')
    conn.mac_address = "00:11:22:33:44:ZZ".to_string();
    assert!(conn.validate_mac().is_err());
}

#[test]
fn test_appconfig_serde_deserialization_missing_legacy_fields() {
    // 1. Empty JSON object deserializes cleanly into defaults
    let json_empty = "{}";
    let config_empty: AppConfig = serde_json::from_str(json_empty)
        .expect("Empty JSON should deserialize into default AppConfig");
    assert_eq!(config_empty.theme, "default");
    assert_eq!(config_empty.default_protocol, Protocol::Rdp);
    assert_eq!(config_empty.auto_connect_last, false);
    assert_eq!(config_empty.default_vnc_scaling, VncScaling::OriginalSize);
    assert_eq!(config_empty.last_connected_id, None);

    // 2. Partial JSON with only theme specified
    let json_partial = r#"{"theme": "dark"}"#;
    let config_partial: AppConfig = serde_json::from_str(json_partial)
        .expect("Partial JSON should deserialize missing fields with defaults");
    assert_eq!(config_partial.theme, "dark");
    assert_eq!(config_partial.default_protocol, Protocol::Rdp);
    assert_eq!(config_partial.auto_connect_last, false);
    assert_eq!(config_partial.default_vnc_scaling, VncScaling::OriginalSize);

    // 3. JSON with unknown/legacy Python fields
    let json_legacy = r#"{
        "theme": "light",
        "color_scheme": "prefer-light",
        "legacy_window_width": 1024,
        "legacy_window_height": 768,
        "auto_save_interval": 30
    }"#;
    let config_legacy: AppConfig = serde_json::from_str(json_legacy)
        .expect("JSON with legacy fields should ignore unknown properties");
    assert_eq!(config_legacy.theme, "light");
    assert_eq!(config_legacy.default_protocol, Protocol::Rdp);

    // 4. Null vs string for last_connected_id
    let json_null_id = r#"{"last_connected_id": null}"#;
    let config_null_id: AppConfig = serde_json::from_str(json_null_id).unwrap();
    assert_eq!(config_null_id.last_connected_id, None);

    let json_str_id = r#"{"last_connected_id": "550e8400-e29b-41d4-a716-446655440000"}"#;
    let config_str_id: AppConfig = serde_json::from_str(json_str_id).unwrap();
    assert_eq!(config_str_id.last_connected_id, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
}

#[test]
fn test_search_filtering_logic_multi_field() {
    let mut conn1 = Connection::default();
    conn1.name = "Web Frontend".to_string();
    conn1.host = "192.168.1.10".to_string();
    conn1.group = "Production".to_string();
    conn1.username = "admin".to_string();
    conn1.protocol = Protocol::Rdp;

    let mut conn2 = Connection::default();
    conn2.name = "Database Primary".to_string();
    conn2.host = "10.0.0.25".to_string();
    conn2.group = "Infrastructure".to_string();
    conn2.username = "postgres".to_string();
    conn2.protocol = Protocol::Vnc;

    let mut conn3 = Connection::default();
    conn3.name = "Bastion Host".to_string();
    conn3.host = "172.16.0.1".to_string();
    conn3.group = "Security".to_string();
    conn3.username = "deploy".to_string();
    conn3.protocol = Protocol::Ssh;

    let mut window = MainWindow::new(vec![conn1, conn2, conn3], AppConfig::default());

    // 1. Empty filter returns all 3 connections
    assert_eq!(window.filtered_connections().len(), 3);

    // 2. Filter by Name (case-insensitive "frontend")
    window.set_search_filter("FRONTEND");
    let res = window.filtered_connections();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "Web Frontend");

    // 3. Filter by Host ("10.0.0.")
    window.set_search_filter("10.0.0.");
    let res = window.filtered_connections();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "Database Primary");

    // 4. Filter by Group ("security")
    window.set_search_filter("security");
    let res = window.filtered_connections();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "Bastion Host");

    // 5. Filter by Username ("postgres")
    window.set_search_filter("postgres");
    let res = window.filtered_connections();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "Database Primary");

    // 6. Filter by Protocol ("ssh")
    window.set_search_filter("ssh");
    let res = window.filtered_connections();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "Bastion Host");

    // 7. Non-matching search query
    window.set_search_filter("nonexistent_term");
    assert_eq!(window.filtered_connections().len(), 0);
}

#[test]
fn test_search_grouping_logic() {
    let mut conn1 = Connection::default();
    conn1.name = "App Alpha".to_string();
    conn1.group = "Zeta Group".to_string();

    let mut conn2 = Connection::default();
    conn2.name = "App Beta".to_string();
    conn2.group = "Alpha Group".to_string();

    let mut conn3 = Connection::default();
    conn3.name = "App Gamma".to_string();
    conn3.group = "Alpha Group".to_string();

    let window = MainWindow::new(vec![conn1, conn2, conn3], AppConfig::default());
    let grouped = window.grouped_connections();

    // BTreeMap keys must be sorted alphabetically: "Alpha Group", "Zeta Group"
    let group_names: Vec<&String> = grouped.keys().collect();
    assert_eq!(group_names, vec!["Alpha Group", "Zeta Group"]);

    assert_eq!(grouped.get("Alpha Group").unwrap().len(), 2);
    assert_eq!(grouped.get("Zeta Group").unwrap().len(), 1);
}
