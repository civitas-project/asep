//! Configuration schemas, editor preferences, and TOML persistence.

pub mod settings;
pub mod budget;

pub use settings::{
    get_config_path, load_config, load_config_checked, save_config, CapabilityProfile, EditorPreference, EditorSettings,
    EngineConfig,
};
