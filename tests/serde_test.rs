use ver::models::AdvancedSettings;
#[test]
fn test_serde() {
    let json = r#"{
        "vnc_viewonly": false
    }"#;
    let settings: AdvancedSettings = serde_json::from_str(json).unwrap();
    println!("vnc_clipboard: {}", settings.vnc_clipboard);
}
