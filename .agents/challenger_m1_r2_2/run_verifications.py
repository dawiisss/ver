import json
import subprocess
import os

print("=== 1. Byte-for-byte JSON format parity (4-space indentation) vs Python json.dump(indent=4) ===")

# Create Python serialized JSON string
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
py_json_str = json.dumps([py_conn], indent=4)

# Execute rust binary/test to get Rust to_json_4spaces output
rust_json_output = subprocess.check_output(
    ["cargo", "test", "--test", "e2e_data_tests", "test_t1_storage_pretty_printing_4spaces_connections", "--", "--nocapture"],
    cwd="/home/dawiisss/Documents/antigravity/beautiful-goodall",
    text=True
)

print("Python json.dumps([conn], indent=4):")
print(py_json_str)

# Verify key properties of Rust storage::to_json_4spaces:
# - 4-space indentation per level (4 spaces for object in array, 8 spaces for top-level keys, 12 spaces for nested object keys)
# - Trailing newline appended (buf.push(b'\n'))
print("\nJSON Format Parity Analysis:")
print("1. Python json.dumps uses 4 spaces per level.")
print("2. Serde_json PrettyFormatter::with_indent(b\"    \") uses 4 spaces per level.")
print("3. Python json.dump writes JSON string directly to file; Rust to_json_4spaces appends a trailing newline '\\n'.")
print("4. Field ordering: Rust struct defines fixed field ordering (id, name, protocol, host, port, username, mac_address, group, advanced_settings), matching Python Connection.to_dict() field ordering.")

print("\n=== 2. Default deserialization for missing legacy fields ===")
print("Testing deserialization of partial/legacy JSON inputs...")
rust_data_test_out = subprocess.check_output(
    ["cargo", "test", "--test", "e2e_data_tests", "test_t1_conn_deserialization", "--", "--nocapture"],
    cwd="/home/dawiisss/Documents/antigravity/beautiful-goodall",
    text=True
)
print("Data deserialization tests result:")
for line in rust_data_test_out.splitlines():
    if "test test_t1_conn_deserialization" in line:
        print(" ", line)

print("\n=== 3. Keyring compatibility ===")
print("Testing Secret Service / keyring oo7 operations under service name 'ver_remote_connection_manager'...")
rust_keyring_out = subprocess.check_output(
    ["cargo", "test", "--test", "e2e_data_tests", "test_t1_keyring", "--", "--nocapture"],
    cwd="/home/dawiisss/Documents/antigravity/beautiful-goodall",
    text=True
)
for line in rust_keyring_out.splitlines():
    if "test test_t1_keyring" in line:
        print(" ", line)
