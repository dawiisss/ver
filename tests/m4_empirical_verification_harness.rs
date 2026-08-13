use beautiful_goodall::launcher::{
    build_rdp_args, build_ssh_args, build_ssh_args_with_identity, build_terminal_command,
    detect_terminal_emulator, find_binary_in_path, TERMINAL_CANDIDATES,
};
use beautiful_goodall::models::{Connection, Protocol};
use beautiful_goodall::network::{
    build_wol_packet, build_wol_packet_bytes, parse_mac_address, send_wol_to, DEFAULT_BROADCAST_ADDR,
    DEFAULT_WOL_PORT,
};
use std::net::UdpSocket;
use std::time::Duration;

#[test]
fn test_wol_packet_binary_format_exhaustive() {
    assert_eq!(DEFAULT_BROADCAST_ADDR, "255.255.255.255");
    assert_eq!(DEFAULT_WOL_PORT, 9);

    let mac_formats = vec![
        "00:11:22:33:44:55",
        "00-11-22-33-44-55",
        "0011.2233.4455",
        "00.11.22.33.44.55",
        "001122334455",
        "  00:11:22:33:44:55  ",
        "00:11:22:33:44:55\n",
        "00:11:22:33:44:55\r\n",
        "00:11:22:33:44:55\t",
    ];

    let expected_mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];

    for mac_str in mac_formats {
        let parsed = parse_mac_address(mac_str).expect("Parsing valid MAC format must succeed");
        assert_eq!(parsed, expected_mac);

        let packet = build_wol_packet(mac_str).expect("Building WoL packet must succeed");
        assert_eq!(packet.len(), 102, "WoL packet length must be exactly 102 bytes");

        // Verify prefix: 6 bytes of 0xFF
        assert_eq!(&packet[0..6], &[0xFF; 6], "Prefix must be 6 bytes of 0xFF");

        // Verify payload: 16 iterations of 6-byte MAC
        for i in 0..16 {
            let start = 6 + i * 6;
            let end = start + 6;
            assert_eq!(
                &packet[start..end],
                &expected_mac,
                "Iteration {} of MAC payload in WoL packet mismatch",
                i
            );
        }
    }

    // Verify build_wol_packet_bytes
    let array_packet = build_wol_packet_bytes(&expected_mac);
    assert_eq!(array_packet.len(), 102);
    assert_eq!(&array_packet[0..6], &[0xFF; 6]);
    for i in 0..16 {
        let start = 6 + i * 6;
        assert_eq!(&array_packet[start..start + 6], &expected_mac);
    }
}

#[test]
fn test_wol_send_to_loopback_socket_binding_and_payload() {
    let rx_socket = UdpSocket::bind("127.0.0.1:0").expect("Binding receiver socket must succeed");
    let rx_addr = rx_socket.local_addr().expect("Getting local socket address must succeed");
    rx_socket
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .expect("Setting timeout must succeed");

    let dest = format!("127.0.0.1:{}", rx_addr.port());
    let mac = "AA:BB:CC:DD:EE:FF";
    let expected_mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];

    let result = send_wol_to(mac, &dest);
    assert!(result.is_ok(), "send_wol_to to loopback destination should return Ok");

    let mut buf = [0u8; 512];
    let (amt, src) = rx_socket.recv_from(&mut buf).expect("Must receive UDP WoL packet on loopback");

    assert_eq!(amt, 102, "Received WoL packet payload must be exactly 102 bytes");
    assert_eq!(&buf[0..6], &[0xFF; 6], "First 6 bytes must be 0xFF");

    for i in 0..16 {
        let start = 6 + i * 6;
        assert_eq!(
            &buf[start..start + 6],
            &expected_mac,
            "Received MAC byte repetition {} mismatch",
            i
        );
    }

    assert_ne!(src.port(), 0, "Source socket port must be bound");
}

#[test]
fn test_terminal_emulator_path_resolution_and_stdio_flags() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "test.server.org".to_string();
    conn.port = 22;
    conn.username = "alice".to_string();

    let default_ssh_args = build_ssh_args(&conn);
    assert_eq!(default_ssh_args, vec!["ssh", "alice@test.server.org"]);

    let id_ssh_args = build_ssh_args_with_identity(&conn, Some("/tmp/id_rsa"));
    assert_eq!(id_ssh_args, vec!["ssh", "-i", "/tmp/id_rsa", "alice@test.server.org"]);

    let mut rdp_conn = Connection::default();
    rdp_conn.protocol = Protocol::Rdp;
    rdp_conn.host = "192.168.1.100".to_string();
    let rdp_args = build_rdp_args(&rdp_conn, Some("secret"));
    assert!(rdp_args.contains(&"/v:192.168.1.100:3389".to_string()));
    assert!(rdp_args.contains(&"/p:secret".to_string()));

    for &term in TERMINAL_CANDIDATES {
        let cmd = build_terminal_command(term, &conn, None);

        // Verify program name matches candidate
        assert_eq!(cmd.get_program(), term);

        // Inspect command arguments
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        match term {
            "ptyxis" | "gnome-terminal" => {
                assert_eq!(args[0], "--");
                assert_eq!(args[1], "ssh");
                assert_eq!(args[2], "alice@test.server.org");
            }
            "kgx" => {
                assert_eq!(args[0], "-e");
                assert_eq!(args[1], "ssh alice@test.server.org");
            }
            _ => {
                assert_eq!(args[0], "-e");
                assert_eq!(args[1], "ssh");
                assert_eq!(args[2], "alice@test.server.org");
            }
        }
    }

    // Verify find_binary_in_path finds sh/bash
    let sh_path = find_binary_in_path("sh");
    assert!(sh_path.is_some(), "System binary 'sh' should be found in PATH");
    assert!(sh_path.unwrap().is_file());

    // Verify detect_terminal_emulator returns Some or None cleanly without panicking
    let detected = detect_terminal_emulator();
    if let Some((name, path)) = detected {
        assert!(TERMINAL_CANDIDATES.contains(&name));
        assert!(path.is_file());
    }
}
