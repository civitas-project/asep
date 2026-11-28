//! Canonical ASEP policy module.
//!
//! Reads `spec-engine/policy/asep-policy.toml` (embedded via `include_str!`)
//! into `Policy` with `toml::from_str`. That TOML is the only place edge
//! kinds, coverage rules COV-01..COV-06, rigor tiers T0..T4, and exit codes
//! are defined.

use serde::{Deserialize, Serialize};

/// Typed traceability edge kind.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EdgeKind {
    Satisfies,
    Verifies,
    Informs,
    Constrains,
}

/// Maps the three documented engineering rigor levels to the canonical tiers.
/// Lightweight keeps T0/T1 semantics (no mandatory semantic evidence),
/// Structured requires T2, High Assurance requires T3/T4. Callers should use
/// `TierId::from_str` for explicit tier overrides; this mapping avoids a
/// silently weaker requirement when only a level name is supplied.
pub fn tier_for_rigor_level(level: &str) -> Option<TierId> {
    match level.trim().to_ascii_lowercase().as_str() {
        "lightweight" | "light" => Some(TierId::T1),
        "structured" | "medium" => Some(TierId::T2),
        "high assurance" | "high_assurance" | "high" => Some(TierId::T4),
        _ => None,
    }
}

/// Rigor tier.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum TierId {
    T0,
    T1,
    T2,
    T3,
    T4,
}

impl TierId {
    pub fn as_str(self) -> &'static str {
        match self {
            TierId::T0 => "T0",
            TierId::T1 => "T1",
            TierId::T2 => "T2",
            TierId::T3 => "T3",
            TierId::T4 => "T4",
        }
    }
}

impl std::str::FromStr for TierId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "T0" => Ok(TierId::T0),
            "T1" => Ok(TierId::T1),
            "T2" => Ok(TierId::T2),
            "T3" => Ok(TierId::T3),
            "T4" => Ok(TierId::T4),
            other => Err(format!("invalid rigor tier '{other}'; expected T0..T4")),
        }
    }
}

/// Rule severity.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Canonical coverage rule.
#[derive(Clone, Debug, Deserialize)]
pub struct CoverageRule {
    pub id: String,
    pub min: usize,
    pub severity: Severity,
}

/// Rigor tier declaration.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Tier {
    pub id: TierId,
    pub semantic_evidence_required: bool,
}

/// Anti-slop rule with a regex-ready pattern.
#[derive(Clone, Debug, Deserialize)]
pub struct AntiSlopRule {
    pub id: String,
    pub domain: String,
    pub severity: Severity,
    pub pattern: String,
    pub message: String,
    #[serde(default)]
    pub max_nesting: Option<usize>,
}

/// Exit codes from the global constraints.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ExitCodes {
    pub pass: u8,
    pub fail: u8,
    pub config_io: u8,
    pub unproven: u8,
}

/// Canonical policy loaded from `spec-engine/policy/asep-policy.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct Policy {
    pub edge_kinds: Vec<EdgeKind>,
    pub coverage_rules: Vec<CoverageRule>,
    pub tiers: Vec<Tier>,
    pub exit_codes: ExitCodes,
    #[serde(default)]
    pub antislop_rules: Vec<AntiSlopRule>,
}

impl Policy {
    pub fn coverage_rules(&self) -> &[CoverageRule] {
        &self.coverage_rules
    }

    pub fn tier(&self, id: TierId) -> &Tier {
        self.tiers
            .iter()
            .find(|tier| tier.id == id)
            .expect("policy TOML must declare every rigor tier")
    }

    pub fn exit_codes(&self) -> &ExitCodes {
        &self.exit_codes
    }

    pub fn antislop_rules(&self) -> &[AntiSlopRule] {
        &self.antislop_rules
    }
}

/// Loads the canonical policy from the embedded TOML.
pub fn load_policy() -> Policy {
    toml::from_str(include_str!("../../policy/asep-policy.toml"))
        .expect("spec-engine/policy/asep-policy.toml is valid TOML")
}
