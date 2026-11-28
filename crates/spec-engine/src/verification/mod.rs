//! Verification status, evidence, and exit codes (Task 4).
//!
//! Combines the relation graph (`GraphReport`), semantic evidence, and the
//! behavioral report into a single `VerificationReport`. The verdict drives
//! the `asep check` exit code: `Pass`=0, `Fail`=1, `Unproven`=3.

use crate::graph::model::{GraphReport, NodeKind};
use crate::policy::{load_policy, TierId};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Snapshot and policy identity bound to evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBinding {
    pub revision: String,
    pub tree_digest: String,
    pub policy_digest: String,
    pub toolchain: String,
}

impl EvidenceBinding {
    pub fn current(root: &Path) -> Result<Self, String> {
        let git = |args: &[&str]| -> Result<String, String> {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .map_err(|e| format!("git spawn failed: {e}"))?;
            if !out.status.success() {
                return Err(format!("git {:?} failed", args));
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        };
        let policy_text = include_str!("../../policy/asep-policy.toml");
        let toolchain = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .map_err(|e| format!("rustc spawn failed: {e}"))?;
        Ok(Self {
            revision: git(&["rev-parse", "HEAD"])?,
            tree_digest: git(&["rev-parse", "HEAD^{tree}"])?,
            policy_digest: hex_digest(policy_text.as_bytes()),
            toolchain: String::from_utf8_lossy(&toolchain.stdout).trim().to_string(),
        })
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

// @impl:FR-VERIFY-01
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum VerificationStatus {
    Pass,
    Fail,
    Unproven,
}

// @impl:FR-VERIFY-01-B
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerificationReport {
    pub status: VerificationStatus,
    pub structural: bool,
    pub behavioral: bool,
    pub semantic: bool,
    pub skipped: Vec<String>,
}

/// Semantic evidence file parsed for schema validation only.
/// The validator checks presence + schema (`reviewer`, `verdict`, `rationale`)
/// and that `verdict` is `ACCEPTED` or `REJECTED`; it never inspects content.
#[allow(dead_code)]
struct SemanticEvidence {
    reviewer: String,
    verdict: String,
    rationale: String,
    schema_version: Option<String>,
    revision: Option<String>,
    tree_digest: Option<String>,
    policy_digest: Option<String>,
    changed_paths_digest: Option<String>,
    toolchain: Option<String>,
    state: EvidenceState,
}

/// Result of semantic evidence evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Pass,
    Missing,
    Stale,
    Rejected,
    Unproven,
}

fn parse_semantic_evidence(content: &str) -> Option<SemanticEvidence> {
    let mut reviewer = None;
    let mut verdict = None;
    let mut rationale = None;
    let mut schema_version = None;
    let mut revision = None;
    let mut tree_digest = None;
    let mut policy_digest = None;
    let mut changed_paths_digest = None;
    let mut toolchain = None;
    let mut state = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            match key.trim() {
                "reviewer" => reviewer = Some(value.trim().to_string()),
                "verdict" => verdict = Some(value.trim().to_string()),
                "rationale" => rationale = Some(value.trim().to_string()),
                "schema_version" => schema_version = Some(value.trim().to_string()),
                "revision" => revision = Some(value.trim().to_string()),
                "tree_digest" => tree_digest = Some(value.trim().to_string()),
                "policy_digest" => policy_digest = Some(value.trim().to_string()),
                "changed_paths_digest" => changed_paths_digest = Some(value.trim().to_string()),
                "toolchain" => toolchain = Some(value.trim().to_string()),
                "policy_digest" => policy_digest = Some(value.trim().to_string()),
                "toolchain" => toolchain = Some(value.trim().to_string()),
                "evidence_state" => {
                    state = Some(match value.trim() {
                        "PASS" => EvidenceState::Pass,
                        "MISSING" => EvidenceState::Missing,
                        "STALE" => EvidenceState::Stale,
                        "REJECTED" => EvidenceState::Rejected,
                        "UNPROVEN" => EvidenceState::Unproven,
                        _ => return None,
                    });
                }
                _ => {}
            }
        }
    }
    let reviewer = reviewer?;
    let verdict = verdict?;
    let rationale = rationale?;
    if verdict.is_empty() || rationale.is_empty() {
        return None;
    }
    Some(SemanticEvidence {
        reviewer,
        verdict,
        rationale,
        schema_version,
        revision,
        tree_digest,
        policy_digest,
        changed_paths_digest,
        toolchain,
        state: state?,
    })
}

fn evidence_path(evidence_dir: &Path, subject: &str) -> PathBuf {
    evidence_dir.join("semantic").join(format!("{subject}.md"))
}

fn load_semantic_evidence(evidence_dir: &Path, subject: &str) -> Option<String> {
    fs::read_to_string(evidence_path(evidence_dir, subject)).ok()
}

