use beautiful_goodall::models::{Connection, Protocol, VncColorLevel, VncEncodingOption};
use beautiful_goodall::launcher::build_vnc_args;

#[test]
fn test_print_args() {
    let mut conn = Connection::new_with_protocol(Protocol::Vnc);
    conn.name = "Test VNC".to_string();
    conn.host = "192.168.1.100".to_string();
    conn.port = 5900;
    conn.advanced_settings.vnc_shared = true;
    conn.advanced_settings.vnc_encoding = VncEncodingOption::Tight;
    conn.advanced_settings.vnc_color_level = VncColorLevel::Full;

    let args = build_vnc_args(&conn);
    assert!(args.contains(&"-Shared".to_string()));
    assert!(args.contains(&"-PreferredEncoding=Tight".to_string()));
    assert!(args.contains(&"192.168.1.100:5900".to_string()));
}

