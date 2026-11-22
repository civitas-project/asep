//! Offline acceptance evaluation for a single tested snapshot.
//!
//! Pure fail-closed evaluation. The trusted runner records the snapshot
//! identity and per-gate evidence (command, exit code, skip flag) and passes
//! it here. This module performs no IO, spawns no processes, reads no files,
//! and never authenticates JSON from the workspace. Any guard-clause failure
//! rejects; there is no public boolean trust boundary.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Expected acceptance schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// Fixed, ordered set of mandatory acceptance gate IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcceptanceGate {
    Formal,
    Structural,
    TypeCheck,
    Clippy,
    Tests,
    Format,
    BenchPreflight,
}

/// Mandatory gates in the exact required order.
pub const MANDATORY_GATES: [AcceptanceGate; 7] = [
    AcceptanceGate::Formal,
    AcceptanceGate::Structural,
    AcceptanceGate::TypeCheck,
    AcceptanceGate::Clippy,
    AcceptanceGate::Tests,
    AcceptanceGate::Format,
    AcceptanceGate::BenchPreflight,
];

/// Exact approved identity bound to one tested snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotIdentity {
    pub commit_sha: String,
    pub snapshot_sha256: String,
    pub policy_sha256: String,
    pub runner_id: String,
    pub toolchain_id: String,
    pub schema_version: u32,
}

/// Per-gate evidence recorded by the trusted runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidence {
    pub gate: AcceptanceGate,
    pub command: Vec<String>,
    pub exit: Option<i32>,
    pub skipped: bool,
}

/// Evidence bundle: observed identity plus ordered gate evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceEvidence {
    pub identity: SnapshotIdentity,
    pub gates: Vec<GateEvidence>,
}

/// Fail-closed evaluation outcome. No public boolean trust boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceVerdict {
    Accepted,
    Rejected(AcceptanceRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceRejection {
    InvalidIdentity {
        field: &'static str,
        detail: String,
    },
    IdentityMismatch,
    GatesEmpty,
    GateCountMismatch {
        expected: usize,
        actual: usize,
    },
    GateSequenceMismatch {
        position: usize,
        expected: AcceptanceGate,
        actual: AcceptanceGate,
    },
    GateCommandMissing {
        gate: AcceptanceGate,
    },
    GateExitNotRecorded {
        gate: AcceptanceGate,
    },
    GateFailed {
        gate: AcceptanceGate,
        exit: i32,
    },
    GateSkipped {
        gate: AcceptanceGate,
    },
}

impl SnapshotIdentity {
    /// Recorded constraints: nonempty commit_sha, snapshot_sha256,
    /// policy_sha256, runner_id, and toolchain_id; commit SHA is 40 or 64
    /// lowercase hex digits; snapshot and policy digests are 64 lowercase
    /// hex digits; schema version equals SCHEMA_VERSION.
    pub fn validate(&self) -> Result<(), AcceptanceRejection> {
        present(&self.commit_sha, "commit_sha")?;
        require_commit_sha(&self.commit_sha)?;
        present(&self.snapshot_sha256, "snapshot_sha256")?;
        require_digest(&self.snapshot_sha256, "snapshot_sha256")?;
        present(&self.policy_sha256, "policy_sha256")?;
        require_digest(&self.policy_sha256, "policy_sha256")?;
        present(&self.runner_id, "runner_id")?;
        present(&self.toolchain_id, "toolchain_id")?;
        if self.schema_version != SCHEMA_VERSION {
            return Err(AcceptanceRejection::InvalidIdentity {
                field: "schema_version",
                detail: format!("expected {}", SCHEMA_VERSION),
            });
        }
        Ok(())
    }
}

fn present(value: &str, field: &'static str) -> Result<(), AcceptanceRejection> {
    if value.trim().is_empty() {
        return Err(AcceptanceRejection::InvalidIdentity {
            field,
            detail: "must be nonempty".to_string(),
        });
    }
    Ok(())
}

fn require_commit_sha(value: &str) -> Result<(), AcceptanceRejection> {
    let len = value.len();
    if !(len == 40 || len == 64) {
        return Err(AcceptanceRejection::InvalidIdentity {
            field: "commit_sha",
            detail: "must be full lowercase 40 or 64 hex".to_string(),
        });
    }
    if !value.bytes().all(is_lowercase_hex) {
        return Err(AcceptanceRejection::InvalidIdentity {
            field: "commit_sha",
            detail: "must be full lowercase hex".to_string(),
        });
    }
    Ok(())
}

fn require_digest(value: &str, field: &'static str) -> Result<(), AcceptanceRejection> {
    if value.len() != 64 {
        return Err(AcceptanceRejection::InvalidIdentity {
            field,
            detail: "must be 64 lowercase hex".to_string(),
        });
    }
    if !value.bytes().all(is_lowercase_hex) {
        return Err(AcceptanceRejection::InvalidIdentity {
            field,
            detail: "must be lowercase hex".to_string(),
        });
    }
    Ok(())
}

fn is_lowercase_hex(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'a'..=b'f')
}

