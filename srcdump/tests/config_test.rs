//! Tests for srcdump configuration defaults.

use srcdump::config::Config;
use std::fs;
use std::io;
use std::path::PathBuf;

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

fn unique_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("config_test_{}_{}.toml", std::process::id(), nanos)
}

/// Temp TOML fixture removed on drop (panics included).
struct TempFile(PathBuf);

impl TempFile {
    fn create() -> io::Result<Self> {
        let path = std::env::temp_dir().join(unique_name());
        let _file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn test_from_file_valid_toml() {
    let temp = TempFile::create().expect("create temp fixture");
    let content = r#"
format = "json"
mode = "summary"
strip_comments = false
jobs = 4
summary_threshold = 500
tokenizer = "test_tokenizer"
verbose = true
include_ext = ["rs", "py"]
max_lines = 100
"#;
    fs::write(temp.path(), content).expect("write valid TOML fixture");
    let config = Config::from_file(temp.path()).expect("valid TOML should load");
    assert_eq!(config.format, srcdump::config::OutputFormat::Json);
    assert_eq!(config.mode, srcdump::config::OutputMode::Summary);
    assert!(!config.strip_comments);
    assert_eq!(config.jobs, 4);
    assert_eq!(config.summary_threshold, 500);
    assert_eq!(config.tokenizer, "test_tokenizer");
    assert!(config.verbose);
    assert_eq!(config.include_ext, vec!["rs".to_string(), "py".to_string()]);
    assert_eq!(config.max_lines, Some(100));
    assert!(!config.include_symbols);
    assert!(config.gitignore);
    assert!(!config.no_content);
}

#[test]
fn test_from_file_invalid_toml() {
    let temp = TempFile::create().expect("create temp fixture");
    fs::write(temp.path(), "this is not = valid toml [[[").expect("write invalid TOML fixture");
    let result = Config::from_file(temp.path());
    assert!(result.is_err(), "malformed TOML should be rejected");
    let err = result.unwrap_err();
    assert!(
        err.is::<toml::de::Error>(),
        "rejection should be a TOML parse error, got {}",
        err
    );
}
