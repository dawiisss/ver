use beautiful_goodall::launcher::{build_rdp_args, build_ssh_args};
use beautiful_goodall::models::{Connection, Protocol, VncScaling};
use beautiful_goodall::network::build_wol_packet;
use beautiful_goodall::secrets;
use beautiful_goodall::storage::{load_connections_from_path, save_connections_to_path};
use beautiful_goodall::ui::MainWindow;
use beautiful_goodall::vnc::{VncClient, VncWidget};
use tempfile::tempdir;

// ============================================================================
// Tier 4: Real-World Workload Scenarios
// ============================================================================

#[test]
fn test_t4_workload_migrate_legacy_python_connection_format() {
    let dir = tempdir().expect("Failed to create temp dir");
    let legacy_file = dir.path().join("connections.json");

    // Legacy Python JSON format with extra python-specific fields, omitted fields, and snake_case format
    let legacy_json = r#"[
        {
            "id": "legacy-py-uuid-001",
            "name": "Legacy Python App Server",
            "protocol": "rdp",
            "host": "192.168.1.50",
            "port": 3389,
            "username": "pyadmin",
            "group": "Python Legacy",
            "mac_address": "00:11:22:33:44:55",
            "python_extra_setting": "deprecated_value",
            "gtk_widget_cache": true
        },
        {
            "id": "legacy-py-uuid-002",
            "name": "Legacy VNC Host",
            "protocol": "vnc",
            "host": "10.0.0.12",
            "python_class": "VncConnectionHandler"
        }
    ]"#;

    std::fs::write(&legacy_file, legacy_json).expect("Failed to write legacy JSON file");

    // Rust storage engine reads legacy file
    let connections = load_connections_from_path(&legacy_file).expect("Rust storage must parse legacy Python JSON");
    assert_eq!(connections.len(), 2);

    assert_eq!(connections[0].id, "legacy-py-uuid-001");
    assert_eq!(connections[0].name, "Legacy Python App Server");
    assert_eq!(connections[0].protocol, Protocol::Rdp);
    assert_eq!(connections[0].port, 3389);
    assert_eq!(connections[0].username, "pyadmin");
    assert_eq!(connections[0].group, "Python Legacy");
    assert_eq!(connections[0].mac_address, "00:11:22:33:44:55");

    assert_eq!(connections[1].id, "legacy-py-uuid-002");
    assert_eq!(connections[1].name, "Legacy VNC Host");
    assert_eq!(connections[1].protocol, Protocol::Vnc);
    assert_eq!(connections[1].port, 3389); // Default port fallback
    assert_eq!(connections[1].group, "Default");

    // Re-save in standard 4-space Rust JSON format
    save_connections_to_path(&legacy_file, &connections).expect("Failed to save re-migrated JSON");

    let migrated_content = std::fs::read_to_string(&legacy_file).expect("Failed to read migrated file");
    assert!(migrated_content.contains("    \"name\": \"Legacy Python App Server\""));
    assert!(!migrated_content.contains("python_extra_setting"));
    assert!(!migrated_content.contains("gtk_widget_cache"));
}

#[test]
fn test_t4_workload_multi_group_connection_persistence_and_grouping() {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("connections.json");

    let mut c1 = Connection::default();
    c1.name = "Web Front 01".to_string();
    c1.group = "Web Tier".to_string();

    let mut c2 = Connection::default();
    c2.name = "Web API 01".to_string();
    c2.group = "Web Tier".to_string();

    let mut c3 = Connection::default();
    c3.name = "PostgreSQL Main".to_string();
    c3.group = "Data Tier".to_string();

    let mut c4 = Connection::default();
    c4.name = "Redis Primary".to_string();
    c4.group = "Data Tier".to_string();

    let mut c5 = Connection::default();
    c5.name = "Jump Host".to_string();
    c5.group = "Ops Tier".to_string();

    let original = vec![c1, c2, c3, c4, c5];
    save_connections_to_path(&file_path, &original).expect("Failed to save multi-group connections");

    let reloaded = load_connections_from_path(&file_path).expect("Failed to reload multi-group connections");
    assert_eq!(reloaded.len(), 5);

    let window = MainWindow::new(reloaded, Default::default());
    let grouped = window.grouped_connections();

    assert_eq!(grouped.len(), 3); // Web Tier, Data Tier, Ops Tier
    assert_eq!(grouped.get("Web Tier").unwrap().len(), 2);
    assert_eq!(grouped.get("Data Tier").unwrap().len(), 2);
    assert_eq!(grouped.get("Ops Tier").unwrap().len(), 1);
}

