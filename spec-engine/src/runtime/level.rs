// @impl:FR-LEVEL-01

use serde::{Deserialize, Serialize};

/// Engineer level — scope of competence, not model size or permission.
/// L1: function/component/implementation.
/// L2: module/subsystem/integration.
/// L3: architecture/system/strategic decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EngineerLevel {
    #[default]
    #[serde(rename = "L1")]
    L1,
    #[serde(rename = "L2")]
    L2,
    #[serde(rename = "L3")]
    L3,
}

impl EngineerLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineerLevel::L1 => "L1",
            EngineerLevel::L2 => "L2",
            EngineerLevel::L3 => "L3",
        }
    }

    pub fn scope(&self) -> &'static str {
        match self {
            EngineerLevel::L1 => "function, component, implementation",
            EngineerLevel::L2 => "module, subsystem, integration",
            EngineerLevel::L3 => "architecture, system, strategic decision",
        }
    }
}

impl std::fmt::Display for EngineerLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    // @test:FR-LEVEL-01
    use super::*;

    #[test]
    fn round_trip_serde() {
        for level in [EngineerLevel::L1, EngineerLevel::L2, EngineerLevel::L3] {
            let json = serde_json::to_string(&level).unwrap();
            let expected = format!("\"{}\"", level.as_str());
            assert_eq!(json, expected);
            let back: EngineerLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(back, level);
        }
    }

    #[test]
    fn default_is_l1() {
        assert_eq!(EngineerLevel::default(), EngineerLevel::L1);
    }

    #[test]
    fn scope_non_empty() {
        for level in [EngineerLevel::L1, EngineerLevel::L2, EngineerLevel::L3] {
            assert!(!level.scope().is_empty());
        }
    }
}
