use beautiful_goodall::models::{AppConfig, Connection, Protocol, VncScaling};
use beautiful_goodall::ui::{ConnectionEditor, MainWindow};

#[test]
fn test_stress_search_query_whitespace_handling() {
    let mut conn1 = Connection::default();
    conn1.name = "Web Server".to_string();
    conn1.protocol = Protocol::Rdp;

    let mut conn2 = Connection::default();
    conn2.name = "Database Server".to_string();
    conn2.protocol = Protocol::Vnc;

    let mut window = MainWindow::new(vec![conn1, conn2], AppConfig::default());

    // Exact search
    window.set_search_filter("rdp");
    assert_eq!(window.filtered_connections().len(), 1);

    // Search query with leading/trailing spaces vs behavior
    // MainWindow::filtered_connections checks contains(query)
    // Note: GTK UI setup_filtering calls .trim() on query before searching
    window.set_search_filter("  rdp  ");
    // Untrimmed in filtered_connections: "  rdp  " won't match "rdp"
    assert_eq!(window.filtered_connections().len(), 0);

    // If query is trimmed before setting or in setup_filtering:
    window.set_search_filter("  rdp  ".trim());
    assert_eq!(window.filtered_connections().len(), 1);
}

#[test]
fn test_stress_search_special_characters_and_unicode() {
    let mut conn1 = Connection::default();
    conn1.name = "Prod-Server [01] 🚀".to_string();
    conn1.host = "192.168.1.100".to_string();
    conn1.group = "Web/App & DB".to_string();

    let mut conn2 = Connection::default();
    conn2.name = "Test-Server (Dev) 🧪".to_string();
    conn2.host = "10.0.0.1".to_string();
    conn2.group = "Test/Dev".to_string();

    let mut window = MainWindow::new(vec![conn1, conn2], AppConfig::default());

    // Search by Unicode emoji
    window.set_search_filter("🚀");
    assert_eq!(window.filtered_connections().len(), 1);
    assert_eq!(window.filtered_connections()[0].name, "Prod-Server [01] 🚀");

    // Search by special characters: brackets "[01]"
    window.set_search_filter("[01]");
    assert_eq!(window.filtered_connections().len(), 1);

    // Search by ampersand "&"
    window.set_search_filter("&");
    assert_eq!(window.filtered_connections().len(), 1);

    // Search by slash "/"
    window.set_search_filter("/");
    assert_eq!(window.filtered_connections().len(), 2);
}

#[test]
fn test_stress_connection_editor_boundary_mutations() {
    let conn = Connection::default();
    let mut editor = ConnectionEditor::new(conn, "secret_123".to_string());
    assert!(!editor.is_dirty);

    // Update fields and verify dirty tracking
    editor.update_name("New Edge Name 🎯");
    assert!(editor.is_dirty);
    assert_eq!(editor.connection.name, "New Edge Name 🎯");

    editor.update_host("fe80::1ff:fe23:4567:890a"); // IPv6 host
    assert_eq!(editor.connection.host, "fe80::1ff:fe23:4567:890a");

    // Port upper boundary (max u16)
    editor.update_port(65535);
    assert_eq!(editor.connection.port, 65535);
    assert!(editor.validate().is_ok());

    // Port lower boundary invalid (0)
    editor.update_port(0);
    assert!(editor.validate().is_err());
}

#[test]
fn test_stress_large_connection_list_filtering_and_grouping() {
    let mut connections = Vec::new();
    for i in 0..1000 {
        let mut conn = Connection::default();
        conn.id = format!("uuid-{}", i);
        conn.name = format!("Server #{:04}", i);
        conn.host = format!("10.{}.{}.{}", i / 256, (i % 256), 1);
        conn.group = format!("Group-{}", i % 20); // 20 distinct groups
        conn.username = format!("user_{}", i % 5);
        conn.protocol = match i % 3 {
            0 => Protocol::Rdp,
            1 => Protocol::Vnc,
            _ => Protocol::Ssh,
        };
        connections.push(conn);
    }

    let mut window = MainWindow::new(connections, AppConfig::default());

    // Filter 1000 connections
    let start = std::time::Instant::now();
    window.set_search_filter("Server #00");
    let filtered = window.filtered_connections();
    let duration = start.elapsed();

    // 10 matches (#0000 to #0009, #0010.. no wait: #0000..#0009, #0010.. #0099 -> 100 matches)
    assert_eq!(filtered.len(), 100);
    assert!(duration.as_millis() < 50, "Filtering 1,000 connections should take < 50ms (took {:?})", duration);

    // Grouping performance
    let start_grouping = std::time::Instant::now();
    let grouped = window.grouped_connections();
    let group_duration = start_grouping.elapsed();

    assert!(grouped.len() <= 20);
    assert!(group_duration.as_millis() < 50, "Grouping 1,000 connections should take < 50ms (took {:?})", group_duration);
}

#[test]
fn test_stress_appconfig_roundtrip_serde_matrix() {
    let themes = vec!["default", "dark", "light", "custom_theme"];
    let protocols = vec![Protocol::Rdp, Protocol::Vnc, Protocol::Ssh];
    let scalings = vec![VncScaling::OriginalSize, VncScaling::FitToWindow, VncScaling::Stretch];
    let last_ids = vec![None, Some("id-123-abc".to_string())];

    for theme in &themes {
        for proto in &protocols {
            for scaling in &scalings {
                for last_id in &last_ids {
                    let config = AppConfig {
                        theme: theme.to_string(),
                        default_protocol: *proto,
                        auto_connect_last: true,
                        default_vnc_scaling: *scaling,
                        last_connected_id: last_id.clone(),
                    };

                    let json = serde_json::to_string(&config).expect("Serialization must succeed");
                    let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialization must succeed");

                    assert_eq!(deserialized, config);
                }
            }
        }
    }
}
