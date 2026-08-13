//! Tier 5 White-Box Adversarial Coverage Hardening Test Suite
//! Focuses on UI (`src/ui/`) and VNC (`src/vnc/`) modules:
//! 1. Rapid scaling mode toggles (OriginalSize <-> FitToWindow <-> Stretch).
//! 2. Multi-threaded VNC tile decoding under malformed/truncated RFB packet streams.
//! 3. DiscoveryDialog subnet scanner port scanning timeouts and main loop channel dispatch.
//! 4. Theme toggling under GTK uninitialized / headless environments.

use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;
use vnc::{PixelFormat, Rect};

use beautiful_goodall::models::{AppConfig, Connection, Protocol, VncScaling};
use beautiful_goodall::ui::{
    apply_theme, ConnectionEditor, DiscoveredService, DiscoveryDialog, MainWindow, PreferencesWindow,
};
use beautiful_goodall::vnc::client::{copy_tile_raw, decode_tile_raw};
use beautiful_goodall::vnc::{VncCommand, VncFrameUpdate, VncWidget};

static GTK_TEST_LOCK: Mutex<()> = Mutex::new(());

// ============================================================================
// 1. Rapid Scaling Mode Toggles
// ============================================================================

#[test]
fn test_rapid_scaling_mode_toggles_without_channel() {
    let _guard = GTK_TEST_LOCK.lock().unwrap();
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    assert_eq!(widget.scaling, VncScaling::OriginalSize);

    let modes = [
        VncScaling::FitToWindow,
        VncScaling::Stretch,
        VncScaling::OriginalSize,
        VncScaling::Stretch,
        VncScaling::FitToWindow,
    ];

    // Rapidly toggle scaling modes 1000 times
    for i in 0..1000 {
        let mode = modes[i % modes.len()];
        widget.set_scaling(mode);
        assert_eq!(widget.scaling, mode);
    }
}

#[test]
fn test_rapid_scaling_mode_toggles_with_command_channel() {
    let _guard = GTK_TEST_LOCK.lock().unwrap();
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let (cmd_tx, mut cmd_rx) = unbounded_channel();
    widget.set_cmd_tx(cmd_tx);

    let modes = [
        VncScaling::FitToWindow,
        VncScaling::Stretch,
        VncScaling::OriginalSize,
    ];

    let toggle_count = 300;
    for i in 0..toggle_count {
        let mode = modes[i % modes.len()];
        widget.set_scaling(mode);
    }

    let mut received_count = 0;
    while let Ok(cmd) = cmd_rx.try_recv() {
        if let VncCommand::SetScaling(sc) = cmd {
            let expected_mode = modes[received_count % modes.len()];
            assert_eq!(sc, expected_mode);
            received_count += 1;
        }
    }
    assert_eq!(received_count, toggle_count);
}

#[test]
fn test_coordinate_translation_under_rapid_scaling_transitions() {
    let _guard = GTK_TEST_LOCK.lock().unwrap();
    let mut widget = VncWidget::new(VncScaling::OriginalSize);

    let frame = VncFrameUpdate {
        width: 1920,
        height: 1080,
        stride: 1920 * 4,
        pixels: vec![0u8; 1920 * 1080 * 4],
    };
    widget.render_frame(frame);

    for _ in 0..100 {
        // OriginalSize check
        widget.set_scaling(VncScaling::OriginalSize);
        assert_eq!(widget.translate_coordinates(500.0, 300.0), (500, 300));
        assert_eq!(widget.translate_coordinates(2000.0, 1500.0), (1919, 1079));

        // Stretch check
        widget.set_scaling(VncScaling::Stretch);
        assert_eq!(widget.translate_coordinates(960.0, 540.0), (960, 540));

        // FitToWindow check
        widget.set_scaling(VncScaling::FitToWindow);
        assert_eq!(widget.translate_coordinates(100.0, 100.0), (100, 100));
    }
}

