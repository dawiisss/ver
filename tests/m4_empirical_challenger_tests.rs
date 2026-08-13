use beautiful_goodall::launcher::{
    build_rdp_args, build_ssh_args, build_ssh_args_with_identity, build_terminal_command,
    detect_terminal_emulator, find_binary_in_path, launch_rdp, launch_ssh, TERMINAL_CANDIDATES,
};
use beautiful_goodall::models::{Connection, Protocol};
use beautiful_goodall::network::{
    build_wol_packet, build_wol_packet_bytes, parse_mac_address, send_wol_to,
    DEFAULT_BROADCAST_ADDR, DEFAULT_WOL_PORT,
};
use std::net::UdpSocket;
use std::time::Duration;

#[test]
fn test_process_argument_escaping_rdp() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Rdp;
    conn.host = "rdp server.internal.domain".to_string();
    conn.port = 33890;
    conn.username = "domain\\user name".to_string();
    conn.advanced_settings.clipboard_sharing = true;
    conn.advanced_settings.color_depth = 24;
    conn.advanced_settings.rdp_multimon = true;
    conn.advanced_settings.rdp_fullscreen = true;
    conn.advanced_settings.rdp_audio = true;

    let pass_with_special = "P@ss:w0rd! with spaces & 'quotes'";
    let args = build_rdp_args(&conn, Some(pass_with_special));

    // Verify /v argument contains host and custom port
    assert!(args.contains(&"/v:rdp server.internal.domain:33890".to_string()));
    // Verify /u argument contains verbatim username with spaces/slashes
    assert!(args.contains(&"/u:domain\\user name".to_string()));
    // Verify /p argument contains verbatim password with special characters
    assert!(args.contains(&format!("/p:{}", pass_with_special)));
    // Verify feature flags
    assert!(args.contains(&"/cert:ignore".to_string()));
    assert!(args.contains(&"/dynamic-resolution".to_string()));
    assert!(args.contains(&"+clipboard".to_string()));
    assert!(args.contains(&"/bpp:24".to_string()));
    assert!(args.contains(&"/multimon".to_string()));
    assert!(args.contains(&"/f".to_string()));
    assert!(args.contains(&"/sound".to_string()));

    // Test disabled clipboard flag (-)
    conn.advanced_settings.clipboard_sharing = false;
    let args_no_clip = build_rdp_args(&conn, None);
    assert!(args_no_clip.contains(&"-clipboard".to_string()));
    assert!(!args_no_clip.iter().any(|a| a.starts_with("/p:")));
}

#[test]
fn test_process_argument_escaping_ssh_and_identity() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = " bastion.prod.org ".to_string();
    conn.port = 2222;
    conn.username = " sysadmin user ".to_string();

    let key_path = "/home/user/my secret keys/id_ed25519";
    let args = build_ssh_args_with_identity(&conn, Some(key_path));

    assert_eq!(args[0], "ssh");
    assert_eq!(args[1], "-p");
    assert_eq!(args[2], "2222");
    assert_eq!(args[3], "-i");
    assert_eq!(args[4], key_path);
    assert_eq!(args[5], "sysadmin user@bastion.prod.org");

    // Test standard port 22 omit -p
    conn.port = 22;
    let default_port_args = build_ssh_args(&conn);
    assert_eq!(default_port_args, vec!["ssh", "sysadmin user@bastion.prod.org"]);
}

#[test]
fn test_terminal_emulator_command_construction_and_kgx_escaping() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "server.example.com".to_string();
    conn.port = 22;
    conn.username = "alice".to_string();

    let key_path = "/tmp/key with spaces/id_rsa";

    // Verify terminal detection helper functions
    let _ = detect_terminal_emulator();
    let _ = find_binary_in_path("sh");

    for &term in TERMINAL_CANDIDATES {
        let cmd = build_terminal_command(term, &conn, Some(key_path));
        assert_eq!(cmd.get_program(), term);

        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();

        match term {
            "ptyxis" | "gnome-terminal" => {
                assert_eq!(args[0], "--");
                assert_eq!(args[1], "ssh");
                assert_eq!(args[2], "-i");
                assert_eq!(args[3], key_path);
                assert_eq!(args[4], "alice@server.example.com");
            }
            "kgx" => {
                assert_eq!(args[0], "-e");
                let expected_ssh_str = format!("ssh -i {} alice@server.example.com", key_path);
                assert_eq!(args[1], expected_ssh_str);
            }
            _ => {
                assert_eq!(args[0], "-e");
                assert_eq!(args[1], "ssh");
                assert_eq!(args[2], "-i");
                assert_eq!(args[3], key_path);
                assert_eq!(args[4], "alice@server.example.com");
            }
        }
    }
}

