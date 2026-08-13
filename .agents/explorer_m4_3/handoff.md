# Milestone 4 Technical Report: Wake-on-LAN (WoL) Magic Packet Generator

**Author**: `explorer_m4_3` (teamwork_preview_explorer)  
**Target Module**: `src/network.rs`  
**Date**: 2026-08-12  

---

## 1. Observation

### 1.1 Existing Files and Code State
- **`src/models.rs` (Lines 224-236)**:
  `Connection::validate_mac(&self)` strips characters with `c.is_ascii_hexdigit()` and verifies length is 12:
  ```rust
  pub fn validate_mac(&self) -> Result<Option<String>, String> {
      let trimmed = self.mac_address.trim();
      if trimmed.is_empty() {
          return Ok(None);
      }
      let clean: String = trimmed.chars().filter(|c| c.is_ascii_hexdigit()).collect();
      if clean.len() == 12 {
          Ok(Some(clean.to_uppercase()))
      } else {
          Err(format!("Invalid MAC address format: '{}'", self.mac_address))
      }
  }
  ```
- **`src/network.rs` (Lines 1-43)**:
  Existing `src/network.rs` implementation:
  ```rust
  use anyhow::{anyhow, Result};
  use std::net::UdpSocket;

  pub fn build_wol_packet(mac_address: &str) -> Result<Vec<u8>> {
      let clean_mac = mac_address.replace(':', "").replace('-', "");
      if clean_mac.len() != 12 {
          return Err(anyhow!("Invalid MAC address length: {}", mac_address));
      }
      let mac_bytes = hex::decode(&clean_mac)
          .map_err(|e| anyhow!("Invalid hex characters in MAC: {}", e))?;

      let mut packet = vec![0xFF; 6];
      for _ in 0..16 {
          packet.extend_from_slice(&mac_bytes);
      }
      Ok(packet)
  }

  pub fn send_wol(mac_address: &str) -> Result<()> {
      let packet = build_wol_packet(mac_address)?;
      let socket = UdpSocket::bind("0.0.0.0:0")?;
      socket.set_broadcast(true)?;
      socket.send_to(&packet, "255.255.255.255:9")?;
      Ok(())
  }

  mod hex {
      use anyhow::{anyhow, Result};

      pub fn decode(hex_str: &str) -> Result<Vec<u8>> {
          if hex_str.len() % 2 != 0 {
              return Err(anyhow!("Odd length hex string"));
          }
          let mut bytes = Vec::with_capacity(hex_str.len() / 2);
          for i in (0..hex_str.len()).step_by(2) {
              let byte = u8::from_str_radix(&hex_str[i..i + 2], 16)
                  .map_err(|e| anyhow!("Invalid hex byte: {}", e))?;
              bytes.push(byte);
          }
          Ok(bytes)
      }
  }
  ```
- **Current Deficiencies in `src/network.rs`**:
  1. **Dot Delimiters Unsupported**: `.replace(':', "").replace('-', "")` fails for Cisco format MAC strings (`0011.2233.4455`) or dot-separated byte strings (`00.11.22.33.44.55`).
  2. **Whitespace Unsupported**: Leading/trailing or internal spaces cause validation failure.
  3. **No Type-Safe 6-byte Array MAC Parsing**: MAC address parsing returns dynamic `Vec<u8>` without enforcing exact `[u8; 6]` type guarantee.
  4. **Zero Unit Tests**: `src/network.rs` currently has 0 unit tests.
  5. **Hardcoded Destination**: `send_wol` cannot send to directed broadcast addresses (e.g. `192.168.1.255`) or custom ports (e.g. port 7).

---

## 2. Logic Chain

1. **MAC Address Parsing & Normalization Requirements**:
   - Supported input formats:
     - Colon-separated: `00:11:22:33:44:55`
     - Hyphen-separated: `00-11-22-33-44-55`
     - Cisco dot-separated: `0011.2233.4455`
     - Byte dot-separated: `00.11.22.33.44.55`
     - Unseparated hex: `001122334455`
     - Mixed case / outer whitespace: `  00:aB:cD:eF:12:34  `
   - Algorithm:
     Filter out common hex separators (`:`, `-`, `.`, and whitespace). Check that remaining string length is exactly 12 characters. Parse consecutive pairs of 2 hex characters using `u8::from_str_radix(&slice, 16)` into a fixed-size `[u8; 6]` array.

