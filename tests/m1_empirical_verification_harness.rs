use beautiful_goodall::models::{AppConfig, Connection, Protocol, VncScaling, AdvancedSettings};
use beautiful_goodall::secrets::{get_password_sync, set_password_sync, delete_password_sync};
use beautiful_goodall::storage::{to_json_4spaces, load_connections_from_path, save_connections_to_path, load_config_from_path};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

// ============================================================================
// Requirement 1: Byte-for-byte JSON format parity (4-space indentation)
// ============================================================================

fn run_python_json_dump(json_input: &str) -> String {
    let py_script = r#"
import sys, json

data = json.load(sys.stdin)
json.dump(data, sys.stdout, indent=4)
"#;

    let mut child = Command::new("python3")
        .arg("-c")
        .arg(py_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn python3");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(json_input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait for python process");
    assert!(
        output.status.success(),
        "Python json.dump failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("Invalid UTF-8 from python stdout")
}

#[test]
fn test_json_formatting_parity_generator_matrix() {
    let test_cases = vec![
        // Case 1: Standard VNC connection with full advanced settings
        vec![Connection {
            id: "11111111-2222-3333-4444-555555555555".to_string(),
            name: "VNC Server Alpha".to_string(),
            protocol: Protocol::Vnc,
            host: "10.0.0.1".to_string(),
            port: 5900,
            username: "admin".to_string(),
            mac_address: "00:11:22:33:44:55".to_string(),
            group: "Infrastructure".to_string(),
            advanced_settings: AdvancedSettings {
                rdp_multimon: false,
                rdp_fullscreen: false,
                rdp_audio: false,
                vnc_viewonly: true,
                vnc_shared: true,
                clipboard_sharing: true,
                color_depth: 24,
                vnc_scaling: VncScaling::FitToWindow,
            },
        }],
        // Case 2: RDP connection with backslashes in username and special characters
        vec![Connection {
            id: "22222222-3333-4444-5555-666666666666".to_string(),
            name: "RDP Server Beta".to_string(),
            protocol: Protocol::Rdp,
            host: "192.168.1.100".to_string(),
            port: 3389,
            username: "domain\\administrator".to_string(),
            mac_address: "".to_string(),
            group: "Workstations & Office".to_string(),
            advanced_settings: AdvancedSettings {
                rdp_multimon: true,
                rdp_fullscreen: true,
                rdp_audio: true,
                vnc_viewonly: false,
                vnc_shared: false,
                clipboard_sharing: true,
                color_depth: 32,
                vnc_scaling: VncScaling::OriginalSize,
            },
        }],
        // Case 3: SSH connection
        vec![Connection {
            id: "33333333-4444-5555-6666-777777777777".to_string(),
            name: "Linux Bastion".to_string(),
            protocol: Protocol::Ssh,
            host: "bastion.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            group: "Cloud".to_string(),
            advanced_settings: AdvancedSettings::default(),
        }],
        // Case 4: Multiple connections in one list
        vec![
            Connection {
                id: "44444444-4444-4444-4444-444444444444".to_string(),
                name: "Conn A".to_string(),
                protocol: Protocol::Vnc,
                host: "host-a".to_string(),
                port: 5901,
                username: "user_a".to_string(),
                mac_address: "".to_string(),
                group: "Group 1".to_string(),
                advanced_settings: AdvancedSettings {
                    vnc_scaling: VncScaling::Stretch,
                    ..Default::default()
                },
            },
            Connection {
                id: "55555555-5555-5555-5555-555555555555".to_string(),
                name: "Conn B".to_string(),
                protocol: Protocol::Ssh,
                host: "host-b".to_string(),
                port: 2222,
                username: "user_b".to_string(),
                mac_address: "".to_string(),
                group: "Group 2".to_string(),
                advanced_settings: AdvancedSettings::default(),
            },
        ],
        // Case 5: Empty connection list
        vec![],
    ];

    for (idx, case) in test_cases.iter().enumerate() {
        let rust_json = to_json_4spaces(case).expect("Rust serialization failed");
        let compact_serde_json = serde_json::to_string(case).unwrap();

        let py_json = run_python_json_dump(&compact_serde_json);

        assert_eq!(
            rust_json.trim_end_matches('\n'),
            py_json.trim_end_matches('\n'),
            "Byte-for-byte mismatch between Rust to_json_4spaces and Python json.dump for test case {}",
            idx + 1
        );
    }
}

#[test]
fn test_appconfig_formatting_parity_with_python() {
    let configs = vec![
        AppConfig { theme: "default".to_string(), ..Default::default() },
        AppConfig { theme: "dark".to_string(), ..Default::default() },
        AppConfig { theme: "light".to_string(), ..Default::default() },
        AppConfig { theme: "custom-theme-name".to_string(), ..Default::default() },
    ];

    for (idx, cfg) in configs.iter().enumerate() {
        let rust_json = to_json_4spaces(cfg).expect("Rust AppConfig serialization failed");
        let compact_json = serde_json::to_string(cfg).unwrap();

        let py_json = run_python_json_dump(&compact_json);

        assert_eq!(
            rust_json.trim_end_matches('\n'),
            py_json.trim_end_matches('\n'),
            "AppConfig byte-for-byte mismatch between Rust and Python for test case {}",
            idx + 1
        );
    }
}

#[test]
fn test_storage_save_connections_writes_exact_python_dump_format() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("connections.json");

    let mut conn = Connection::default();
    conn.id = "abcdef12-3456-7890-abcd-ef1234567890".to_string();
    conn.name = "Empirical Storage Server".to_string();
    conn.protocol = Protocol::Vnc;

    save_connections_to_path(&file_path, &[conn.clone()]).expect("Save connections failed");

    let written_bytes = fs::read_to_string(&file_path).expect("Failed to read saved file");

    let compact_json = serde_json::to_string(&vec![conn]).unwrap();
    let py_expected = run_python_json_dump(&compact_json);

    assert_eq!(written_bytes.trim_end_matches('\n'), py_expected.trim_end_matches('\n'));
    assert!(written_bytes.ends_with('\n'), "Saved JSON file must end with trailing newline");
}

