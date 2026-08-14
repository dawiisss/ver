use ver::models::{AppConfig, Connection};
use ver::ui::{
    ConnectionEditor, DiscoveredService, DiscoveryDialog, MainWindow, PreferencesWindow,
};

#[test]
fn test_main_window_initialization_and_filtering() {
    let conn1 = Connection {
        name: "Web Server".to_string(),
        host: "192.168.1.10".to_string(),
        group: "Web".to_string(),
        ..Default::default()
    };

    let conn2 = Connection {
        name: "Database Server".to_string(),
        host: "10.0.0.20".to_string(),
        group: "Database".to_string(),
        ..Default::default()
    };

    let window = MainWindow::new(vec![conn1, conn2], AppConfig::default());
    assert_eq!(window.filtered_connections().len(), 2);

    let mut window_filtered = MainWindow::new(window.connections.clone(), AppConfig::default());
    window_filtered.set_search_filter("Web");
    let filtered = window_filtered.filtered_connections();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Web Server");
}

#[test]
fn test_main_window_grouped_connections() {
    let conn1 = Connection {
        name: "App 1".to_string(),
        group: "Production".to_string(),
        ..Default::default()
    };

    let conn2 = Connection {
        name: "App 2".to_string(),
        group: "Production".to_string(),
        ..Default::default()
    };

    let conn3 = Connection {
        name: "Staging 1".to_string(),
        group: "Staging".to_string(),
        ..Default::default()
    };

    let window = MainWindow::new(vec![conn1, conn2, conn3], AppConfig::default());
    let grouped = window.grouped_connections();

    assert_eq!(grouped.len(), 2); // Production, Staging
    assert_eq!(grouped.get("Production").unwrap().len(), 2);
    assert_eq!(grouped.get("Staging").unwrap().len(), 1);
}

#[test]
fn test_connection_editor_dirty_tracking() {
    let conn = Connection::default();
    let mut editor = ConnectionEditor::new(conn, "initial_pass".to_string());
    assert!(!editor.is_dirty);

    editor.update_name("Updated Name");
    assert!(editor.is_dirty);
    assert_eq!(editor.connection.name, "Updated Name");

    editor.update_host("192.168.1.50");
    assert_eq!(editor.connection.host, "192.168.1.50");

    editor.update_port(8080);
    assert_eq!(editor.connection.port, 8080);

    editor.update_password("new_pass_456");
    assert_eq!(editor.password, "new_pass_456");
}

#[test]
fn test_preferences_window_theme_selection() {
    let config = AppConfig::default();
    let mut pref_win = PreferencesWindow::new(config);
    assert_eq!(pref_win.config.theme, "default");

    pref_win.set_theme("dark");
    assert_eq!(pref_win.config.theme, "dark");

    pref_win.set_theme("light");
    assert_eq!(pref_win.config.theme, "light");
}

#[test]
fn test_discovery_dialog_add_services() {
    let mut dialog = DiscoveryDialog::new();
    assert!(dialog.discovered_services.is_empty());

    dialog.add_service(DiscoveredService {
        name: "Remote Mac".to_string(),
        protocol: "vnc".to_string(),
        host: "macbook.local".to_string(),
        port: 5900,
    });

    assert_eq!(dialog.discovered_services.len(), 1);
    assert_eq!(dialog.discovered_services[0].name, "Remote Mac");
    assert_eq!(dialog.discovered_services[0].protocol, "vnc");
    assert_eq!(dialog.discovered_services[0].port, 5900);
}

#[test]
fn test_active_session_tracking() {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let is_running_1 = Arc::new(AtomicBool::new(true));
    let is_running_2 = Arc::new(AtomicBool::new(false));

    let mut sessions: HashMap<String, Arc<AtomicBool>> = HashMap::new();
    sessions.insert("conn-1".to_string(), is_running_1.clone());
    sessions.insert("conn-2".to_string(), is_running_2.clone());

    assert!(sessions
        .get("conn-1")
        .map(|s| s.load(Ordering::SeqCst))
        .unwrap_or(false));
    assert!(!sessions
        .get("conn-2")
        .map(|s| s.load(Ordering::SeqCst))
        .unwrap_or(false));
    assert!(!sessions
        .get("conn-3")
        .map(|s| s.load(Ordering::SeqCst))
        .unwrap_or(false));

    // Simulate session termination
    is_running_1.store(false, Ordering::SeqCst);
    assert!(!sessions
        .get("conn-1")
        .map(|s| s.load(Ordering::SeqCst))
        .unwrap_or(false));
}
