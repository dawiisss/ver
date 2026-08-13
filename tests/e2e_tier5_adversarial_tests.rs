// Tier 5 White-Box Adversarial Coverage Hardening Test Suite
// Targets: src/models.rs, src/storage.rs, src/secrets.rs, src/network.rs, src/launcher.rs

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::net::UdpSocket;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

use beautiful_goodall::launcher::{
    build_rdp_args, build_ssh_args, build_ssh_args_with_identity, build_terminal_command,
    detect_terminal_emulator, launch_ssh, TERMINAL_CANDIDATES,
};
use beautiful_goodall::models::{
    AdvancedSettings, AppConfig, Connection, Protocol, VncScaling,
};
use beautiful_goodall::network::{
    build_wol_packet, build_wol_packet_bytes, parse_mac_address, send_wol, send_wol_to,
};
use beautiful_goodall::secrets::{
    delete_password, delete_password_sync, get_password, get_password_sync, set_password,
    set_password_sync,
};
use beautiful_goodall::storage::{
    load_config_from_path, load_connections_from_path, save_connections_to_path,
};

// ============================================================================
// Group 1: Storage, JSON Recovery, Path Traversal & Model Sanitization
// ============================================================================

#[test]
fn test_tier5_corrupted_json_recovery_truncation() {
    let dir = tempdir().expect("Failed to create temp dir");
    let truncated_path = dir.path().join("truncated.json");

    // Write incomplete/truncated JSON string
    fs::write(&truncated_path, r#"[{"id": "123", "name": "Incomplete"#).unwrap();

    let loaded = load_connections_from_path(&truncated_path)
        .expect("Truncated JSON load must recover gracefully");
    assert!(loaded.is_empty(), "Corrupted truncated file must return empty connection vector");

    // Verify backup file created
    let entries = fs::read_dir(dir.path()).unwrap();
    let backup_exists = entries.filter_map(|e| e.ok()).any(|e| {
        e.file_name().to_string_lossy().contains("truncated.json.corrupt.")
    });
    assert!(backup_exists, "Corrupt backup file must be created on truncation error");
}

#[test]
fn test_tier5_malformed_json_type_mismatches() {
    let dir = tempdir().expect("Failed to create temp dir");
    let malformed_path = dir.path().join("type_mismatch.json");

    // JSON object instead of array, string for port, number for protocol
    let invalid_json = r#"{
        "id": 12345,
        "name": true,
        "protocol": 999,
        "port": "invalid_port_string"
    }"#;
    fs::write(&malformed_path, invalid_json).unwrap();

    let loaded = load_connections_from_path(&malformed_path)
        .expect("Type mismatched JSON must recover gracefully");
    assert!(loaded.is_empty(), "Type mismatched JSON must return empty vector");

    let entries = fs::read_dir(dir.path()).unwrap();
    let backup_exists = entries.filter_map(|e| e.ok()).any(|e| {
        e.file_name().to_string_lossy().contains("type_mismatch.json.corrupt.")
    });
    assert!(backup_exists, "Corrupt backup file must be created on type mismatch");
}

#[test]
fn test_tier5_path_traversal_in_connection_ids() {
    let traversal_ids = vec![
        "../../../etc/passwd",
        "..\\..\\Windows\\System32\\cmd.exe",
        "/absolute/path/traversal",
        "id/with/slashes",
        "id\\with\\backslashes",
        "normal..id..with..dots",
    ];

    for bad_id in traversal_ids {
        let mut conn = Connection {
            id: bad_id.to_string(),
            ..Default::default()
        };

        let modified = conn.sanitize();
        assert!(modified, "Sanitize must return true when correcting path traversal ID: '{}'", bad_id);
        assert_ne!(conn.id, bad_id, "ID must be changed from invalid traversal string");
        assert!(Uuid::parse_str(&conn.id).is_ok(), "Sanitized ID must be a valid UUID v4");
    }
}

#[test]
fn test_tier5_path_traversal_and_control_chars_in_group_and_name() {
    let mut conn = Connection {
        id: Uuid::new_v4().to_string(),
        name: "   \t\r\n   ".to_string(),
        group: "".to_string(),
        ..Default::default()
    };

    assert!(conn.sanitize());
    assert_eq!(conn.name, "New Connection");
    assert_eq!(conn.group, "Default");

    // Special unicode control chars & path traversal strings in name/group
    let mut conn2 = Connection {
        id: Uuid::new_v4().to_string(),
        name: "../../etc/shadow".to_string(),
        group: "Admin/Root/..".to_string(),
        ..Default::default()
    };

    // Name and group with non-empty content should be preserved without panicking
    let _ = conn2.sanitize();
    assert_eq!(conn2.name, "../../etc/shadow");
    assert_eq!(conn2.group, "Admin/Root/..");
}

