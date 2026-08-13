import json
import subprocess
import os
import sys

def test_json_parity():
    print("=== Testing Byte-for-Byte JSON Format Parity ===")
    py_conn = {
        "id": "11111111-2222-3333-4444-555555555555",
        "name": "Test Server",
        "protocol": "vnc",
        "host": "192.168.1.100",
        "port": 5900,
        "username": "admin",
        "mac_address": "00:11:22:33:44:55",
        "group": "Servers",
        "advanced_settings": {
            "rdp_multimon": False,
            "rdp_fullscreen": False,
            "rdp_audio": False,
            "vnc_viewonly": False,
            "vnc_shared": False,
            "clipboard_sharing": False,
            "color_depth": 0,
            "vnc_scaling": "Original Size"
        }
    }
    py_json = json.dumps([py_conn], indent=4)
    print("Python json.dumps output length:", len(py_json))
    
    # Run rust test or code to get rust JSON string
    # We can write a quick rust file in current dir and compile with cargo/rustc
    rust_code = """
use beautiful_goodall::models::*;
use beautiful_goodall::storage::*;

fn main() {
    let mut conn = Connection::default();
    conn.id = "11111111-2222-3333-4444-555555555555".to_string();
    conn.name = "Test Server".to_string();
    conn.protocol = Protocol::Vnc;
    conn.host = "192.168.1.100".to_string();
    conn.port = 5900;
    conn.username = "admin".to_string();
    conn.mac_address = "00:11:22:33:44:55".to_string();
    conn.group = "Servers".to_string();

    let json_str = to_json_4spaces(&vec![conn]).unwrap();
    print!("{}", json_str);
}
"""
    rust_file = "/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/challenger_m1_r2_2/rust_json_test.rs"
    with open(rust_file, "w") as f:
        f.write(rust_code)
        
    rlib = subprocess.check_output(
        ["find", "/home/dawiisss/Documents/antigravity/beautiful-goodall/target/debug/deps", "-name", "libbeautiful_goodall-*.rlib"],
        text=True
    ).strip().splitlines()[0]
    
    out_bin = "/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/challenger_m1_r2_2/rust_json_test"
    comp_res = subprocess.run(
        ["rustc", "--extern", f"beautiful_goodall={rlib}", rust_file, "-o", out_bin, "-L", "/home/dawiisss/Documents/antigravity/beautiful-goodall/target/debug/deps"],
        capture_output=True, text=True
    )
    if comp_res.returncode != 0:
        print("Rust test compilation error:", comp_res.stderr)
        return False

    rust_json = subprocess.check_output([out_bin], text=True)
    print("Rust to_json_4spaces output length:", len(rust_json))

    # Difference check
    print("Py JSON ends with newline?", py_json.endswith('\n'))
    print("Rust JSON ends with newline?", rust_json.endswith('\n'))

    if py_json == rust_json:
        print("MATCH: Byte-for-byte identical without trailing newline diff!")
    elif py_json + "\n" == rust_json:
        print("MATCH (with trailing newline): Rust appends \\n to Python json.dumps output.")
        print("Python string:\n", py_json)
        print("Rust string:\n", rust_json)
    else:
        print("MISMATCH between Python and Rust JSON!")
        print("Py repr:", repr(py_json))
        print("Rust repr:", repr(rust_json))