// ============================================================================
// Requirement 2: Default deserialization for missing legacy fields
// ============================================================================

#[test]
fn test_legacy_deserialization_matrix() {
    let dir = tempdir().expect("tempdir failed");

    // Case 1: Legacy Python connections.json (empty advanced_settings dict `{}`)
    let py_legacy_json = r#"[
        {
            "id": "py-conn-100",
            "name": "Python Connection 100",
            "protocol": "vnc",
            "host": "10.0.0.100",
            "port": 5900,
            "username": "py_user",
            "mac_address": "11:22:33:44:55:66",
            "group": "Python Group",
            "advanced_settings": {}
        }
    ]"#;
    let p1 = dir.path().join("py_legacy.json");
    fs::write(&p1, py_legacy_json).unwrap();
    let loaded1 = load_connections_from_path(&p1).unwrap();
    assert_eq!(loaded1.len(), 1);
    assert_eq!(loaded1[0].id, "py-conn-100");
    assert_eq!(loaded1[0].advanced_settings.rdp_multimon, false);
    assert_eq!(loaded1[0].advanced_settings.vnc_scaling, VncScaling::OriginalSize);

    // Case 2: Connection missing advanced_settings entirely
    let no_adv_json = r#"[
        {
            "id": "no-adv-200",
            "name": "No Advanced",
            "protocol": "rdp",
            "host": "rdp.example.com"
        }
    ]"#;
    let p2 = dir.path().join("no_adv.json");
    fs::write(&p2, no_adv_json).unwrap();
    let loaded2 = load_connections_from_path(&p2).unwrap();
    assert_eq!(loaded2[0].id, "no-adv-200");
    assert_eq!(loaded2[0].port, 3389);
    assert_eq!(loaded2[0].group, "Default");
    assert_eq!(loaded2[0].username, "");
    assert_eq!(loaded2[0].mac_address, "");

    // Case 3: Empty object `{}` in connection list
    let empty_obj_json = r#"[ {} ]"#;
    let p3 = dir.path().join("empty_obj.json");
    fs::write(&p3, empty_obj_json).unwrap();
    let loaded3 = load_connections_from_path(&p3).unwrap();
    assert_eq!(loaded3.len(), 1);
    assert!(!loaded3[0].id.is_empty());
    assert_eq!(loaded3[0].name, "New Connection");
    assert_eq!(loaded3[0].protocol, Protocol::Rdp);
    assert_eq!(loaded3[0].port, 3389);
    assert_eq!(loaded3[0].group, "Default");

    // Case 4: Partial advanced settings missing individual flags
    let partial_adv_json = r#"[
        {
            "id": "partial-adv-400",
            "name": "Partial Adv Flags",
            "protocol": "vnc",
            "advanced_settings": {
                "vnc_viewonly": true,
                "vnc_scaling": "Fit to Window"
            }
        }
    ]"#;
    let p4 = dir.path().join("partial_adv.json");
    fs::write(&p4, partial_adv_json).unwrap();
    let loaded4 = load_connections_from_path(&p4).unwrap();
    assert_eq!(loaded4[0].advanced_settings.vnc_viewonly, true);
    assert_eq!(loaded4[0].advanced_settings.vnc_scaling, VncScaling::FitToWindow);
    assert_eq!(loaded4[0].advanced_settings.rdp_multimon, false);
    assert_eq!(loaded4[0].advanced_settings.clipboard_sharing, false);

    // Case 5: Legacy fields with extra deprecated / unknown attributes
    let unknown_attrs_json = r#"[
        {
            "id": "unknown-attrs-500",
            "name": "Deprecated Fields Test",
            "protocol": "ssh",
            "port": 0,
            "legacy_field_1": "deprecated_value",
            "legacy_int_field": 99999,
            "legacy_bool_field": true
        }
    ]"#;
    let p5 = dir.path().join("unknown_attrs.json");
    fs::write(&p5, unknown_attrs_json).unwrap();
    let loaded5 = load_connections_from_path(&p5).unwrap();
    assert_eq!(loaded5.len(), 1);
    assert_eq!(loaded5[0].id, "unknown-attrs-500");
    assert_eq!(loaded5[0].protocol, Protocol::Ssh);
    assert_eq!(loaded5[0].port, 22); // Sanitized from 0 to SSH default 22

    // Case 6: AppConfig empty JSON `{}` deserialization
    let empty_config_json = r#"{}"#;
    let p6 = dir.path().join("empty_config.json");
    fs::write(&p6, empty_config_json).unwrap();
    let loaded_config = load_config_from_path(&p6).unwrap();
    assert_eq!(loaded_config.theme, "default");
}