2. **Magic Packet Construction**:
   - WoL magic packet spec: 6 sync bytes of `0xFF` followed by 16 iterations of the 6-byte target MAC address.
   - Total length: `6 + (16 * 6) = 102` bytes.
   - Statically sized output `[u8; 102]` or `Vec<u8>` with capacity 102 ensures optimal memory layout and zero heap reallocation.

3. **UDP Socket Broadcast Mechanics**:
   - Bind local UDP socket to `0.0.0.0:0` (OS chooses ephemeral port).
   - Must set `socket.set_broadcast(true)`; omitting this option causes `send_to` to fail with permission denied on broadcast IP targets (`255.255.255.255` or `x.y.z.255`).
   - Default broadcast target: `255.255.255.255:9`.
   - Provide `send_wol_to(mac, broadcast_host, port)` for custom subnet broadcasts.

4. **Unit Test Strategy**:
   - Test `parse_mac_address` across all format variants (colon, hyphen, cisco dot, byte dot, unseparated, uppercase, lowercase, whitespace).
   - Test invalid inputs (too short, too long, invalid hex characters).
   - Test `build_wol_packet` structure (length = 102, prefix = 6x0xFF, 16x MAC repeat).
   - Test live UDP transmission via loopback (`127.0.0.1`) socket receiver to verify end-to-end packet transmission without requiring network broadcast privileges.

---

## 3. Caveats

- **Network Environment Restrictions**: Some network routers or firewall rules may block limited broadcast packets (`255.255.255.255`). Providing `send_wol_to` allows sending to subnet-directed broadcast IPs (e.g., `192.168.1.255`).
- **Permissions**: Standard UDP broadcast does not require root privileges on Linux when using `UdpSocket::bind("0.0.0.0:0")` and `set_broadcast(true)`.

---

## 4. Conclusion & Proposed Code Implementation

The following complete refactored `src/network.rs` fulfills all requirements:

```rust
use anyhow::{anyhow, Result};
use std::net::UdpSocket;

/// Default UDP port for Wake-on-LAN (Discard Protocol).
pub const DEFAULT_WOL_PORT: u16 = 9;

/// Default broadcast IPv4 address for local network segment.
pub const DEFAULT_BROADCAST_ADDR: &str = "255.255.255.255";

/// Parse a MAC address string into a 6-byte array.
///
/// Supports colon (`00:11:22:33:44:55`), hyphen (`00-11-22-33-44-55`),
/// Cisco dot (`0011.2233.4455`), byte dot (`00.11.22.33.44.55`),
/// and unseparated hex strings (`001122334455`).
pub fn parse_mac_address(mac_address: &str) -> Result<[u8; 6]> {
    let clean: String = mac_address
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-' && *c != '.')
        .collect();

    if clean.len() != 12 {
        return Err(anyhow!(
            "Invalid MAC address length (expected 12 hex digits, found {}): '{}'",
            clean.len(),
            mac_address
        ));
    }

    let mut mac_bytes = [0u8; 6];
    for i in 0..6 {
        let hex_slice = &clean[i * 2..i * 2 + 2];
        mac_bytes[i] = u8::from_str_radix(hex_slice, 16).map_err(|_| {
            anyhow!("Invalid hex byte '{}' in MAC address: '{}'", hex_slice, mac_address)
        })?;
    }

    Ok(mac_bytes)
}

/// Construct a Wake-on-LAN Magic Packet payload (102 bytes).
///
/// Payload consists of 6 bytes of 0xFF followed by 16 repetitions of the 6-byte MAC address.
pub fn build_wol_packet(mac_address: &str) -> Result<Vec<u8>> {
    let mac = parse_mac_address(mac_address)?;
    let mut packet = Vec::with_capacity(102);
    packet.extend_from_slice(&[0xFF; 6]);
    for _ in 0..16 {
        packet.extend_from_slice(&mac);
    }
    Ok(packet)
}

/// Send a Wake-on-LAN magic packet to the default broadcast address (255.255.255.255:9).
pub fn send_wol(mac_address: &str) -> Result<()> {
    send_wol_to(mac_address, DEFAULT_BROADCAST_ADDR, DEFAULT_WOL_PORT)
}

/// Send a Wake-on-LAN magic packet to a specified target broadcast host/IP and UDP port.
pub fn send_wol_to(mac_address: &str, broadcast_host: &str, port: u16) -> Result<()> {
    let packet = build_wol_packet(mac_address)?;
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| anyhow!("Failed to bind UDP socket: {}", e))?;

    socket
        .set_broadcast(true)
        .map_err(|e| anyhow!("Failed to set UDP socket broadcast option: {}", e))?;

    let dest = format!("{}:{}", broadcast_host, port);
    let bytes_sent = socket
        .send_to(&packet, &dest)
        .map_err(|e| anyhow!("Failed to send WoL packet to {}: {}", dest, e))?;

    if bytes_sent != packet.len() {
        return Err(anyhow!(
            "Incomplete WoL packet sent: expected {} bytes, sent {} bytes",
            packet.len(),
            bytes_sent
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_mac_colon_format() {
        let mac = parse_mac_address("00:11:22:33:44:55").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_hyphen_format() {
        let mac = parse_mac_address("00-11-22-33-44-55").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_cisco_dot_format() {
        let mac = parse_mac_address("0011.2233.4455").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_byte_dot_format() {
        let mac = parse_mac_address("00.11.22.33.44.55").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_unseparated_format() {
        let mac = parse_mac_address("001122334455").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_case_and_whitespace() {
        let mac = parse_mac_address("  00:aB:cD:eF:12:34  ").unwrap();
        assert_eq!(mac, [0x00, 0xAB, 0xCD, 0xEF, 0x12, 0x34]);
    }

    #[test]
    fn test_parse_mac_invalid_inputs() {
        assert!(parse_mac_address("00:11:22:33:44").is_err());
        assert!(parse_mac_address("00:11:22:33:44:55:66").is_err());
        assert!(parse_mac_address("00:11:22:33:44:ZZ").is_err());
    }

    #[test]
    fn test_build_wol_packet_structure() {
        let mac_str = "00:11:22:33:44:55";
        let packet = build_wol_packet(mac_str).unwrap();

        assert_eq!(packet.len(), 102);
        assert_eq!(&packet[0..6], &[0xFF; 6]);

        let expected_mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        for i in 0..16 {
            let start = 6 + i * 6;
            assert_eq!(&packet[start..start + 6], &expected_mac);
        }
    }

    #[test]
    fn test_send_wol_loopback() {
        let rx_socket = UdpSocket::bind("127.0.0.1:0").expect("Must bind receiver socket");
        let rx_addr = rx_socket.local_addr().expect("Must get local addr");
        rx_socket
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("Must set timeout");

        let result = send_wol_to("00:11:22:33:44:55", "127.0.0.1", rx_addr.port());
        assert!(result.is_ok(), "Sending WoL to loopback should succeed");

        let mut buf = [0u8; 200];
        let (amt, _src) = rx_socket.recv_from(&mut buf).expect("Must receive UDP packet");
        assert_eq!(amt, 102);
        assert_eq!(&buf[0..6], &[0xFF; 6]);
        assert_eq!(&buf[6..12], &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }
}
```

---

## 5. Verification Method

1. **Code Inspection**:
   - Check `src/network.rs` contains `parse_mac_address`, `build_wol_packet`, `send_wol`, and `send_wol_to`.
   - Verify support for `:`, `-`, `.`, and raw hex formats.
2. **Command Verification**:
   - Run `cargo test --lib network::tests`
   - Expect all 9 unit tests (`test_parse_mac_*`, `test_build_wol_packet_structure`, `test_send_wol_loopback`) to pass with 0 failures.
3. **Invalidation Conditions**:
   - Test fails if MAC formats with dots (e.g. `0011.2233.4455`) reject parsing.
   - Test fails if magic packet payload length is not 102 bytes or sync header is missing `0xFF * 6`.
   - Test fails if `set_broadcast(true)` is omitted when broadcasting.