def test_missing_legacy_fields():
    print("\n=== Testing Default Deserialization for Missing Legacy Fields ===")
    rust_code = """
use beautiful_goodall::models::*;

fn main() {
    // Legacy json missing id, group, port, advanced_settings, etc.
    let legacy_json = r#"[
        {
            "name": "Legacy Server 1",
            "host": "10.0.0.1"
        },
        {
            "id": "custom-uuid-123",
            "name": "Legacy Server 2",
            "protocol": "ssh",
            "unknown_legacy_field": "foo_bar_deprecated"
        }
    ]"#;

    let conns: Vec<Connection> = serde_json::from_str(legacy_json).expect("Deserialization failed");
    println!("Parsed {} connections", conns.len());
    
    // Check item 1 defaults
    assert!(!conns[0].id.is_empty(), "Missing id should default to random UUID");
    assert_eq!(conns[0].name, "Legacy Server 1");
    assert_eq!(conns[0].protocol, Protocol::Rdp, "Missing protocol should default to rdp");
    assert_eq!(conns[0].port, 3389, "Missing port should default to 3389");
    assert_eq!(conns[0].group, "Default", "Missing group should default to Default");
    assert_eq!(conns[0].advanced_settings, AdvancedSettings::default(), "Missing advanced_settings should default");

    // Check item 2
    assert_eq!(conns[1].id, "custom-uuid-123");
    assert_eq!(conns[1].name, "Legacy Server 2");
    assert_eq!(conns[1].protocol, Protocol::Ssh);
    assert_eq!(conns[1].port, 3389, "Missing port defaults to 3389 before resolve_port");

    println!("SUCCESS: Default deserialization for missing legacy fields verified!");
}
"""
    rust_file = "/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/challenger_m1_r2_2/rust_legacy_test.rs"
    with open(rust_file, "w") as f:
        f.write(rust_code)
        
    rlib = subprocess.check_output(
        ["find", "/home/dawiisss/Documents/antigravity/beautiful-goodall/target/debug/deps", "-name", "libbeautiful_goodall-*.rlib"],
        text=True
    ).strip().splitlines()[0]
    
    out_bin = "/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/challenger_m1_r2_2/rust_legacy_test"
    comp_res = subprocess.run(
        ["rustc", "--extern", f"beautiful_goodall={rlib}", rust_file, "-o", out_bin, "-L", "/home/dawiisss/Documents/antigravity/beautiful-goodall/target/debug/deps"],
        capture_output=True, text=True
    )
    if comp_res.returncode != 0:
        print("Rust legacy test compilation error:", comp_res.stderr)
        return

    out = subprocess.check_output([out_bin], text=True)
    print(out)

def test_keyring_compatibility():
    print("\n=== Testing Keyring Compatibility ===")
    rust_code = """
use beautiful_goodall::secrets::*;

#[tokio::main]
async fn main() {
    let test_id = "compat-test-uuid-9999";
    let test_pass = "SecretPassw0rd!#";

    println!("Testing keyring set_password...");
    match set_password(test_id, test_pass).await {
        Ok(_) => println!("set_password succeeded or handled gracefully"),
        Err(e) => println!("set_password error: {}", e),
    }

    println!("Testing keyring get_password...");
    match get_password(test_id).await {
        Ok(opt) => println!("get_password returned: {:?}", opt),
        Err(e) => println!("get_password error: {}", e),
    }

    println!("Testing keyring delete_password...");
    match delete_password(test_id).await {
        Ok(_) => println!("delete_password succeeded or handled gracefully"),
        Err(e) => println!("delete_password error: {}", e),
    }
}
"""
    rust_file = "/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/challenger_m1_r2_2/rust_keyring_test.rs"
    with open(rust_file, "w") as f:
        f.write(rust_code)
        
    rlib = subprocess.check_output(
        ["find", "/home/dawiisss/Documents/antigravity/beautiful-goodall/target/debug/deps", "-name", "libbeautiful_goodall-*.rlib"],
        text=True
    ).strip().splitlines()[0]
    
    out_bin = "/home/dawiisss/Documents/antigravity/beautiful-goodall/.agents/challenger_m1_r2_2/rust_keyring_test"
    deps_dir = "/home/dawiisss/Documents/antigravity/beautiful-goodall/target/debug/deps"
    
    # We use cargo run or rustc with all deps
    # Cargo is easier by creating an integration test or running cargo test
    cmd = ["cargo", "test", "--test", "e2e_data_tests", "test_t1_keyring", "--", "--nocapture"]
    res = subprocess.run(cmd, cwd="/home/dawiisss/Documents/antigravity/beautiful-goodall", capture_output=True, text=True)
    print(res.stdout)

if __name__ == "__main__":
    test_json_parity()
    test_missing_legacy_fields()
    test_keyring_compatibility()
