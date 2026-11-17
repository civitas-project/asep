//! Configuration schemas, editor preferences, and TOML persistence.

pub mod settings;

pub use settings::{
    get_config_path, load_config, save_config, EditorPreference, EditorSettings, EngineConfig,
};
