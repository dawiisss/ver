import json
import subprocess
import os

# 1. Define sample connection data structure in Python dict format matching Rust defaults
conn_data = [{
    "id": "11111111-2222-3333-4444-555555555555",
    "name": "New Connection",
    "protocol": "rdp",
    "host": "",
    "port": 3389,
    "username": "",
    "mac_address": "",
    "group": "Default",
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
}]

py_serialized = json.dumps(conn_data, indent=4)
print("--- Python json.dumps(indent=4) ---")
print(repr(py_serialized))

# 2. Get Rust serialized JSON from storage::to_json_4spaces
# We can invoke rustc linking with libbeautiful_goodall or run a cargo test that prints it.
rust_test_code = """
use beautiful_goodall::models::*;
use beautiful_goodall::storage::*;

fn main() {
    let mut conn = Connection::default();
    conn.id = "11111111-2222-3333-4444-555555555555".to_string();
    let json_str = to_json_4spaces(&vec![conn]).unwrap();
    print!("{}", json_str);
}
"""

with open("/tmp/rust_test.rs", "w") as f:
    f.write(rust_test_code)

# Compile rust_test with rustc linking the target lib
res = subprocess.run(
    ["cargo", "run", "--bin", "beautiful-goodall", "--color", "never"],
    cwd="/home/dawiisss/Documents/antigravity/beautiful-goodall",
    capture_output=True, text=True
)
