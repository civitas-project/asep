//! Tests for srcdump configuration defaults.

use srcdump::config::Config;

#[test]
fn test_default_include_ext_includes_tex_and_latex() {
    let config = Config::default();
    assert!(
        config.include_ext.iter().any(|e| e == "tex"),
        "default include_ext should include 'tex'"
    );
    assert!(
        config.include_ext.iter().any(|e| e == "latex"),
        "default include_ext should include 'latex'"
    );
}