#[test]
fn test_t4_workload_full_rdp_connection_lifecycle() {
    let dir = tempdir().expect("Failed to create temp dir");
    let connections_file = dir.path().join("connections.json");

    // 1. Initial load returns empty connections list
    let initial_conns = load_connections_from_path(&connections_file).expect("Should load empty");
    assert!(initial_conns.is_empty());

    // 2. Create new RDP connection
    let mut conn = Connection::default();
    conn.name = "Prod Windows Server".to_string();
    conn.protocol = Protocol::Rdp;
    conn.host = "192.168.10.50".to_string();
    conn.port = 3389;
    conn.username = "Administrator".to_string();
    conn.mac_address = "AA:BB:CC:DD:EE:FF".to_string();
    conn.group = "Production Windows".to_string();
    conn.advanced_settings.clipboard_sharing = true;
    conn.advanced_settings.color_depth = 32;
    conn.advanced_settings.rdp_multimon = true;
    conn.advanced_settings.rdp_fullscreen = true;

    let conn_id = conn.id.clone();
    let password = "ComplexP@ssw0rd2026!";

    // 3. Save connection to JSON and password to secrets vault
    save_connections_to_path(&connections_file, &[conn.clone()]).expect("Failed to save JSON");
    secrets::set_password_sync(&conn_id, password).expect("Failed to save secret");

    // 4. Verify JSON output has 4-space indent
    let raw_json = std::fs::read_to_string(&connections_file).expect("Failed to read raw JSON");
    assert!(raw_json.contains("    \"name\": \"Prod Windows Server\""));

    // 5. Reload connection list and secret
    let reloaded_conns = load_connections_from_path(&connections_file).expect("Failed to reload JSON");
    assert_eq!(reloaded_conns.len(), 1);
    assert_eq!(reloaded_conns[0].name, "Prod Windows Server");

    let reloaded_pass = secrets::get_password_sync(&conn_id).expect("Failed to reload password");
    assert_eq!(reloaded_pass, Some(password.to_string()));

    // 6. Wake-on-LAN trigger check
    let wol_packet = build_wol_packet(&reloaded_conns[0].mac_address).expect("WoL packet failed");
    assert_eq!(wol_packet.len(), 102);

    // 7. Launch CLI argument generation check
    let rdp_args = build_rdp_args(&reloaded_conns[0], reloaded_pass.as_deref());
    assert!(rdp_args.contains(&"/v:192.168.10.50:3389".to_string()));
    assert!(rdp_args.contains(&"/u:Administrator".to_string()));
    assert!(rdp_args.contains(&format!("/p:{}", password)));
    assert!(rdp_args.contains(&"+clipboard".to_string()));
    assert!(rdp_args.contains(&"/multimon".to_string()));

    // 8. Delete connection
    save_connections_to_path(&connections_file, &[]).expect("Failed to clear connections");
    secrets::delete_password_sync(&conn_id).expect("Failed to delete password");

    // 9. Final empty state check
    let final_conns = load_connections_from_path(&connections_file).expect("Failed to load");
    assert!(final_conns.is_empty());
    assert_eq!(secrets::get_password_sync(&conn_id).unwrap(), None);
}

#[test]
fn test_t4_workload_full_vnc_embedded_session_lifecycle() {
    let mut conn = Connection::default();
    conn.name = "Linux Desktop".to_string();
    conn.protocol = Protocol::Vnc;
    conn.host = "10.0.0.100".to_string();
    conn.port = 5900;
    conn.advanced_settings.vnc_scaling = VncScaling::FitToWindow;

    // Simulate launching embedded VNC widget session
    let mut widget = VncWidget::new(conn.advanced_settings.vnc_scaling.clone());
    let client = VncClient::new(conn.host.clone(), conn.port, conn.advanced_settings.vnc_scaling.clone());

    // Stream 1 frame update
    let frame_rgb = vec![0, 0, 0, 255, 255, 255]; // Black and White 2x1
    let frame_update = client.process_frame_buffer(&frame_rgb, 2, 1);
    widget.render_frame(frame_update);

    assert!(widget.current_frame.is_some());
    assert_eq!(widget.current_frame.as_ref().unwrap().width, 2);

    // Propagate user input
    widget.send_pointer_event(10, 20, 0);
    widget.send_key_event(0x0061, true); // 'a' key down

    assert_eq!(widget.events_sent.len(), 2);
}

#[test]
fn test_t4_workload_full_ssh_session_lifecycle() {
    let mut conn = Connection::default();
    conn.name = "Ubuntu Router".to_string();
    conn.protocol = Protocol::Ssh;
    conn.host = "192.168.1.1".to_string();
    conn.port = 22;
    conn.username = "admin".to_string();

    let ssh_args = build_ssh_args(&conn);
    assert_eq!(ssh_args, vec!["ssh", "admin@192.168.1.1"]);
}
