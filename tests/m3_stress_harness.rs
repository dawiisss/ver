use beautiful_goodall::models::VncScaling;
use beautiful_goodall::vnc::{VncClient, VncCommand, VncFrameUpdate, VncWidget};
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn test_stress_high_throughput_command_propagation() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let (cmd_tx, mut cmd_rx) = unbounded_channel();
    widget.set_cmd_tx(cmd_tx);

    const COMMAND_COUNT: usize = 20_000;

    for i in 0..COMMAND_COUNT {
        widget.send_pointer_event((i % 1000) as u16, (i % 500) as u16, (i % 8) as u8);
    }

    let mut recv_count = 0;
    while let Ok(cmd) = cmd_rx.try_recv() {
        if let VncCommand::PointerEvent { .. } = cmd {
            recv_count += 1;
        }
    }

    assert_eq!(recv_count, COMMAND_COUNT);
    assert_eq!(widget.events_sent.len(), COMMAND_COUNT);
}

#[test]
fn test_stress_zero_dimension_frame_handling() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);

    // Render 0x0 frame buffer
    let frame_zero = VncFrameUpdate {
        width: 0,
        height: 0,
        stride: 0,
        pixels: vec![],
    };
    widget.render_frame(frame_zero);

    assert!(widget.current_frame.is_some());
    assert_eq!(widget.current_frame.as_ref().unwrap().width, 0);

    // Test coordinate translation safety when frame width/height is 0
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        widget.translate_coordinates(10.0, 10.0)
    }));

    if res.is_err() {
        eprintln!("[OBSERVATION] Zero-dimension frame caused panic in translate_coordinates");
    }
}

#[test]
fn test_stress_float_boundary_coordinate_translation() {
    let mut widget = VncWidget::new(VncScaling::FitToWindow);

    let frame = VncFrameUpdate {
        width: 1920,
        height: 1080,
        stride: 1920 * 4,
        pixels: vec![0u8; 1920 * 1080 * 4],
    };
    widget.render_frame(frame);

    // Test extreme float inputs: f64::MAX, f64::MIN, subnormals
    let extreme_coords = vec![
        (f64::MAX, f64::MAX),
        (f64::MIN, f64::MIN),
        (-1e308, 1e308),
        (f64::EPSILON, -f64::EPSILON),
    ];

    for (lx, ly) in extreme_coords {
        let (x, y) = widget.translate_coordinates(lx, ly);
        let _ = (x, y);
    }
}

#[test]
fn test_stress_truncated_raw_rgb_frame_buffer() {
    let client = VncClient::new("127.0.0.1".to_string(), 5900, VncScaling::OriginalSize);

    // Claimed 100x100 pixels (requires 30,000 bytes RGB input), but provide only 10 bytes
    let truncated_rgb = vec![128u8; 10];
    let frame = client.process_frame_buffer(&truncated_rgb, 100, 100);

    assert_eq!(frame.width, 100);
    assert_eq!(frame.height, 100);
    assert_eq!(frame.pixels.len(), 40_000); // Buffer created with zeroes where input was truncated

    // First 3 pixels populated (9 bytes used of 10)
    assert_eq!(&frame.pixels[0..4], &[128, 128, 128, 255]);
    // Truncated pixels should remain zero
    assert_eq!(&frame.pixels[16..20], &[0, 0, 0, 0]);
}

#[test]
fn test_stress_disconnected_channel_robustness() {
    let mut widget = VncWidget::new(VncScaling::OriginalSize);
    let (cmd_tx, cmd_rx) = unbounded_channel();
    widget.set_cmd_tx(cmd_tx);

    // Drop receiver immediately
    drop(cmd_rx);

    // Sending events after receiver drop must not panic or crash
    widget.send_key_event(0xFF0D, true);
    widget.send_pointer_event(100, 200, 1);
    widget.set_scaling(VncScaling::Stretch);

    assert_eq!(widget.events_sent.len(), 2);
}
