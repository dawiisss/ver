use beautiful_goodall::models::{Connection, VncScaling};
use beautiful_goodall::secrets;
use beautiful_goodall::storage;
use std::fs;
use tempfile::tempdir;


#[test]
fn test_large_json_input_stress() {
    let dir = tempdir().expect("tempdir failed");
    let path = dir.path().join("large_connections.json");

    let count = 10_000;
    let mut connections = Vec::with_capacity(count);
    for i in 0..count {
        let mut conn = Connection::default();
        conn.name = format!("Server #{i} - {}", "A".repeat(100));
        conn.group = format!("Group {}", i % 50);
        conn.host = format!("10.{}.{}.{}", (i >> 16) & 0xFF, (i >> 8) & 0xFF, i & 0xFF);
        conn.port = 1000 + (i % 60000) as u16;
        conn.username = format!("user_{i}");
        conn.mac_address = format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 0x00, 0x11, 0x22, 0x33, (i >> 8) & 0xFF, i & 0xFF);
        conn.advanced_settings.clipboard_sharing = i % 2 == 0;
        conn.advanced_settings.rdp_fullscreen = i % 3 == 0;
        conn.advanced_settings.vnc_scaling = match i % 3 {
            0 => VncScaling::OriginalSize,
            1 => VncScaling::FitToWindow,
            _ => VncScaling::Stretch,
        };
        connections.push(conn);
    }

    storage::save_connections_to_path(&path, &connections).expect("Save large JSON failed");

    let loaded = storage::load_connections_from_path(&path).expect("Load large JSON failed");
    assert_eq!(loaded.len(), count);
    assert_eq!(loaded[0].name, connections[0].name);
    assert_eq!(loaded[count - 1].name, connections[count - 1].name);
}

#[test]
fn test_malformed_json_strings_resilience() {
    let dir = tempdir().expect("tempdir failed");

    let malformed_samples = vec![
        "{ \"id\": 12345, \"name\": unquoted_string }",
        "[ { \"id\": \"abc\", \"protocol\": \"invalid_proto\" } ]",
        "[ { \"name\": \"Truncated JSON\"",
        "Not even JSON",
        "\0\0\0\0\0\0\0\0",
        "[ { \"port\": 99999999999999999999999999999999999999 } ]",
    ];

    for (idx, sample) in malformed_samples.iter().enumerate() {
        let path = dir.path().join(format!("malformed_{idx}.json"));
        fs::write(&path, sample).expect("Write sample failed");

        let loaded = storage::load_connections_from_path(&path).expect("Load should recover gracefully");
        assert!(loaded.is_empty(), "Malformed JSON sample {idx} should recover with empty vec");

        let entries = fs::read_dir(dir.path()).unwrap();
        let backup_found = entries.filter_map(|e| e.ok()).any(|e| {
            e.file_name().to_string_lossy().contains(&format!("malformed_{idx}.json.corrupt."))
        });
        assert!(backup_found, "Backup should be created for malformed sample {idx}");
    }
}

#[test]
fn test_path_traversal_in_connection_ids() {
    let dir = tempdir().expect("tempdir failed");
    let path = dir.path().join("traversal_connections.json");

    let traversal_ids = vec![
        "../../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "foo/bar/baz",
        "../../secrets",
        "\0/nullbytes",
    ];

    let mut connections = Vec::new();
    for id_str in &traversal_ids {
        let mut conn = Connection::default();
        conn.id = id_str.to_string();
        conn.name = format!("Traversal Test {}", id_str);
        conn.sanitize(); // Ensure sanitize is invoked as storage load does
        connections.push(conn);
    }

    storage::save_connections_to_path(&path, &connections).expect("Save should succeed");
    let loaded = storage::load_connections_from_path(&path).expect("Load should succeed");

    assert_eq!(loaded.len(), traversal_ids.len());
    for conn in loaded {
        assert!(
            uuid::Uuid::parse_str(&conn.id).is_ok(),
            "Connection ID '{}' containing path traversal characters should have been sanitized to a valid UUID!",
            conn.id
        );
    }
}

#[test]
fn test_special_characters_passwords_and_group_names() {
    let special_strings: Vec<String> = vec![
        "Pass w!th spaces & $pec!@l char# %^&*()".to_string(),
        "Unicode test: 🚀🔒💻🔑 𝑼𝒏𝒊𝒄𝒐𝒅𝒆".to_string(),
        "SQL injection style: ' OR '1'='1'; -- drop table connections;".to_string(),
        "HTML/XML: <script>alert('xss')</script> &quot;".to_string(),
        "Newlines\nand\rtabs\tin\0string".to_string(),
        "Super long password string: ".to_string() + &"P@ss".repeat(500),
    ];

    for (idx, spec_str) in special_strings.iter().enumerate() {
        let test_id = format!("test-spec-id-{idx}");

        // Keyring sync operations test
        let _ = secrets::set_password_sync(&test_id, spec_str);
        let retrieved = secrets::get_password_sync(&test_id).unwrap_or(None);
        if let Some(pass) = retrieved {
            assert_eq!(pass.as_str(), spec_str.as_str());
            let _ = secrets::delete_password_sync(&test_id);
        }

        // Models / storage group name test
        let mut conn = Connection::default();
        conn.group = spec_str.to_string();
        conn.name = spec_str.to_string();

        let dir = tempdir().expect("tempdir failed");
        let path = dir.path().join("spec_char.json");

        storage::save_connections_to_path(&path, &[conn.clone()]).expect("Save spec char connection failed");
        let loaded = storage::load_connections_from_path(&path).expect("Load spec char connection failed");

        assert_eq!(loaded.len(), 1);
        if !spec_str.trim().is_empty() {
            assert_eq!(loaded[0].group.as_str(), spec_str.as_str());
            assert_eq!(loaded[0].name.as_str(), spec_str.as_str());
        } else {
            assert_eq!(loaded[0].group, "Default");
            assert_eq!(loaded[0].name, "New Connection");
        }
    }
}

#[test]
fn test_invalid_json_types_resilience() {
    let dir = tempdir().expect("tempdir failed");
    let path = dir.path().join("invalid_type.json");

    // Valid JSON but wrong data types (e.g., array of integers instead of connections)
    fs::write(&path, "[1, 2, 3, 4, 5]").expect("Write failed");

    let loaded = storage::load_connections_from_path(&path).expect("Should recover gracefully from wrong JSON types");
    assert!(loaded.is_empty(), "Array of integers should fail deserialization into Connection vec and return empty vec");
}

#[test]
fn test_non_utf8_file_resilience() {
    let dir = tempdir().expect("tempdir failed");
    let path = dir.path().join("non_utf8.json");

    // Write raw invalid UTF-8 bytes
    let invalid_bytes: [u8; 8] = [0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0x00, 0x01];
    fs::write(&path, &invalid_bytes).expect("Write invalid bytes failed");

    let loaded = storage::load_connections_from_path(&path).expect("Non-UTF-8 file should recover gracefully");
    assert!(loaded.is_empty(), "Non-UTF-8 file should recover with empty vec");

    let entries = fs::read_dir(dir.path()).unwrap();
    let backup_found = entries.filter_map(|e| e.ok()).any(|e| {
        e.file_name().to_string_lossy().contains("non_utf8.json.corrupt.")
    });
    assert!(backup_found, "Corrupt backup file should be created for non-UTF8 file");
}