#[test]
fn test_mac_address_parsing_edge_cases() {
    // Valid standard and variation formats
    let valid_cases = vec![
        ("00:11:22:33:44:55", [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ("00-11-22-33-44-55", [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ("0011.2233.4455", [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ("00.11.22.33.44.55", [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ("001122334455", [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ("  aa:bb:cc:dd:ee:ff  ", [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
        ("AA-BB-CC-DD-EE-FF\r\n", [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
        ("00:aB:cD:eF:12:34", [0x00, 0xAB, 0xCD, 0xEF, 0x12, 0x34]),
        ("00 : 11 : 22 : 33 : 44 : 55", [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
    ];

    for (input, expected) in valid_cases {
        let mac = parse_mac_address(input)
            .unwrap_or_else(|e| panic!("Failed to parse valid MAC '{}': {}", input, e));
        assert_eq!(mac, expected, "Parsed MAC mismatch for input: '{}'", input);
    }

    // Invalid MAC inputs
    let invalid_cases = vec![
        "",
        "   ",
        "00:11:22:33:44",                  // 10 hex digits
        "00:11:22:33:44:55:66",            // 14 hex digits
        "00:11:22:33:44:GG",               // Invalid hex char G
        "0x001122334455",                  // Prefix '0x' adds 2 non-delimiters
        "00:11:22:33:44:5!",               // Invalid punctuation
        "00:11:22:33:44:55\u{0660}",       // Non-ASCII unicode digit
    ];

    for input in invalid_cases {
        assert!(
            parse_mac_address(input).is_err(),
            "Invalid MAC input '{}' should return error",
            input
        );
    }
}

#[test]
fn test_wol_magic_packet_generation_and_payload_integrity() {
    let mac_str = "12:34:56:78:9A:BC";
    let mac_bytes = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];

    let packet = build_wol_packet(mac_str).expect("Building packet from &str must succeed");
    assert_eq!(packet.len(), 102);
    assert_eq!(&packet[0..6], &[0xFF; 6]);

    for i in 0..16 {
        let start = 6 + i * 6;
        assert_eq!(&packet[start..start + 6], &mac_bytes);
    }

    // Verify fixed array builder parity
    let array_packet = build_wol_packet_bytes(&mac_bytes);
    assert_eq!(array_packet.len(), 102);
    assert_eq!(&array_packet[..], packet.as_slice());
}

#[test]
fn test_udp_socket_broadcast_transmit_loopback() {
    assert_eq!(DEFAULT_BROADCAST_ADDR, "255.255.255.255");
    assert_eq!(DEFAULT_WOL_PORT, 9);

    let rx_socket = UdpSocket::bind("127.0.0.1:0").expect("Binding UDP receiver socket must succeed");
    let rx_addr = rx_socket.local_addr().expect("Getting local socket address must succeed");
    rx_socket
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .expect("Setting timeout must succeed");

    let dest = format!("127.0.0.1:{}", rx_addr.port());
    let mac_str = "DE:AD:BE:EF:CA:FE";
    let expected_mac = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];

    let send_res = send_wol_to(mac_str, &dest);
    assert!(send_res.is_ok(), "send_wol_to must return Ok for loopback address");

    let mut buf = [0u8; 256];
    let (bytes_received, src_addr) = rx_socket
        .recv_from(&mut buf)
        .expect("Must receive UDP WoL magic packet on bound receiver");

    assert_eq!(bytes_received, 102, "UDP payload length must be 102 bytes");
    assert_eq!(&buf[0..6], &[0xFF; 6], "Magic packet header must be 6x 0xFF");

    for i in 0..16 {
        let start = 6 + i * 6;
        assert_eq!(
            &buf[start..start + 6],
            &expected_mac,
            "MAC repetition {} mismatch in received packet",
            i
        );
    }

    assert_ne!(src_addr.port(), 0);
}

#[test]
fn test_udp_socket_invalid_address_error_handling() {
    let res = send_wol_to("00:11:22:33:44:55", "999.999.999.999:9999");
    assert!(res.is_err(), "send_wol_to with invalid IP must return Err");
}

#[test]
fn test_launch_rdp_ssh_empty_host_guard() {
    let mut conn = Connection::default();
    conn.host = "   \t\n  ".to_string();

    assert!(launch_rdp(&conn, None).is_err(), "Empty host must fail RDP launch");
    assert!(launch_ssh(&conn).is_err(), "Empty host must fail SSH launch");
}