#[test]
fn test_coordinate_translation_extreme_and_invalid_inputs() {
    let _guard = GTK_TEST_LOCK.lock().unwrap();
    let mut widget = VncWidget::new(VncScaling::OriginalSize);

    // No frame set -> returns (0, 0)
    assert_eq!(widget.translate_coordinates(100.0, 100.0), (0, 0));

    // Zero-dimension frame -> returns (0, 0)
    widget.render_frame(VncFrameUpdate {
        width: 0,
        height: 0,
        stride: 0,
        pixels: vec![],
    });
    assert_eq!(widget.translate_coordinates(50.0, 50.0), (0, 0));

    // Frame with width 1, height 1
    widget.render_frame(VncFrameUpdate {
        width: 1,
        height: 1,
        stride: 4,
        pixels: vec![0; 4],
    });

    widget.set_scaling(VncScaling::OriginalSize);
    assert_eq!(widget.translate_coordinates(0.0, 0.0), (0, 0));
    assert_eq!(widget.translate_coordinates(1000.0, 1000.0), (0, 0));

    widget.set_scaling(VncScaling::Stretch);
    assert_eq!(widget.translate_coordinates(500.0, 500.0), (0, 0));

    widget.set_scaling(VncScaling::FitToWindow);
    assert_eq!(widget.translate_coordinates(-100.0, -100.0), (0, 0));
}

// ============================================================================
// 2. Multi-Threaded VNC Tile Decoding Under Malformed/Truncated Packets
// ============================================================================

#[test]
fn test_decode_tile_32bit_malformed_and_truncated_streams() {
    let width = 10u32;
    let height = 10u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    let fmt_be = PixelFormat {
        bits_per_pixel: 32,
        depth: 24,
        big_endian: true,
        true_colour: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };

    let fmt_le = PixelFormat {
        bits_per_pixel: 32,
        depth: 24,
        big_endian: false,
        true_colour: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };

    // 1. Fully truncated (empty tile bytes) -> should not panic
    let rect = Rect { left: 0, top: 0, width: 5, height: 5 };
    decode_tile_raw(&mut buffer, width, height, &rect, &[], &fmt_be);

    // 2. Partial pixel (3 bytes for a 4-bpp format) -> should skip incomplete pixels gracefully
    let partial_tile = vec![0xFF, 0x00, 0xAA];
    decode_tile_raw(&mut buffer, width, height, &rect, &partial_tile, &fmt_be);

    // 3. Valid 1 pixel input (4 bytes BE: Red=255, Green=128, Blue=64)
    let single_pixel = vec![0, 255, 128, 64];
    let rect_1x1 = Rect { left: 2, top: 2, width: 1, height: 1 };
    decode_tile_raw(&mut buffer, width, height, &rect_1x1, &single_pixel, &fmt_be);

    let dst_idx = (2 * width + 2) as usize * 4;
    assert_eq!(buffer[dst_idx], 64);   // B
    assert_eq!(buffer[dst_idx + 1], 128); // G
    assert_eq!(buffer[dst_idx + 2], 255); // R
    assert_eq!(buffer[dst_idx + 3], 255); // A

    // 4. Little endian decode
    decode_tile_raw(&mut buffer, width, height, &rect_1x1, &single_pixel, &fmt_le);

    // 5. Zero red/green/blue_max (divide by zero protection check)
    let zero_max_fmt = PixelFormat {
        bits_per_pixel: 32,
        depth: 24,
        big_endian: false,
        true_colour: true,
        red_max: 0,
        green_max: 0,
        blue_max: 0,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };
    decode_tile_raw(&mut buffer, width, height, &rect_1x1, &single_pixel, &zero_max_fmt);

    // 6. Out-of-bounds rect (left/top/width/height exceeding framebuffer boundaries)
    let oob_rect = Rect { left: 8, top: 8, width: 10, height: 10 };
    let tile_data = vec![128u8; 100 * 4];
    decode_tile_raw(&mut buffer, width, height, &oob_rect, &tile_data, &fmt_be);
}

