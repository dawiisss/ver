use std::net::UdpSocket;

/// Default UDP port for Wake-on-LAN (Discard protocol port 9).
pub const DEFAULT_WOL_PORT: u16 = 9;

/// Default broadcast IPv4 address for local network segment.
pub const DEFAULT_BROADCAST_ADDR: &str = "255.255.255.255";

/// Parse a MAC address string into a 6-byte array.
///
/// Supports colon (`00:11:22:33:44:55`), hyphen (`00-11-22-33-44-55`),
/// Cisco dot (`0011.2233.4455`), byte dot (`00.11.22.33.44.55`),
/// and unseparated hex strings (`001122334455`).
pub fn parse_mac_address(mac_address: &str) -> Result<[u8; 6], String> {
    let clean: String = mac_address
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-' && *c != '.')
        .collect();

    if clean.len() != 12 {
        return Err(format!(
            "Invalid MAC address length (expected 12 hex digits, found {}): '{}'",
            clean.len(),
            mac_address
        ));
    }

    let mut mac_bytes = [0u8; 6];
    for i in 0..6 {
        let hex_slice = &clean[i * 2..i * 2 + 2];
        mac_bytes[i] = u8::from_str_radix(hex_slice, 16).map_err(|_| {
            format!("Invalid hex byte '{}' in MAC address: '{}'", hex_slice, mac_address)
        })?;
    }

    Ok(mac_bytes)
}

/// Trait to support flexible MAC address inputs for `build_wol_packet`.
pub trait WolMacInput {
    fn to_mac_bytes(&self) -> Result<[u8; 6], String>;
}

impl WolMacInput for &str {
    fn to_mac_bytes(&self) -> Result<[u8; 6], String> {
        parse_mac_address(self)
    }
}

impl WolMacInput for &String {
    fn to_mac_bytes(&self) -> Result<[u8; 6], String> {
        parse_mac_address(self.as_str())
    }
}

impl WolMacInput for String {
    fn to_mac_bytes(&self) -> Result<[u8; 6], String> {
        parse_mac_address(self.as_str())
    }
}

impl WolMacInput for [u8; 6] {
    fn to_mac_bytes(&self) -> Result<[u8; 6], String> {
        Ok(*self)
    }
}

impl WolMacInput for &[u8; 6] {
    fn to_mac_bytes(&self) -> Result<[u8; 6], String> {
        Ok(**self)
    }
}

/// Construct a Wake-on-LAN Magic Packet payload (102 bytes).
///
/// Payload consists of 6 bytes of 0xFF followed by 16 repetitions of the 6-byte MAC address.
pub fn build_wol_packet<T: WolMacInput>(mac: T) -> Result<Vec<u8>, String> {
    let mac_bytes = mac.to_mac_bytes()?;
    let mut packet = Vec::with_capacity(102);
    packet.extend_from_slice(&[0xFF; 6]);
    for _ in 0..16 {
        packet.extend_from_slice(&mac_bytes);
    }
    Ok(packet)
}

/// Helper returning fixed-size 102-byte array directly from 6-byte MAC array.
pub fn build_wol_packet_bytes(mac: &[u8; 6]) -> [u8; 102] {
    let mut packet = [0xFFu8; 102];
    for i in 0..16 {
        let start = 6 + i * 6;
        packet[start..start + 6].copy_from_slice(mac);
    }
    packet
}

/// Send a Wake-on-LAN magic packet to specified broadcast address (host or host:port).
pub fn send_wol_to(mac_address: &str, target_addr: &str) -> Result<(), String> {
    let packet = build_wol_packet(mac_address)?;
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    socket
        .set_broadcast(true)
        .map_err(|e| format!("Failed to set UDP broadcast option: {}", e))?;

    let addr = if target_addr.contains(':') {
        target_addr.to_string()
    } else {
        format!("{}:{}", target_addr, DEFAULT_WOL_PORT)
    };

    let bytes_sent = socket
        .send_to(&packet, &addr)
        .map_err(|e| format!("Failed to send WoL packet to {}: {}", addr, e))?;

    if bytes_sent != packet.len() {
        return Err(format!(
            "Incomplete WoL packet sent: expected {} bytes, sent {} bytes",
            packet.len(),
            bytes_sent
        ));
    }

    Ok(())
}

/// Send a Wake-on-LAN magic packet to default broadcast address (255.255.255.255:9).
pub fn send_wol(mac_address: &str) -> Result<(), String> {
    send_wol_to(mac_address, DEFAULT_BROADCAST_ADDR)
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
    fn test_build_wol_packet_str_and_bytes() {
        let mac_str = "00:11:22:33:44:55";
        let packet = build_wol_packet(mac_str).unwrap();

        assert_eq!(packet.len(), 102);
        assert_eq!(&packet[0..6], &[0xFF; 6]);

        let expected_mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        for i in 0..16 {
            let start = 6 + i * 6;
            assert_eq!(&packet[start..start + 6], &expected_mac);
        }

        let array_packet = build_wol_packet_bytes(&expected_mac);
        assert_eq!(array_packet.len(), 102);
        assert_eq!(&array_packet[..], packet.as_slice());
    }

    #[test]
    fn test_send_wol_loopback() {
        let rx_socket = UdpSocket::bind("127.0.0.1:0").expect("Must bind receiver socket");
        let rx_addr = rx_socket.local_addr().expect("Must get local addr");
        rx_socket
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("Must set timeout");

        let dest = format!("127.0.0.1:{}", rx_addr.port());
        let result = send_wol_to("00:11:22:33:44:55", &dest);
        assert!(result.is_ok(), "Sending WoL to loopback should succeed");

        let mut buf = [0u8; 200];
        let (amt, _src) = rx_socket.recv_from(&mut buf).expect("Must receive UDP packet");
        assert_eq!(amt, 102);
        assert_eq!(&buf[0..6], &[0xFF; 6]);
        assert_eq!(&buf[6..12], &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }
}
