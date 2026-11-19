//! Skill data models and status representations.

use std::path::PathBuf;

// @impl:FR-SKILL-INSPECT-01-B

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillStatus {
    Builtin,
    Modified,
    Custom,
    Missing,
}

impl std::fmt::Display for SkillStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillStatus::Builtin => write!(f, "Builtin (original)"),
            SkillStatus::Modified => write!(f, "Modified (customized)"),
            SkillStatus::Custom => write!(f, "Custom (user-created)"),
            SkillStatus::Missing => write!(f, "Missing (can be restored)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file_path: PathBuf,
    pub status: SkillStatus,
    pub version: Option<String>,
    pub depends: Vec<String>,
}