#[test]
fn test_decode_tile_16bit_and_24bit_formats() {
    let width = 8u32;
    let height = 8u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    // 24-bit RGB format (3 bpp)
    let fmt_24 = PixelFormat {
        bits_per_pixel: 24,
        depth: 24,
        big_endian: false,
        true_colour: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };

    let tile_24 = vec![
        255, 0, 0, // P0: Red
        0, 255, 0, // P1: Green
    ];

    let rect = Rect { left: 0, top: 0, width: 2, height: 1 };
    decode_tile_raw(&mut buffer, width, height, &rect, &tile_24, &fmt_24);

    assert_eq!(&buffer[0..4], &[0, 0, 255, 255]);   // P0: Red in BGRA
    assert_eq!(&buffer[4..8], &[0, 255, 0, 255]);   // P1: Green in BGRA

    // 16-bit RGB 565 format (2 bpp)
    let fmt_16 = PixelFormat {
        bits_per_pixel: 16,
        depth: 16,
        big_endian: false,
        true_colour: true,
        red_max: 31,
        green_max: 63,
        blue_max: 31,
        red_shift: 11,
        green_shift: 5,
        blue_shift: 0,
    };

    let tile_16 = vec![0x00, 0xF8]; // Pure Red in 565 LE: 0b1111100000000000
    decode_tile_raw(&mut buffer, width, height, &rect, &tile_16, &fmt_16);

    // Unsupported BPP (e.g. 8-bit or 0-bit) -> gracefully no-op
    let fmt_0 = PixelFormat {
        bits_per_pixel: 0,
        depth: 0,
        big_endian: false,
        true_colour: false,
        red_max: 0,
        green_max: 0,
        blue_max: 0,
        red_shift: 0,
        green_shift: 0,
        blue_shift: 0,
    };
    decode_tile_raw(&mut buffer, width, height, &rect, &tile_16, &fmt_0);
}

#[test]
fn test_copy_tile_out_of_bounds_and_overlapping() {
    let width = 4u32;
    let height = 4u32;
    let mut buffer = vec![
        1, 1, 1, 255,  2, 2, 2, 255,  3, 3, 3, 255,  4, 4, 4, 255,
        5, 5, 5, 255,  6, 6, 6, 255,  7, 7, 7, 255,  8, 8, 8, 255,
        9, 9, 9, 255, 10,10,10, 255, 11,11,11, 255, 12,12,12, 255,
       13,13,13, 255, 14,14,14, 255, 15,15,15, 255, 16,16,16, 255,
    ];

    // 1. Copy with src out of bounds -> no panic
    let oob_src = Rect { left: 10, top: 10, width: 2, height: 2 };
    let dst = Rect { left: 0, top: 0, width: 2, height: 2 };
    copy_tile_raw(&mut buffer, width, height, &oob_src, &dst);

    // 2. Copy with dst out of bounds -> no panic
    let src = Rect { left: 0, top: 0, width: 2, height: 2 };
    let oob_dst = Rect { left: 3, top: 3, width: 2, height: 2 };
    copy_tile_raw(&mut buffer, width, height, &src, &oob_dst);

    // 3. Overlapping copy: top-left to bottom-right
    let src_overlap = Rect { left: 0, top: 0, width: 2, height: 2 };
    let dst_overlap = Rect { left: 1, top: 1, width: 2, height: 2 };
    copy_tile_raw(&mut buffer, width, height, &src_overlap, &dst_overlap);
}