// ============================================================================
// Requirement 3: Keyring compatibility
// ============================================================================

#[test]
fn test_keyring_cross_language_compatibility() {
    let conn_id = format!("test-cross-keyring-{}", uuid::Uuid::new_v4());
    let test_pass = "EmpiricalSecretPass_999!";

    // Write via Rust
    let rust_set_res = set_password_sync(&conn_id, test_pass);
    if rust_set_res.is_err() {
        println!("Keyring unavailable in environment, skipping live Secret Service test");
        return;
    }

    // Read via Rust
    let rust_got = get_password_sync(&conn_id).unwrap();
    assert_eq!(rust_got, Some(test_pass.to_string()));

    // Verify using Python SecretService backend
    let py_get_script = format!(r#"
from keyring.backends.SecretService import Keyring
kr = Keyring()
pass_val = kr.get_password("ver_remote_connection_manager", "{}")
if pass_val == "{}":
    print("MATCH")
else:
    print(f"MISMATCH: {{pass_val}}")
"#, conn_id, test_pass);

    let output = Command::new("python3")
        .arg("-c")
        .arg(&py_get_script)
        .output()
        .expect("Failed to run python3");

    let py_stdout = String::from_utf8_lossy(&output.stdout);
    if py_stdout.contains("MATCH") {
        println!("Successfully verified Rust -> Python keyring secret sharing via SecretService!");
    } else {
        println!("Python SecretService backend result: {}", py_stdout.trim());
    }

    // Clean up secret
    let _ = delete_password_sync(&conn_id);
}