/// Reads `.spec-engine/behavioral.json` and returns the `passed` field.
/// Missing or unreadable file => `false`.
fn read_behavioral_passed(path: &Path) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    value
        .get("passed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// @impl:FR-AUD-04
pub fn evidence_state(
    content: &str,
    expected_revision: Option<&str>,
    expected_tree_digest: Option<&str>,
    expected_policy_digest: Option<&str>,
    expected_toolchain: Option<&str>,
) -> EvidenceState {
    let evidence = match parse_semantic_evidence(content) {
        Some(e) => e,
        None => return EvidenceState::Missing,
    };
    if evidence.state == EvidenceState::Rejected {
        return EvidenceState::Rejected;
    }
    let revision_stale =
        expected_revision.is_some_and(|expected| evidence.revision.as_deref() != Some(expected));
    if revision_stale {
        return EvidenceState::Stale;
    }
    let tree_stale = expected_tree_digest
        .is_some_and(|expected| evidence.tree_digest.as_deref() != Some(expected));
    if tree_stale {
        return EvidenceState::Stale;
    }
    let policy_stale = expected_policy_digest
        .is_some_and(|expected| evidence.policy_digest.as_deref() != Some(expected));
    if policy_stale {
        return EvidenceState::Stale;
    }
    let toolchain_stale = expected_toolchain
        .is_some_and(|expected| evidence.toolchain.as_deref() != Some(expected));
    if toolchain_stale {
        return EvidenceState::Stale;
    }
    let accepted = evidence.state == EvidenceState::Pass;
    let version_ok = evidence
        .schema_version
        .as_deref()
        .map_or(true, |v| v.parse::<u32>().ok() == Some(1));
    if accepted && version_ok {
        return EvidenceState::Pass;
    }
    EvidenceState::Unproven
}

/// Evaluation bound to a concrete snapshot, policy and toolchain. Semantic
/// and behavioral evidence failing any binding check is STALE/UNPROVEN, never
/// a successful verification.
pub fn evaluate_bound(
    graph: &GraphReport,
    evidence_dir: &Path,
    tier: TierId,
    behavioral_report: &Path,
    binding: &EvidenceBinding,
) -> VerificationReport {
    let mut skipped: Vec<String> = Vec::new();
    let structural = graph.is_closed;

    let behavioral = match read_behavioral_bound(behavioral_report, binding) {
        Ok(passed) => passed,
        Err(_) => false,
    };
    if !behavioral_report.exists() {
        skipped.push("behavioral".to_string());
    }

    let policy = load_policy();
    let semantic_required = policy.tier(tier).semantic_evidence_required;
    let req_ids: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Req))
        .map(|node| node.id.as_str())
        .collect();

    let mut semantic = true;
    if semantic_required {
        semantic = req_ids.iter().all(|subject| {
            match load_semantic_evidence(evidence_dir, subject) {
                Some(content) => evidence_state(
                    &content,
                    Some(&binding.revision),
                    Some(&binding.tree_digest),
                    Some(&binding.policy_digest),
                    Some(&binding.toolchain),
                ) == EvidenceState::Pass,
                None => false,
            }
        });
    } else {
        skipped.push("semantic".to_string());
    }

    let mut status = VerificationStatus::Pass;
    if !structural || !behavioral {
        status = VerificationStatus::Fail;
    }
    if structural && behavioral && semantic_required && !semantic {
        status = VerificationStatus::Unproven;
    }
    VerificationReport {
        status,
        structural,
        behavioral,
        semantic,
        skipped,
    }
}

fn read_behavioral_bound(path: &Path, binding: &EvidenceBinding) -> Result<bool, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let matches = |key: &str, expected: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(|actual| actual == expected)
            .unwrap_or(true)
    };
    if !matches("revision", &binding.revision)
        || !matches("tree_digest", &binding.tree_digest)
        || !matches("policy_digest", &binding.policy_digest)
        || !matches("toolchain", &binding.toolchain)
    {
        return Err("behavioral evidence is stale".to_string());
    }
    Ok(value.get("passed").and_then(|v| v.as_bool()).unwrap_or(false))
}

