use beautiful_goodall::launcher::{build_rdp_args, build_ssh_args};
use beautiful_goodall::models::{Connection, Protocol};
use beautiful_goodall::network::build_wol_packet;

#[test]
fn test_build_rdp_args_standard() {
    let mut conn = Connection::default();
    conn.host = "rdp.example.com".to_string();
    conn.port = 3389;
    conn.username = "administrator".to_string();
    conn.advanced_settings.clipboard_sharing = true;
    conn.advanced_settings.color_depth = 32;
    conn.advanced_settings.rdp_multimon = true;
    conn.advanced_settings.rdp_fullscreen = true;
    conn.advanced_settings.rdp_audio = true;

    let args = build_rdp_args(&conn, Some("MySecretPass"));

    assert!(args.contains(&"/v:rdp.example.com:3389".to_string()));
    assert!(args.contains(&"/u:administrator".to_string()));
    assert!(args.contains(&"/p:MySecretPass".to_string()));
    assert!(args.contains(&"/cert:ignore".to_string()));
    assert!(args.contains(&"/dynamic-resolution".to_string()));
    assert!(args.contains(&"+clipboard".to_string()));
    assert!(args.contains(&"/bpp:32".to_string()));
    assert!(args.contains(&"/multimon".to_string()));
    assert!(args.contains(&"/f".to_string()));
    assert!(args.contains(&"/sound".to_string()));
}

#[test]
fn test_build_rdp_args_disabled_clipboard_no_password() {
    let mut conn = Connection::default();
    conn.host = "10.0.0.15".to_string();
    conn.port = 3389;
    conn.username = "user".to_string();
    conn.advanced_settings.clipboard_sharing = false;

    let args = build_rdp_args(&conn, None);

    assert!(args.contains(&"/v:10.0.0.15:3389".to_string()));
    assert!(args.contains(&"-clipboard".to_string()));
    assert!(!args.iter().any(|a| a.starts_with("/p:")));
}

#[test]
fn test_build_ssh_args_custom_port() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "bastion.example.com".to_string();
    conn.port = 2222;
    conn.username = "devops".to_string();

    let args = build_ssh_args(&conn);

    assert_eq!(args[0], "ssh");
    assert_eq!(args[1], "-p");
    assert_eq!(args[2], "2222");
    assert_eq!(args[3], "devops@bastion.example.com");
}

#[test]
fn test_build_ssh_args_default_port_22() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "shell.example.com".to_string();
    conn.port = 22;
    conn.username = "root".to_string();

    let args = build_ssh_args(&conn);

    assert_eq!(args, vec!["ssh", "root@shell.example.com"]);
}

#[test]
fn test_wol_magic_packet_generation() {
    let mac = "00:11:22:33:44:55";
    let packet = build_wol_packet(mac).expect("Failed to build WoL packet");

    // WoL packet length = 6 (FF) + 16 * 6 (MAC) = 102 bytes
    assert_eq!(packet.len(), 102);

    // First 6 bytes must be 0xFF
    for &b in &packet[0..6] {
        assert_eq!(b, 0xFF);
    }

    // Expected MAC bytes: 0x00, 0x11, 0x22, 0x33, 0x44, 0x55
    let expected_mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
    for i in 0..16 {
        let start = 6 + i * 6;
        let end = start + 6;
        assert_eq!(&packet[start..end], &expected_mac);
    }
}

#[test]
fn test_wol_invalid_mac_length_fails() {
    let short_mac = "00:11:22:33:44";
    assert!(build_wol_packet(short_mac).is_err());
}