#[test]
fn test_tier5_storage_atomic_save_directory_creation() {
    let dir = tempdir().expect("Failed to create temp dir");
    let deeply_nested = dir.path().join("level1").join("level2").join("level3").join("connections.json");

    let mut conn = Connection::default();
    conn.name = "Deep Storage Test".to_string();

    save_connections_to_path(&deeply_nested, &[conn.clone()])
        .expect("Atomic save to deeply nested non-existent directory structure must succeed");

    assert!(deeply_nested.exists());
    let loaded = load_connections_from_path(&deeply_nested)
        .expect("Loading from deeply nested path must succeed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "Deep Storage Test");
}

#[test]
fn test_tier5_appconfig_corrupt_recovery() {
    let dir = tempdir().expect("Failed to create temp dir");
    let config_path = dir.path().join("corrupt_config.json");

    fs::write(&config_path, "invalid json config").unwrap();

    let loaded_config = load_config_from_path(&config_path)
        .expect("Corrupt config load must return default AppConfig");
    assert_eq!(loaded_config, AppConfig::default());

    let entries = fs::read_dir(dir.path()).unwrap();
    let backup_exists = entries.filter_map(|e| e.ok()).any(|e| {
        e.file_name().to_string_lossy().contains("corrupt_config.json.corrupt.")
    });
    assert!(backup_exists, "Corrupt config backup file must be created");
}

// ============================================================================
// Group 2: Secret Service D-Bus Fallback & Null/Adversarial Inputs
// ============================================================================

#[tokio::test]
async fn test_tier5_secret_service_fallback_missing_daemon() {
    let test_id = "test-fallback-uuid-9999";
    let test_pass = "secret_pass_1234";

    // When Secret Service D-Bus daemon is unavailable or errors,
    // secrets functions must not panic and must degrade gracefully.
    let get_res = get_password(test_id).await;
    assert!(get_res.is_ok(), "get_password must return Ok even if Secret Service is absent");

    let set_res = set_password(test_id, test_pass).await;
    assert!(set_res.is_ok(), "set_password must return Ok even if Secret Service is absent");

    let del_res = delete_password(test_id).await;
    assert!(del_res.is_ok(), "delete_password must return Ok even if Secret Service is absent");
}

#[tokio::test]
async fn test_tier5_secret_service_empty_and_null_inputs() {
    // Empty connection ID and empty passwords
    assert!(get_password("").await.is_ok());
    assert!(set_password("", "").await.is_ok());
    assert!(delete_password("").await.is_ok());

    // Password containing special characters, unicode, and newline chars
    let weird_pass = "P@ssw0rd!\r\n🦀unicode_string";
    assert!(set_password("weird-id-123", weird_pass).await.is_ok());
    assert!(delete_password("weird-id-123").await.is_ok());
}

#[tokio::test]
async fn test_tier5_secret_service_path_traversal_ids() {
    let traversal_ids = vec![
        "../../../etc/shadow",
        "..\\..\\Windows\\Secret",
        "id_with_space and quotes '\"$",
    ];

    for bad_id in traversal_ids {
        assert!(get_password(bad_id).await.is_ok());
        assert!(set_password(bad_id, "pass").await.is_ok());
        assert!(delete_password(bad_id).await.is_ok());
    }

    // D-Bus specification forbids null bytes (\0) in string attributes.
    // Verify get_password gracefully returns an Err rather than panicking.
    let null_byte_id = "id_with_\0_null_byte";
    let null_res = get_password(null_byte_id).await;
    assert!(null_res.is_err(), "Null byte in keyring string attribute must return an error from D-Bus validation");
}

#[test]
fn test_tier5_secret_service_sync_wrappers_multithread_tokio() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Call sync functions within a multi-threaded Tokio runtime via block_in_place
        let id = "sync-multithread-test-id";
        assert!(get_password_sync(id).is_ok());
        assert!(set_password_sync(id, "sync_pass").is_ok());
        assert!(delete_password_sync(id).is_ok());
    });
}

// ============================================================================
// Group 3: Wake-on-LAN MAC Parsing & Packet Generation
// ============================================================================

