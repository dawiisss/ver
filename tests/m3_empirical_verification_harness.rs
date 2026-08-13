use beautiful_goodall::models::VncScaling;
use beautiful_goodall::vnc::{VncClient, VncCommand, VncFrameUpdate, VncWidget};
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_coordinate_translation_boundary_conditions() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);

    // Frame size 1000x500
    let frame = VncFrameUpdate {
        width: 1000,
        height: 500,
        stride: 4000,
        pixels: vec![0u8; 1000 * 500 * 4],
    };
    widget.render_frame(frame);

    // 1. OriginalSize: In-bounds clicks
    let (x, y) = widget.translate_coordinates(0.0, 0.0);
    assert_eq!((x, y), (0, 0));

    let (x, y) = widget.translate_coordinates(999.0, 499.0);
    assert_eq!((x, y), (999, 499));

    // 2. OriginalSize: Negative coordinates clamp to 0
    let (x, y) = widget.translate_coordinates(-100.0, -50.0);
    assert_eq!((x, y), (0, 0), "Negative coordinates must clamp to (0, 0)");

    // 3. Stretch Mode
    widget.set_scaling(VncScaling::Stretch);
    let (x, y) = widget.translate_coordinates(500.0, 250.0);
    assert_eq!((x, y), (500, 250));

    // 4. FitToWindow Mode
    widget.set_scaling(VncScaling::FitToWindow);
    let (x, y) = widget.translate_coordinates(250.0, 125.0);
    assert_eq!((x, y), (250, 125));
}

#[test]
fn test_coordinate_translation_aspect_ratio_letterboxing() {
    let mut widget = VncWidget::new(VncScaling::FitToWindow);

    // 16:9 Frame (1920x1080)
    let frame = VncFrameUpdate {
        width: 1920,
        height: 1080,
        stride: 1920 * 4,
        pixels: vec![0u8; 1920 * 1080 * 4],
    };
    widget.render_frame(frame);

    // Center click
    let (cx, cy) = widget.translate_coordinates(960.0, 540.0);
    assert_eq!((cx, cy), (960, 540));

    // Top-Left corner
    let (tlx, tly) = widget.translate_coordinates(0.0, 0.0);
    assert_eq!((tlx, tly), (0, 0));

    // Bottom-Right corner
    let (brx, bry) = widget.translate_coordinates(1919.0, 1079.0);
    assert_eq!((brx, bry), (1919, 1079));
}

#[test]
fn test_keysym_conversions_and_mapping() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let (cmd_tx, mut cmd_rx) = unbounded_channel();
    widget.set_cmd_tx(cmd_tx);

    // Comprehensive list of X11/GDK keysyms
    let test_keysyms: Vec<(u32, &str)> = vec![
        (0x0020, "Space"),
        (0x0041, "Latin_A"),
        (0x0061, "Latin_a"),
        (0x0031, "1"),
        (0xFF08, "BackSpace"),
        (0xFF09, "Tab"),
        (0xFF0D, "Return"),
        (0xFF1B, "Escape"),
        (0xFFFF, "Delete"),
        (0xFF51, "Left"),
        (0xFF52, "Up"),
        (0xFF53, "Right"),
        (0xFF54, "Down"),
        (0xFFBE, "F1"),
        (0xFFC9, "F12"),
        (0xFFE1, "Shift_L"),
        (0xFFE3, "Control_L"),
        (0xFFE9, "Alt_L"),
    ];

    for &(keysym, name) in &test_keysyms {
        // Send Key Down
        widget.send_key_event(keysym, true);
        let cmd_down = cmd_rx.try_recv().expect(&format!("Failed to receive key down for {}", name));
        match cmd_down {
            VncCommand::KeyEvent { keysym: k, down } => {
                assert_eq!(k, keysym);
                assert!(down);
            }
            _ => panic!("Expected KeyEvent for {}", name),
        }

        // Send Key Up
        widget.send_key_event(keysym, false);
        let cmd_up = cmd_rx.try_recv().expect(&format!("Failed to receive key up for {}", name));
        match cmd_up {
            VncCommand::KeyEvent { keysym: k, down } => {
                assert_eq!(k, keysym);
                assert!(!down);
            }
            _ => panic!("Expected KeyEvent for {}", name),
        }
    }

    assert_eq!(widget.events_sent.len(), test_keysyms.len() * 2);
}

#[test]
fn test_channel_command_buffer_propagation() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let (cmd_tx, mut cmd_rx) = unbounded_channel();
    widget.set_cmd_tx(cmd_tx);

    // Dispatch 100 interleaved commands
    for i in 0..100 {
        widget.send_key_event(0x0041 + (i % 26), i % 2 == 0);
        widget.send_pointer_event(i as u16 * 10, i as u16 * 5, (i % 8) as u8);
        if i % 10 == 0 {
            widget.set_scaling(VncScaling::FitToWindow);
        }
    }

    let mut key_count = 0;
    let mut pointer_count = 0;
    let mut scaling_count = 0;

    while let Ok(cmd) = cmd_rx.try_recv() {
        match cmd {
            VncCommand::KeyEvent { .. } => key_count += 1,
            VncCommand::PointerEvent { .. } => pointer_count += 1,
            VncCommand::SetScaling(_) => scaling_count += 1,
            _ => {}
        }
    }

    assert_eq!(key_count, 100);
    assert_eq!(pointer_count, 100);
    assert_eq!(scaling_count, 10);
}

#[test]
fn test_vnc_frame_buffer_rgb_to_b8g8r8x8_conversion() {
    let client = VncClient::new("127.0.0.1".to_string(), 5900, VncScaling::OriginalSize);

    // 3x2 image = 6 pixels = 18 bytes RGB input
    let raw_rgb = vec![
        255, 0, 0,     // Red
        0, 255, 0,     // Green
        0, 0, 255,     // Blue
        255, 255, 0,   // Yellow
        0, 255, 255,   // Cyan
        255, 0, 255,   // Magenta
    ];

    let frame = client.process_frame_buffer(&raw_rgb, 3, 2);
    assert_eq!(frame.width, 3);
    assert_eq!(frame.height, 2);
    assert_eq!(frame.stride, 12); // 3 * 4 bytes
    assert_eq!(frame.pixels.len(), 24); // 3 * 2 * 4 bytes

    // Pixel 0 (Red): B=0, G=0, R=255, X=255
    assert_eq!(&frame.pixels[0..4], &[0, 0, 255, 255]);
    // Pixel 1 (Green): B=0, G=255, R=0, X=255
    assert_eq!(&frame.pixels[4..8], &[0, 255, 0, 255]);
    // Pixel 2 (Blue): B=255, G=0, R=0, X=255
    assert_eq!(&frame.pixels[8..12], &[255, 0, 0, 255]);
    // Pixel 3 (Yellow): B=0, G=255, R=255, X=255
    assert_eq!(&frame.pixels[12..16], &[0, 255, 255, 255]);
}
