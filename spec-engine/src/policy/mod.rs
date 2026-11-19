//! Canonical ASEP policy module.
//!
//! Reads `spec-engine/policy/asep-policy.toml` (embedded via `include_str!`)
//! into `Policy` with `toml::from_str`. That TOML is the only place edge
//! kinds, coverage rules COV-01..COV-06, rigor tiers T0..T4, and exit codes
//! are defined.

use serde::Deserialize;

/// Typed traceability edge kind.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum EdgeKind {
    Satisfies,
    Verifies,
    Informs,
    Constrains,
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

/// Rule severity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
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
}

/// Loads the canonical policy from the embedded TOML.
pub fn load_policy() -> Policy {
    toml::from_str(include_str!("../../policy/asep-policy.toml"))
        .expect("spec-engine/policy/asep-policy.toml is valid TOML")
}
