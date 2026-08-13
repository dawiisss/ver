use beautiful_goodall::models::VncScaling;
use beautiful_goodall::vnc::{VncClient, VncEvent, VncFrameUpdate, VncWidget};

#[test]
fn test_vnc_frame_buffer_processing_rgb_to_b8g8r8x8() {
    let client = VncClient::new("127.0.0.1".to_string(), 5900, VncScaling::OriginalSize);

    // 2x2 test image in RGB format (12 bytes: 4 pixels, 3 bytes per pixel)
    // Pixel 0: R=255, G=0, B=0 (Red)
    // Pixel 1: R=0, G=255, B=0 (Green)
    // Pixel 2: R=0, G=0, B=255 (Blue)
    // Pixel 3: R=255, G=255, B=255 (White)
    let raw_rgb = vec![
        255, 0, 0,
        0, 255, 0,
        0, 0, 255,
        255, 255, 255,
    ];

    let frame = client.process_frame_buffer(&raw_rgb, 2, 2);
    assert_eq!(frame.width, 2);
    assert_eq!(frame.height, 2);
    assert_eq!(frame.stride, 8); // 2 pixels * 4 bytes
    assert_eq!(frame.pixels.len(), 16); // 2 * 2 * 4

    // Pixel 0 in B8G8R8X8: B=0, G=0, R=255, X=255
    assert_eq!(frame.pixels[0], 0);   // B
    assert_eq!(frame.pixels[1], 0);   // G
    assert_eq!(frame.pixels[2], 255); // R
    assert_eq!(frame.pixels[3], 255); // X

    // Pixel 2 in B8G8R8X8: B=255, G=0, R=0, X=255
    assert_eq!(frame.pixels[8], 255); // B
    assert_eq!(frame.pixels[9], 0);   // G
    assert_eq!(frame.pixels[10], 0);  // R
    assert_eq!(frame.pixels[11], 255); // X
}

#[test]
fn test_vnc_widget_render_frame_and_events() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    assert!(widget.current_frame.is_none());

    let frame = VncFrameUpdate {
        width: 1024,
        height: 768,
        stride: 4096,
        pixels: vec![0u8; 1024 * 768 * 4],
    };

    widget.render_frame(frame);
    assert!(widget.current_frame.is_some());
    assert_eq!(widget.current_frame.as_ref().unwrap().width, 1024);

    // Send Key Event (e.g. Enter key down and up)
    widget.send_key_event(0xFF0D, true);  // Enter down
    widget.send_key_event(0xFF0D, false); // Enter up

    // Send Pointer Event (mouse motion to x=100, y=200 with left button mask 1)
    widget.send_pointer_event(100, 200, 1);

    assert_eq!(widget.events_sent.len(), 3);
    assert_eq!(
        widget.events_sent[0],
        VncEvent::Key { keysym: 0xFF0D, down: true }
    );
    assert_eq!(
        widget.events_sent[1],
        VncEvent::Key { keysym: 0xFF0D, down: false }
    );
    assert_eq!(
        widget.events_sent[2],
        VncEvent::Pointer { x: 100, y: 200, mask: 1 }
    );
}

#[test]
fn test_vnc_scaling_switches() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    assert_eq!(widget.scaling, VncScaling::OriginalSize);

    widget.set_scaling(VncScaling::FitToWindow);
    assert_eq!(widget.scaling, VncScaling::FitToWindow);

    widget.set_scaling(VncScaling::Stretch);
    assert_eq!(widget.scaling, VncScaling::Stretch);
}

#[test]
fn test_vnc_coordinate_translation_modes() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);

    // Frame size 1000x500
    let frame = VncFrameUpdate {
        width: 1000,
        height: 500,
        stride: 4000,
        pixels: vec![0u8; 1000 * 500 * 4],
    };
    widget.render_frame(frame);

    // Original Size: (local_x, local_y) clamped to [0, fw-1] x [0, fh-1]
    let (x, y) = widget.translate_coordinates(250.0, 150.0);
    assert_eq!((x, y), (250, 150));

    // Stretch Mode
    widget.set_scaling(VncScaling::Stretch);
    let (x_stretch, y_stretch) = widget.translate_coordinates(500.0, 250.0);
    assert_eq!((x_stretch, y_stretch), (500, 250));

    // Fit To Window Mode
    widget.set_scaling(VncScaling::FitToWindow);
    let (x_fit, y_fit) = widget.translate_coordinates(100.0, 50.0);
    assert_eq!((x_fit, y_fit), (100, 50));
}

#[test]
fn test_vnc_widget_command_channel_integration() {
    use beautiful_goodall::vnc::VncCommand;
    use tokio::sync::mpsc::unbounded_channel;

    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let (cmd_tx, mut cmd_rx) = unbounded_channel();
    widget.set_cmd_tx(cmd_tx);

    widget.send_key_event(0xFF0D, true);
    widget.send_pointer_event(300, 400, 1);
    widget.set_scaling(VncScaling::FitToWindow);

    let cmd1 = cmd_rx.try_recv().unwrap();
    match cmd1 {
        VncCommand::KeyEvent { keysym, down } => {
            assert_eq!(keysym, 0xFF0D);
            assert!(down);
        }
        _ => panic!("Expected KeyEvent"),
    }

    let cmd2 = cmd_rx.try_recv().unwrap();
    match cmd2 {
        VncCommand::PointerEvent { x, y, mask } => {
            assert_eq!((x, y, mask), (300, 400, 1));
        }
        _ => panic!("Expected PointerEvent"),
    }

    let cmd3 = cmd_rx.try_recv().unwrap();
    match cmd3 {
        VncCommand::SetScaling(scaling) => {
            assert_eq!(scaling, VncScaling::FitToWindow);
        }
        _ => panic!("Expected SetScaling"),
    }
}