#[test]
fn test_tier5_wol_mac_parsing_non_standard_delimiters() {
    // Standard formats
    assert_eq!(
        parse_mac_address("00:11:22:33:44:55").unwrap(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );
    assert_eq!(
        parse_mac_address("00-11-22-33-44-55").unwrap(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );
    assert_eq!(
        parse_mac_address("0011.2233.4455").unwrap(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );
    assert_eq!(
        parse_mac_address("00.11.22.33.44.55").unwrap(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );
    assert_eq!(
        parse_mac_address("001122334455").unwrap(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );

    // Whitespace + mixed allowed separators
    assert_eq!(
        parse_mac_address(" \t 00 : 11 - 22 . 33 44 55 \n ").unwrap(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );

    // Unsupported non-standard delimiters (should fail parse_mac_address)
    assert!(parse_mac_address("00_11_22_33_44_55").is_err());
    assert!(parse_mac_address("00/11/22/33/44/55").is_err());
    assert!(parse_mac_address("00#11#22#33#44#55").is_err());
}

#[test]
fn test_tier5_wol_mac_parsing_invalid_lengths() {
    let invalid_length_macs = vec![
        "",
        "0",
        "00:11:22:33:44",         // 10 hex digits
        "00:11:22:33:44:5",        // 11 hex digits
        "00:11:22:33:44:55:6",     // 13 hex digits
        "00:11:22:33:44:55:66",    // 14 hex digits
        "00112233445566778899aabb", // 24 hex digits
    ];

    for bad_mac in invalid_length_macs {
        let res = parse_mac_address(bad_mac);
        assert!(
            res.is_err(),
            "MAC address with invalid length '{}' must fail parsing",
            bad_mac
        );
    }
}

#[test]
fn test_tier5_wol_mac_parsing_invalid_hex_characters() {
    let invalid_hex_macs = vec![
        "00:11:22:33:44:ZZ",
        "00:11:22:33:44:5G",
        "GG:HH:II:JJ:KK:LL",
        "00:11:22:33:44:??",
    ];

    for bad_mac in invalid_hex_macs {
        let res = parse_mac_address(bad_mac);
        assert!(
            res.is_err(),
            "MAC address with non-hex characters '{}' must fail parsing",
            bad_mac
        );
    }
}

#[test]
fn test_tier5_wol_magic_packet_structural_verification() {
    let mac_str = "AA:BB:CC:DD:EE:FF";
    let mac_bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];

    let packet = build_wol_packet(mac_str).expect("Packet build must succeed for valid MAC");
    assert_eq!(packet.len(), 102, "WoL packet length must be exactly 102 bytes");

    // Header: 6 bytes of 0xFF
    assert_eq!(&packet[0..6], &[0xFF; 6]);

    // Body: 16 repetitions of 6-byte MAC address
    for i in 0..16 {
        let offset = 6 + i * 6;
        assert_eq!(&packet[offset..offset + 6], &mac_bytes);
    }

    // Direct byte array overload
    let array_packet = build_wol_packet_bytes(&mac_bytes);
    assert_eq!(array_packet.len(), 102);
    assert_eq!(&array_packet[..], packet.as_slice());
}

#[test]
fn test_tier5_wol_udp_send_to_loopback_target() {
    let rx_socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP rx socket");
    let port = rx_socket.local_addr().unwrap().port();
    rx_socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();

    let target = format!("127.0.0.1:{}", port);
    let mac = "12:34:56:78:90:AB";

    let send_res = send_wol_to(mac, &target);
    assert!(send_res.is_ok(), "Sending WoL to loopback target must succeed");

    let mut buf = [0u8; 128];
    let (received_len, _src) = rx_socket.recv_from(&mut buf).expect("Must receive UDP WoL packet");
    assert_eq!(received_len, 102);
    assert_eq!(&buf[0..6], &[0xFF; 6]);
    assert_eq!(&buf[6..12], &[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB]);

    // Convenient wrapper helper
    let _ = send_wol("00:11:22:33:44:55");
}

// ============================================================================
// Group 4: Launcher Terminal Emulator Resolution Fallback & Argument Safety
// ============================================================================

#[test]
fn test_tier5_terminal_resolution_fallback_custom_path() {
    let dir = tempdir().expect("Failed to create temp dir");
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create a dummy executable for `alacritty` only in custom bin_dir
    let dummy_alacritty = bin_dir.join("alacritty");
    {
        let mut f = File::create(&dummy_alacritty).unwrap();
        writeln!(f, "#!/bin/sh\nexit 0").unwrap();
    }
    let mut perms = fs::metadata(&dummy_alacritty).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dummy_alacritty, perms).unwrap();

    // Save original PATH
    let original_path = env::var_os("PATH");

    // Set PATH to custom bin_dir only
    env::set_var("PATH", &bin_dir);

    let detected = detect_terminal_emulator();
    assert!(detected.is_some(), "Should detect alacritty in custom PATH");
    let (term_name, term_path) = detected.unwrap();
    assert_eq!(term_name, "alacritty");
    assert_eq!(term_path, dummy_alacritty);

    // Now test fallback to xterm when only xterm exists
    fs::remove_file(&dummy_alacritty).unwrap();
    let dummy_xterm = bin_dir.join("xterm");
    {
        let mut f = File::create(&dummy_xterm).unwrap();
        writeln!(f, "#!/bin/sh\nexit 0").unwrap();
    }
    let mut perms = fs::metadata(&dummy_xterm).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dummy_xterm, perms).unwrap();

    let detected_xterm = detect_terminal_emulator();
    assert!(detected_xterm.is_some(), "Should fallback to xterm when only xterm is available");
    let (term_name_x, term_path_x) = detected_xterm.unwrap();
    assert_eq!(term_name_x, "xterm");
    assert_eq!(term_path_x, dummy_xterm);

    // Restore PATH
    if let Some(p) = original_path {
        env::set_var("PATH", p);
    } else {
        env::remove_var("PATH");
    }
}

#[test]
fn test_tier5_launch_ssh_failure_when_no_terminal_available() {
    let dir = tempdir().expect("Failed to create temp dir");
    let empty_bin_dir = dir.path().join("empty_bin");
    fs::create_dir_all(&empty_bin_dir).unwrap();

    let original_path = env::var_os("PATH");
    env::set_var("PATH", &empty_bin_dir);

    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "10.0.0.1".to_string();

    let res = launch_ssh(&conn);
    assert!(
        res.is_err(),
        "launch_ssh must fail when no terminal emulator binary exists on PATH"
    );
    assert!(res.unwrap_err().contains("No supported terminal emulator found"));

    if let Some(p) = original_path {
        env::set_var("PATH", p);
    } else {
        env::remove_var("PATH");
    }
}

#[test]
fn test_tier5_rdp_args_adversarial_configurations() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Rdp;
    conn.host = "  192.168.1.50  ".to_string();
    conn.port = 3389;
    conn.username = "  admin_user  ".to_string();
    conn.advanced_settings = AdvancedSettings {
        color_depth: 24,
        rdp_multimon: true,
        rdp_fullscreen: true,
        rdp_audio: true,
        clipboard_sharing: false,
        vnc_viewonly: false,
        vnc_shared: false,
        vnc_scaling: VncScaling::OriginalSize,
    };

    let pass = Some("p@ssw0rd with spaces & 'quotes' \"double\"");
    let args = build_rdp_args(&conn, pass);

    assert!(args.contains(&"/v:  192.168.1.50  :3389".to_string()));
    assert!(args.contains(&"/u:admin_user".to_string()));
    assert!(args.contains(&format!("/p:{}", pass.unwrap())));
    assert!(args.contains(&"-clipboard".to_string()));
    assert!(args.contains(&"/bpp:24".to_string()));
    assert!(args.contains(&"/multimon".to_string()));
    assert!(args.contains(&"/f".to_string()));
    assert!(args.contains(&"/sound".to_string()));

    // Test default ssh args wrapper for coverage
    let ssh_conn = Connection {
        protocol: Protocol::Ssh,
        host: "host.domain".to_string(),
        port: 2222,
        ..Default::default()
    };
    let ssh_args = build_ssh_args(&ssh_conn);
    assert_eq!(ssh_args, vec!["ssh", "-p", "2222", "host.domain"]);
}

#[test]
fn test_tier5_ssh_args_identity_file_traversal_and_spaces() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "server.example.com".to_string();
    conn.port = 2222;
    conn.username = "user1".to_string();

    let identity = Some("  /path with spaces/and..traversal/id_ed25519  ");
    let args = build_ssh_args_with_identity(&conn, identity);

    assert_eq!(args[0], "ssh");
    assert_eq!(args[1], "-p");
    assert_eq!(args[2], "2222");
    assert_eq!(args[3], "-i");
    assert_eq!(args[4], "/path with spaces/and..traversal/id_ed25519");
    assert_eq!(args[5], "user1@server.example.com");
}

#[test]
fn test_tier5_build_terminal_command_variants() {
    let mut conn = Connection::default();
    conn.protocol = Protocol::Ssh;
    conn.host = "bastion.internal".to_string();
    conn.port = 22;
    conn.username = "root".to_string();

    for &term in TERMINAL_CANDIDATES {
        let cmd = build_terminal_command(term, &conn, None);
        assert_eq!(cmd.get_program().to_string_lossy(), term);
    }
}
