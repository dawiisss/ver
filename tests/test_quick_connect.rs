use ver::models::Protocol;
use ver::ui::quick_connect::parse_quick_connect;

#[test]
fn test_parse_schemed_uris() {
    // SSH URI
    let conn = parse_quick_connect("ssh://admin@192.168.1.50:2222", Protocol::Rdp).unwrap();
    assert_eq!(conn.protocol, Protocol::Ssh);
    assert_eq!(conn.username, "admin");
    assert_eq!(conn.host, "192.168.1.50");
    assert_eq!(conn.port, 2222);

    // RDP URI
    let conn = parse_quick_connect("rdp://corp_admin@remote.company.com:3399", Protocol::Ssh).unwrap();
    assert_eq!(conn.protocol, Protocol::Rdp);
    assert_eq!(conn.username, "corp_admin");
    assert_eq!(conn.host, "remote.company.com");
    assert_eq!(conn.port, 3399);

    // VNC URI
    let conn = parse_quick_connect("vnc://10.0.0.12:5901", Protocol::Rdp).unwrap();
    assert_eq!(conn.protocol, Protocol::Vnc);
    assert_eq!(conn.host, "10.0.0.12");
    assert_eq!(conn.port, 5901);

    // SPICE URI
    let conn = parse_quick_connect("spice://localhost:5900", Protocol::Rdp).unwrap();
    assert_eq!(conn.protocol, Protocol::Spice);
    assert_eq!(conn.host, "localhost");
    assert_eq!(conn.port, 5900);
}

#[test]
fn test_parse_shorthand_syntaxes() {
    // user@host:port with default SSH
    let conn = parse_quick_connect("root@192.168.1.1:2200", Protocol::Ssh).unwrap();
    assert_eq!(conn.protocol, Protocol::Ssh);
    assert_eq!(conn.username, "root");
    assert_eq!(conn.host, "192.168.1.1");
    assert_eq!(conn.port, 2200);

    // user@host (default port for protocol)
    let conn = parse_quick_connect("developer@mybox.internal", Protocol::Ssh).unwrap();
    assert_eq!(conn.protocol, Protocol::Ssh);
    assert_eq!(conn.username, "developer");
    assert_eq!(conn.host, "mybox.internal");
    assert_eq!(conn.port, 22);

    // plain host:port
    let conn = parse_quick_connect("192.168.10.5:3389", Protocol::Rdp).unwrap();
    assert_eq!(conn.protocol, Protocol::Rdp);
    assert_eq!(conn.host, "192.168.10.5");
    assert_eq!(conn.port, 3389);
    assert_eq!(conn.username, "");

    // plain host IP
    let conn = parse_quick_connect("10.0.0.99", Protocol::Rdp).unwrap();
    assert_eq!(conn.protocol, Protocol::Rdp);
    assert_eq!(conn.host, "10.0.0.99");
    assert_eq!(conn.port, 3389);
}

#[test]
fn test_parse_ipv6_addresses() {
    let conn = parse_quick_connect("ssh://user@[2001:db8::1]:2222", Protocol::Rdp).unwrap();
    assert_eq!(conn.protocol, Protocol::Ssh);
    assert_eq!(conn.username, "user");
    assert_eq!(conn.host, "2001:db8::1");
    assert_eq!(conn.port, 2222);
}

#[test]
fn test_parse_invalid_inputs() {
    assert!(parse_quick_connect("", Protocol::Rdp).is_err());
    assert!(parse_quick_connect("   ", Protocol::Rdp).is_err());
    assert!(parse_quick_connect("invalid://host", Protocol::Rdp).is_err());
    assert!(parse_quick_connect("host:invalid_port", Protocol::Rdp).is_err());
}