/// Pure fail-closed evaluation.
///
/// `approved` is the reference identity accepted by policy. `evidence` is the
/// runner-recorded bundle; its identity must match `approved` exactly and its
/// gates must match `MANDATORY_GATES` in order, each with a recorded command,
/// a recorded zero exit code, and no skips.
pub fn evaluate_acceptance(
    approved: &SnapshotIdentity,
    evidence: &AcceptanceEvidence,
) -> AcceptanceVerdict {
    if let Err(reason) = approved.validate() {
        return AcceptanceVerdict::Rejected(reason);
    }
    if evidence.identity != *approved {
        return AcceptanceVerdict::Rejected(AcceptanceRejection::IdentityMismatch);
    }
    let gates = &evidence.gates;
    if gates.is_empty() {
        return AcceptanceVerdict::Rejected(AcceptanceRejection::GatesEmpty);
    }
    if gates.len() != MANDATORY_GATES.len() {
        return AcceptanceVerdict::Rejected(AcceptanceRejection::GateCountMismatch {
            expected: MANDATORY_GATES.len(),
            actual: gates.len(),
        });
    }
    for (position, (gate_evidence, expected_gate)) in
        gates.iter().zip(MANDATORY_GATES.iter()).enumerate()
    {
        if gate_evidence.gate != *expected_gate {
            return AcceptanceVerdict::Rejected(AcceptanceRejection::GateSequenceMismatch {
                position,
                expected: *expected_gate,
                actual: gate_evidence.gate,
            });
        }
    }
    for gate_evidence in gates {
        let gate = gate_evidence.gate;
        if gate_evidence.command.is_empty() {
            return AcceptanceVerdict::Rejected(AcceptanceRejection::GateCommandMissing { gate });
        }
        let exit = match gate_evidence.exit {
            Some(exit) => exit,
            None => {
                return AcceptanceVerdict::Rejected(AcceptanceRejection::GateExitNotRecorded {
                    gate,
                })
            }
        };
        if exit != 0 {
            return AcceptanceVerdict::Rejected(AcceptanceRejection::GateFailed { gate, exit });
        }
        if gate_evidence.skipped {
            return AcceptanceVerdict::Rejected(AcceptanceRejection::GateSkipped { gate });
        }
    }
    AcceptanceVerdict::Accepted
}

#[cfg(target_os = "linux")]
pub mod runner;

#[cfg(not(target_os = "linux"))]
pub mod runner {
    use std::path::PathBuf;