/// Same as [`evaluate`] but gates semantic acceptance on
/// `evidence_state(...) == EvidenceState::Pass` with explicit expected
/// revision and tree digest, instead of a bare `verdict == ACCEPTED` check.
pub fn evaluate_with_revision(
    graph: &GraphReport,
    evidence_dir: &Path,
    tier: TierId,
    behavioral_report: &Path,
    expected_revision: Option<&str>,
    expected_tree_digest: Option<&str>,
) -> VerificationReport {
    let mut skipped: Vec<String> = Vec::new();

    let structural = graph.is_closed;

    let behavioral = read_behavioral_passed(behavioral_report);
    if !behavioral_report.exists() {
        skipped.push("behavioral".to_string());
    }

    let policy = load_policy();
    let semantic_required = policy.tier(tier).semantic_evidence_required;

    let req_ids: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Req))
        .map(|n| n.id.as_str())
        .collect();

    let mut semantic = true;
    if semantic_required {
        semantic =
            req_ids.iter().all(
                |subject| match load_semantic_evidence(evidence_dir, subject) {
                    Some(content) => {
                        evidence_state(
                            &content,
                            expected_revision,
                            expected_tree_digest,
                            None,
                            None,
                        ) == EvidenceState::Pass
                    }
                    None => false,
                },
            );
    }
    if !semantic_required {
        skipped.push("semantic".to_string());
    }

    let mut status = VerificationStatus::Pass;
    if !structural || !behavioral {
        status = VerificationStatus::Fail;
    }
    if structural && behavioral && semantic_required && !semantic {
        status = VerificationStatus::Unproven;
    }

    VerificationReport {
        status,
        structural,
        behavioral,
        semantic,
        skipped,
    }
}