#[test]
fn test_multithreaded_vnc_tile_decoding_stress() {
    let thread_count = 16;
    let iterations_per_thread = 500;
    let barrier = Arc::new(Barrier::new(thread_count));

    let mut handles = Vec::new();

    for t in 0..thread_count {
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();

            let width = 32u32;
            let height = 32u32;
            let mut backing_buffer = vec![0u8; (width * height * 4) as usize];

            let fmt_32 = PixelFormat {
                bits_per_pixel: 32,
                depth: 24,
                big_endian: t % 2 == 0,
                true_colour: true,
                red_max: 255,
                green_max: 255,
                blue_max: 255,
                red_shift: 16,
                green_shift: 8,
                blue_shift: 0,
            };

            for i in 0..iterations_per_thread {
                // Generate semi-random tile bytes and dimensions
                let tile_len = (i * 7) % 50;
                let tile_bytes = vec![(t + i) as u8; tile_len];
                let rect = Rect {
                    left: ((i * 3) % 40) as u16,
                    top: ((i * 5) % 40) as u16,
                    width: (i % 16 + 1) as u16,
                    height: (i % 16 + 1) as u16,
                };

                decode_tile_raw(
                    &mut backing_buffer,
                    width,
                    height,
                    &rect,
                    &tile_bytes,
                    &fmt_32,
                );

                if i % 10 == 0 {
                    let src = Rect { left: 0, top: 0, width: 8, height: 8 };
                    let dst = Rect { left: 4, top: 4, width: 8, height: 8 };
                    copy_tile_raw(&mut backing_buffer, width, height, &src, &dst);
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Worker thread panicked during tile decoding stress");
    }
}

// ============================================================================
// 3. DiscoveryDialog Subnet Scanner & Channel Dispatch
// ============================================================================

#[test]
fn test_discovery_dialog_service_management() {
    let mut dialog = DiscoveryDialog::new();
    assert!(dialog.discovered_services.is_empty());

    let s1 = DiscoveredService {
        name: "Host VNC".to_string(),
        protocol: "vnc".to_string(),
        host: "192.168.1.100".to_string(),
        port: 5900,
    };

    let s2 = DiscoveredService {
        name: "Host SSH".to_string(),
        protocol: "ssh".to_string(),
        host: "192.168.1.101".to_string(),
        port: 22,
    };

    let s3 = DiscoveredService {
        name: "Host RDP".to_string(),
        protocol: "rdp".to_string(),
        host: "192.168.1.102".to_string(),
        port: 3389,
    };

    dialog.add_service(s1.clone());
    dialog.add_service(s2.clone());
    dialog.add_service(s3.clone());

    assert_eq!(dialog.discovered_services.len(), 3);
    assert_eq!(dialog.discovered_services[0].protocol, "vnc");
    assert_eq!(dialog.discovered_services[1].protocol, "ssh");
    assert_eq!(dialog.discovered_services[2].protocol, "rdp");
}

#[test]
fn test_subnet_scanner_port_connect_timeouts() {
    // Probe non-routable documentation IP range (192.0.2.1) on closed ports with timeout
    use std::net::{IpAddr, SocketAddr, TcpStream};

    let test_addr: IpAddr = "192.0.2.1".parse().unwrap();
    let ports = [5900, 3389, 22];

    for &port in &ports {
        let addr = SocketAddr::new(test_addr, port);
        let start = std::time::Instant::now();
        let res = TcpStream::connect_timeout(&addr, Duration::from_millis(50));
        let elapsed = start.elapsed();

        assert!(res.is_err());
        // Verify timeout respected bounded duration
        assert!(elapsed < Duration::from_secs(2));
    }
}

#[test]
fn test_discovered_service_channel_dispatch_simulation() {
    let (tx, rx) = std::sync::mpsc::channel::<Option<DiscoveredService>>();

    let handle = thread::spawn(move || {
        let services = vec![
            DiscoveredService {
                name: "VNC Server".into(),
                protocol: "vnc".into(),
                host: "127.0.0.1".into(),
                port: 5900,
            },
            DiscoveredService {
                name: "SSH Server".into(),
                protocol: "ssh".into(),
                host: "127.0.0.1".into(),
                port: 22,
            },
        ];

        for s in services {
            let _ = tx.send(Some(s));
        }
        let _ = tx.send(None); // Scan finished signal
    });

    let mut received = Vec::new();
    let mut finished = false;

    while let Ok(msg) = rx.recv() {
        match msg {
            Some(s) => received.push(s),
            None => {
                finished = true;
                break;
            }
        }
    }

    handle.join().unwrap();
    assert!(finished);
    assert_eq!(received.len(), 2);
    assert_eq!(received[0].protocol, "vnc");
    assert_eq!(received[1].protocol, "ssh");
}

// ============================================================================
// 4. Theme Toggling Under GTK Uninitialized / Headless Environments
// ============================================================================

#[test]
fn test_theme_toggling_in_headless_environment() {
    let _guard = GTK_TEST_LOCK.lock().unwrap();
    // `apply_theme` must check `gtk::is_initialized()` and return cleanly without panicking
    apply_theme("dark");
    apply_theme("light");
    apply_theme("system");
    apply_theme("default");
    apply_theme("INVALID_THEME_STRING");
    apply_theme("");

    let config = AppConfig::default();
    let mut pref_win = PreferencesWindow::new(config.clone());
    pref_win.set_theme("dark");
    assert_eq!(pref_win.config.theme, "dark");

    pref_win.set_theme("light");
    assert_eq!(pref_win.config.theme, "light");

    let mut main_win = MainWindow::new(vec![], config);
    main_win.set_theme("dark");
    assert_eq!(main_win.config.theme, "dark");
}

#[test]
fn test_multithreaded_theme_toggling_headless_stress() {
    use std::sync::Mutex;
    static THEME_LOCK: Mutex<()> = Mutex::new(());
    let threads = 10;
    let mut handles = Vec::new();

    for t in 0..threads {
        handles.push(thread::spawn(move || {
            let themes = ["dark", "light", "system", "custom", "DARK", "LIGHT"];
            for i in 0..200 {
                let theme = themes[(t + i) % themes.len()];
                let _guard = THEME_LOCK.lock().unwrap();
                apply_theme(theme);
            }
        }));
    }

    for h in handles {
        assert!(h.join().is_ok(), "Thread panicked during multithreaded apply_theme stress test");
    }
}

// ============================================================================
// 5. Connection Editor & Main Window Edge Cases
// ============================================================================

#[test]
fn test_connection_editor_validation_boundary_conditions() {
    let mut conn = Connection::default();
    conn.name = "".to_string();
    conn.host = "192.168.1.1".to_string();
    conn.port = 5900;

    let editor = ConnectionEditor::new(conn.clone(), "pass".to_string());
    assert!(editor.validate().is_err()); // Empty name

    conn.name = "Valid Name".to_string();
    conn.host = "   ".to_string();
    let editor2 = ConnectionEditor::new(conn.clone(), "pass".to_string());
    assert!(editor2.validate().is_err()); // Empty host

    conn.host = "192.168.1.1".to_string();
    conn.port = 0;
    let editor3 = ConnectionEditor::new(conn.clone(), "pass".to_string());
    assert!(editor3.validate().is_err()); // Port 0

    conn.port = 5900;
    conn.mac_address = "invalid-mac-str".to_string();
    let editor4 = ConnectionEditor::new(conn.clone(), "pass".to_string());
    assert!(editor4.validate().is_err()); // Invalid MAC

    conn.mac_address = "00:11:22:33:44:55".to_string();
    let editor5 = ConnectionEditor::new(conn.clone(), "pass".to_string());
    assert!(editor5.validate().is_ok()); // Valid
}

#[test]
fn test_main_window_search_and_grouping_edge_cases() {
    let mut c1 = Connection::default();
    c1.name = "Alpha Host".to_string();
    c1.host = "10.0.0.1".to_string();
    c1.group = "Group A".to_string();
    c1.protocol = Protocol::Vnc;

    let mut c2 = Connection::default();
    c2.name = "Beta Host".to_string();
    c2.host = "10.0.0.2".to_string();
    c2.group = "Group B".to_string();
    c2.protocol = Protocol::Ssh;

    let mut main_win = MainWindow::new(vec![c1, c2], AppConfig::default());

    // Search query matching protocol
    main_win.set_search_filter("vnc");
    assert_eq!(main_win.filtered_connections().len(), 1);
    assert_eq!(main_win.filtered_connections()[0].name, "Alpha Host");

    // Search query matching group
    main_win.set_search_filter("Group B");
    assert_eq!(main_win.filtered_connections().len(), 1);
    assert_eq!(main_win.filtered_connections()[0].name, "Beta Host");

    // Non-matching search query
    main_win.set_search_filter("NonExistentHost");
    assert_eq!(main_win.filtered_connections().len(), 0);

    // Clear filter
    main_win.set_search_filter("");
    assert_eq!(main_win.filtered_connections().len(), 2);
}
