use ver::models::AdvancedSettings;

#[test]
fn test_serde() {
    let json = r#"{
        "vnc_viewonly": false,
        "vnc_clipboard": true
    }"#;
    let settings: AdvancedSettings = serde_json::from_str(json).unwrap();
    assert!(!settings.vnc_viewonly);
    assert!(settings.vnc_clipboard);
}