// @impl:FR-VERIFY-01-C
pub fn evaluate(
    graph: &GraphReport,
    evidence_dir: &Path,
    tier: TierId,
    behavioral_report: &Path,
) -> VerificationReport {
    evaluate_with_revision(graph, evidence_dir, tier, behavioral_report, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::model::{CoverageViolation, TraceNode};

    fn closed_graph() -> GraphReport {
        GraphReport {
            nodes: vec![TraceNode {
                kind: NodeKind::Req,
                id: "FR-X".to_string(),
                file_path: PathBuf::from("spec.md"),
                line_number: 1,
            }],
            edges: vec![],
            violations: vec![],
            is_closed: true,
        }
    }

    fn write_evidence(root: &Path, subject: &str, verdict: &str) {
        let dir = root.join("evidence").join("semantic");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{subject}.md")),
            format!("reviewer: qa\nevidence_state: PASS\nverdict: {verdict}\nrationale: looks good\n"),
        )
        .unwrap();
    }

    #[test]
    fn evaluate_missing_semantic_evidence_is_unproven() {
        let root = tempfile::tempdir().unwrap();
        let behavioral = root.path().join(".spec-engine").join("behavioral.json");
        fs::create_dir_all(behavioral.parent().unwrap()).unwrap();
        fs::write(&behavioral, r#"{"passed": true}"#).unwrap();

        let report = evaluate(
            &closed_graph(),
            root.path().join("evidence").as_path(),
            TierId::T2,
            &behavioral,
        );
        assert_eq!(report.status, VerificationStatus::Unproven);
        assert!(report.structural);
        assert!(report.behavioral);
        assert!(!report.semantic);
    }

    #[test]
    fn evaluate_accepted_evidence_is_pass() {
        let root = tempfile::tempdir().unwrap();
        write_evidence(root.path(), "FR-X", "ACCEPTED");
        let behavioral = root.path().join(".spec-engine").join("behavioral.json");
        fs::create_dir_all(behavioral.parent().unwrap()).unwrap();
        fs::write(&behavioral, r#"{"passed": true}"#).unwrap();

        let report = evaluate(
            &closed_graph(),
            root.path().join("evidence").as_path(),
            TierId::T2,
            &behavioral,
        );
        assert_eq!(report.status, VerificationStatus::Pass);
    }

    #[test]
    fn evaluate_t1_skips_semantic() {
        let root = tempfile::tempdir().unwrap();
        let behavioral = root.path().join(".spec-engine").join("behavioral.json");
        fs::create_dir_all(behavioral.parent().unwrap()).unwrap();
        fs::write(&behavioral, r#"{"passed": true}"#).unwrap();

        let report = evaluate(
            &closed_graph(),
            root.path().join("evidence").as_path(),
            TierId::T1,
            &behavioral,
        );
        assert_eq!(report.status, VerificationStatus::Pass);
        assert!(report.skipped.contains(&"semantic".to_string()));
    }

    #[test]
    fn evaluate_broken_graph_is_fail() {
        let root = tempfile::tempdir().unwrap();
        let behavioral = root.path().join(".spec-engine").join("behavioral.json");
        fs::create_dir_all(behavioral.parent().unwrap()).unwrap();
        fs::write(&behavioral, r#"{"passed": true}"#).unwrap();

        let mut broken = closed_graph();
        broken.is_closed = false;
        broken.violations.push(CoverageViolation {
            rule_id: "COV-01".to_string(),
            subject: "FR-X".to_string(),
            found: 0,
            required: 1,
        });
        let report = evaluate(
            &broken,
            root.path().join("evidence").as_path(),
            TierId::T2,
            &behavioral,
        );
        assert_eq!(report.status, VerificationStatus::Fail);
    }

    // @test:FR-AUD-04
    #[test]
    fn evidence_state_missing_when_unparsable() {
        assert_eq!(
            evidence_state("not evidence", None, None, None, None),
            EvidenceState::Missing
        );
        assert_eq!(evidence_state("", None, None, None, None), EvidenceState::Missing);
        assert_eq!(
            evidence_state("reviewer: qa\nverdict: MAYBE\nrationale: x\n", None, None, None, None),
            EvidenceState::Missing
        );
    }

    #[test]
    fn evidence_state_rejected_is_rejected() {
        let content = "reviewer: qa\nevidence_state: REJECTED\nverdict: REJECTED\nrationale: no\n";
        assert_eq!(evidence_state(content, None, None, None, None), EvidenceState::Rejected);
        assert_eq!(
            evidence_state(content, Some("other"), None, None, None),
            EvidenceState::Rejected
        );
    }

    #[test]
    fn evidence_state_stale_when_revision_differs() {
        let content = "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: ok\nrevision: A\n";
        assert_eq!(
            evidence_state(content, Some("B"), None, None, None),
            EvidenceState::Stale
        );
        assert_eq!(
            evidence_state(content, Some("A"), None, None, None),
            EvidenceState::Pass
        );
    }

    #[test]
    fn evidence_state_stale_when_tree_digest_differs() {
        let content = "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: ok\ntree_digest: abc\n";
        assert_eq!(
            evidence_state(content, None, Some("def"), None, None),
            EvidenceState::Stale
        );
        assert_eq!(
            evidence_state(content, None, Some("abc"), None, None),
            EvidenceState::Pass
        );
    }

    #[test]
    fn evidence_state_pass_accepts_schema_v1() {
        let content = "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: ok\nschema_version: 1\n";
        assert_eq!(evidence_state(content, None, None, None, None), EvidenceState::Pass);
    }

    #[test]
    fn evaluate_with_revision_rejects_stale_evidence() {
        let root = tempfile::tempdir().unwrap();
        let behavioral = root.path().join(".spec-engine").join("behavioral.json");
        fs::create_dir_all(behavioral.parent().unwrap()).unwrap();
        fs::write(&behavioral, r#"{"passed": true}"#).unwrap();
        let dir = root.path().join("evidence").join("semantic");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("FR-X.md"),
            "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: ok\nrevision: A\n",
        )
        .unwrap();

        let report = evaluate_with_revision(
            &closed_graph(),
            root.path().join("evidence").as_path(),
            TierId::T2,
            &behavioral,
            Some("B"),
            None,
        );
        assert_eq!(report.status, VerificationStatus::Unproven);
        assert!(!report.semantic);
    }
}

#[cfg(test)]
mod freshness_tests {
    use super::*;
    use crate::graph::model::TraceNode;

    fn report() -> GraphReport {
        GraphReport {
            nodes: vec![TraceNode {
                kind: NodeKind::Req,
                id: "FR-FRESH".to_string(),
                file_path: PathBuf::from("spec.md"),
                line_number: 1,
            }],
            edges: Vec::new(),
            violations: Vec::new(),
            is_closed: true,
        }
    }

    #[test]
    // @test:P0.2
    fn stale_binding_invalidates_semantic_and_behavioral_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let semantic = dir.path().join("semantic");
        fs::create_dir_all(&semantic).unwrap();
        fs::write(
            semantic.join("FR-FRESH.md"),
            "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: ok\nrevision: r1\ntree_digest: t1\npolicy_digest: p1\ntoolchain: rustc-test\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("behavioral.json"),
            r#"{"passed":true,"revision":"r1","tree_digest":"t1","policy_digest":"p1","toolchain":"rustc-test"}"#,
        )
        .unwrap();
        let binding = EvidenceBinding {
            revision: "r1".to_string(),
            tree_digest: "t1".to_string(),
            policy_digest: "p1".to_string(),
            toolchain: "rustc-test".to_string(),
        };
        let fresh = evaluate_bound(
            &report(),
            dir.path(),
            TierId::T2,
            &dir.path().join("behavioral.json"),
            &binding,
        );
        assert_eq!(fresh.status, VerificationStatus::Pass);

        let stale_policy = EvidenceBinding {
            policy_digest: "p2".to_string(),
            ..binding.clone()
        };
        let stale = evaluate_bound(
            &report(),
            dir.path(),
            TierId::T2,
            &dir.path().join("behavioral.json"),
            &stale_policy,
        );
        assert_ne!(stale.status, VerificationStatus::Pass);
        assert!(!stale.behavioral, "stale behavioral report must not pass");
        assert!(!stale.semantic, "stale semantic evidence must not pass");
    }
}