    /// Stub runner for non-Linux hosts. Acceptance provisioning is
    /// Linux-only; fail closed without installing or provisioning anything.
    pub fn accept_commit(commit: &str) -> Result<PathBuf, String> {
        Err(format!("accept_commit unsupported on this OS: {commit}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_sha() -> String {
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string()
    }

    fn digest() -> String {
        "a1b2c3d4e5f6a1b2".repeat(4)
    }

    fn approved_identity() -> SnapshotIdentity {
        SnapshotIdentity {
            commit_sha: commit_sha(),
            snapshot_sha256: digest(),
            policy_sha256: digest(),
            runner_id: "runner-01".to_string(),
            toolchain_id: "rustc-1.78.0".to_string(),
            schema_version: SCHEMA_VERSION,
        }
    }

    fn gate_evidence(gate: AcceptanceGate, exit: i32) -> GateEvidence {
        GateEvidence {
            gate,
            command: vec!["cargo".to_string(), "test".to_string()],
            exit: Some(exit),
            skipped: false,
        }
    }

    fn valid_evidence() -> AcceptanceEvidence {
        AcceptanceEvidence {
            identity: approved_identity(),
            gates: MANDATORY_GATES
                .iter()
                .copied()
                .map(|gate| gate_evidence(gate, 0))
                .collect(),
        }
    }

    #[test]
    fn all_gates_pass_is_accepted() {
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &valid_evidence()),
            AcceptanceVerdict::Accepted
        ));
    }

    #[test]
    fn empty_commit_rejected() {
        let mut identity = approved_identity();
        identity.commit_sha = String::new();
        let verdict = evaluate_acceptance(&identity, &valid_evidence());
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::InvalidIdentity { field, .. })
            if field == "commit_sha"
        ));
    }

    #[test]
    fn whitespace_runner_id_rejected() {
        let mut identity = approved_identity();
        identity.runner_id = "   ".to_string();
        let verdict = evaluate_acceptance(&identity, &valid_evidence());
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::InvalidIdentity { field, .. })
            if field == "runner_id"
        ));
    }

    #[test]
    fn short_commit_rejected() {
        let mut identity = approved_identity();
        identity.commit_sha = "a1b2".to_string();
        let verdict = evaluate_acceptance(&identity, &valid_evidence());
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::InvalidIdentity { .. })
        ));
    }

    #[test]
    fn uppercase_commit_rejected() {
        let mut identity = approved_identity();
        identity.commit_sha = "A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2".to_string();
        let verdict = evaluate_acceptance(&identity, &valid_evidence());
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::InvalidIdentity { .. })
        ));
    }

    #[test]
    fn wrong_digest_length_rejected() {
        let mut identity = approved_identity();
        identity.snapshot_sha256 = "a1b2".to_string();
        let verdict = evaluate_acceptance(&identity, &valid_evidence());
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::InvalidIdentity { field, .. })
            if field == "snapshot_sha256"
        ));
    }

    #[test]
    fn wrong_schema_version_rejected() {
        let mut identity = approved_identity();
        identity.schema_version = SCHEMA_VERSION + 1;
        let verdict = evaluate_acceptance(&identity, &valid_evidence());
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::InvalidIdentity { field, .. })
            if field == "schema_version"
        ));
    }

    #[test]
    fn stale_commit_in_evidence_rejected() {
        let mut evidence = valid_evidence();
        evidence.identity.commit_sha = "0".repeat(40);
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::IdentityMismatch)
        ));
    }

    #[test]
    fn wrong_runner_in_evidence_rejected() {
        let mut evidence = valid_evidence();
        evidence.identity.runner_id = "runner-02".to_string();
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::IdentityMismatch)
        ));
    }

    #[test]
    fn empty_gates_rejected() {
        let evidence = AcceptanceEvidence {
            identity: approved_identity(),
            gates: Vec::new(),
        };
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &evidence),
            AcceptanceVerdict::Rejected(AcceptanceRejection::GatesEmpty)
        ));
    }

    #[test]
    fn missing_gate_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates.pop();
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateCountMismatch { expected, actual })
            if expected == 7 && actual == 6
        ));
    }

    #[test]
    fn extra_gate_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates.push(gate_evidence(AcceptanceGate::Tests, 0));
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateCountMismatch { expected, actual })
            if expected == 7 && actual == 8
        ));
    }

    #[test]
    fn duplicate_gate_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates[2] = GateEvidence {
            gate: AcceptanceGate::Structural,
            ..evidence.gates[2].clone()
        };
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateSequenceMismatch { position, expected, actual })
            if position == 2 && expected == AcceptanceGate::TypeCheck && actual == AcceptanceGate::Structural
        ));
    }

    #[test]
    fn reordered_gates_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates.swap(0, 1);
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateSequenceMismatch { position, expected, actual })
            if position == 0 && expected == AcceptanceGate::Formal && actual == AcceptanceGate::Structural
        ));
    }

    #[test]
    fn failed_gate_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates[3].exit = Some(1);
        let verdict = evaluate_acceptance(&approved_identity(), &evidence);
        assert!(matches!(
            verdict,
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateFailed { gate, exit })
            if gate == AcceptanceGate::Clippy && exit == 1
        ));
    }

    #[test]
    fn negative_exit_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates[3].exit = Some(-1);
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &evidence),
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateFailed { exit: -1, .. })
        ));
    }

    #[test]
    fn exit_not_recorded_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates[3].exit = None;
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &evidence),
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateExitNotRecorded {
                gate: AcceptanceGate::Clippy
            })
        ));
    }

    #[test]
    fn skipped_gate_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates[4].skipped = true;
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &evidence),
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateSkipped {
                gate: AcceptanceGate::Tests
            })
        ));
    }

    #[test]
    fn empty_command_rejected() {
        let mut evidence = valid_evidence();
        evidence.gates[5].command = Vec::new();
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &evidence),
            AcceptanceVerdict::Rejected(AcceptanceRejection::GateCommandMissing {
                gate: AcceptanceGate::Format
            })
        ));
    }

    #[test]
    fn identity_checked_before_gates() {
        let mut evidence = valid_evidence();
        evidence.identity.commit_sha = "0".repeat(40);
        evidence.gates[0].exit = Some(1);
        assert!(matches!(
            evaluate_acceptance(&approved_identity(), &evidence),
            AcceptanceVerdict::Rejected(AcceptanceRejection::IdentityMismatch)
        ));
    }
}
