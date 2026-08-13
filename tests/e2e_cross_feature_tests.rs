use beautiful_goodall::models::{AppConfig, Connection, Protocol, VncScaling};
use beautiful_goodall::secrets;
use beautiful_goodall::storage;
use beautiful_goodall::ui::{ConnectionEditor, MainWindow, PreferencesWindow};
use beautiful_goodall::vnc::{VncClient, VncWidget};
use tempfile::tempdir;

// ============================================================================
// Tier 3: Cross-Feature Combinations
// ============================================================================

#[test]
fn test_t3_cross_feature_storage_and_keyring_roundtrip() {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("connections.json");

    let mut conn1 = Connection::default();
    conn1.name = "Database Server".to_string();
    conn1.host = "10.0.0.5".to_string();
    conn1.protocol = Protocol::Ssh;
    let conn1_id = conn1.id.clone();
    let pass1 = "DbPass123!Secure";

    let mut conn2 = Connection::default();
    conn2.name = "VNC Workstation".to_string();
    conn2.host = "10.0.0.10".to_string();
    conn2.protocol = Protocol::Vnc;
    let conn2_id = conn2.id.clone();
    let pass2 = "VncPass456!Remote";

    // 1. Save passwords to Secret Service / Keyring
    secrets::set_password_sync(&conn1_id, pass1).expect("Failed to store pass1");
    secrets::set_password_sync(&conn2_id, pass2).expect("Failed to store pass2");

    // 2. Save connections metadata to JSON
    let connections = vec![conn1.clone(), conn2.clone()];
    storage::save_connections_to_path(&file_path, &connections).expect("Failed to save connections");

    // 3. Reload connections metadata from JSON
    let reloaded = storage::load_connections_from_path(&file_path).expect("Failed to load connections");
    assert_eq!(reloaded.len(), 2);
    assert_eq!(reloaded[0].name, "Database Server");
    assert_eq!(reloaded[1].name, "VNC Workstation");

    // 4. Retrieve passwords from Keyring using reloaded IDs
    let loaded_pass1 = secrets::get_password_sync(&reloaded[0].id).expect("Failed to retrieve pass1");
    let loaded_pass2 = secrets::get_password_sync(&reloaded[1].id).expect("Failed to retrieve pass2");

    assert_eq!(loaded_pass1, Some(pass1.to_string()));
    assert_eq!(loaded_pass2, Some(pass2.to_string()));

    // 5. Cleanup keyring
    secrets::delete_password_sync(&conn1_id).expect("Failed to delete pass1");
    secrets::delete_password_sync(&conn2_id).expect("Failed to delete pass2");
}

#[test]
fn test_t3_cross_feature_config_file_updates_and_theme_persistence() {
    let dir = tempdir().expect("Failed to create temp dir");
    let config_path = dir.path().join("config.json");

    // 1. Initialize main window with default config
    let mut window = MainWindow::new(Vec::new(), AppConfig::default());
    assert_eq!(window.config.theme, "default");

    // 2. Open preferences window and change theme to "dark"
    let mut pref_win = PreferencesWindow::new(window.config.clone());
    pref_win.set_theme("dark");

    // 3. Update main window config and persist to config.json
    window.config = pref_win.config.clone();
    storage::save_config_to_path(&config_path, &window.config).expect("Failed to persist theme");

    // 4. Reload config from disk
    let loaded_config = storage::load_config_from_path(&config_path).expect("Failed to reload config");
    assert_eq!(loaded_config.theme, "dark");
}

#[test]
fn test_t3_cross_feature_editor_mutation_saves_to_storage_and_keyring() {
    let dir = tempdir().expect("Failed to create temp dir");
    let connections_file = dir.path().join("connections.json");

    let mut conn = Connection::default();
    conn.name = "Initial Server".to_string();
    conn.host = "192.168.1.5".to_string();
    let conn_id = conn.id.clone();
    let initial_pass = "InitialPass123";

    secrets::set_password_sync(&conn_id, initial_pass).expect("Failed to save initial pass");
    storage::save_connections_to_path(&connections_file, &[conn.clone()]).expect("Failed to save initial JSON");

    // Perform mutation in ConnectionEditor UI
    let mut editor = ConnectionEditor::new(conn, initial_pass.to_string());
    editor.update_host("192.168.1.100");
    editor.update_port(8080);
    editor.update_password("MutatedPassword789!");

    assert!(editor.is_dirty);

    // Save updated connection to disk and password to keyring
    storage::save_connections_to_path(&connections_file, &[editor.connection.clone()]).expect("Failed to save mutated JSON");
    secrets::set_password_sync(&conn_id, &editor.password).expect("Failed to save mutated password");

    // Reload and verify mutations
    let reloaded_conns = storage::load_connections_from_path(&connections_file).expect("Failed to reload JSON");
    assert_eq!(reloaded_conns[0].host, "192.168.1.100");
    assert_eq!(reloaded_conns[0].port, 8080);

    let reloaded_pass = secrets::get_password_sync(&conn_id).expect("Failed to reload password");
    assert_eq!(reloaded_pass, Some("MutatedPassword789!".to_string()));

    secrets::delete_password_sync(&conn_id).expect("Failed to delete password");
}

#[test]
fn test_t3_cross_feature_vnc_scaling_switches_during_session() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let client = VncClient::new("127.0.0.1".to_string(), 5900, VncScaling::OriginalSize);

    let frame = client.process_frame_buffer(&[255, 0, 0, 0, 255, 0], 2, 1);
    widget.render_frame(frame);

    assert_eq!(widget.scaling, VncScaling::OriginalSize);

    // Switch scaling mode mid-session to Fit to Window
    widget.set_scaling(VncScaling::FitToWindow);
    assert_eq!(widget.scaling, VncScaling::FitToWindow);

    // Switch scaling mode to Stretch
    widget.set_scaling(VncScaling::Stretch);
    assert_eq!(widget.scaling, VncScaling::Stretch);
}
