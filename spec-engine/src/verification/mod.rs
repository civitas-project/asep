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
}

fn parse_semantic_evidence(content: &str) -> Option<SemanticEvidence> {
    let mut reviewer = None;
    let mut verdict = None;
    let mut rationale = None;
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
                _ => {}
            }
        }
    }
    let reviewer = reviewer?;
    let verdict = verdict?;
    let rationale = rationale?;
    if verdict != "ACCEPTED" && verdict != "REJECTED" {
        return None;
    }
    Some(SemanticEvidence {
        reviewer,
        verdict,
        rationale,
    })
}

fn evidence_path(evidence_dir: &Path, subject: &str) -> PathBuf {
    evidence_dir.join("semantic").join(format!("{subject}.md"))
}

fn load_semantic_evidence(evidence_dir: &Path, subject: &str) -> Option<SemanticEvidence> {
    let content = fs::read_to_string(evidence_path(evidence_dir, subject)).ok()?;
    parse_semantic_evidence(&content)
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

// @impl:FR-VERIFY-01-C
/// Computes the verification status for a `GraphReport`.
///
/// - `structural` = `graph.is_closed` (no error-severity coverage violation).
/// - `behavioral` = `behavioral.json::passed` (missing file => false + skipped).
/// - `semantic` = every `Req` node has `evidence/semantic/<id>.md` with valid
///   schema and `verdict == ACCEPTED`; skipped (and treated as satisfied) when
///   the tier does not require semantic evidence.
///
/// Status: `Fail` if structural or behavioral is false, else `Unproven` if
/// semantic evidence is required but missing, else `Pass`.
pub fn evaluate(
    graph: &GraphReport,
    evidence_dir: &Path,
    tier: TierId,
    behavioral_report: &Path,
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
        semantic = req_ids.iter().all(|subject| {
            load_semantic_evidence(evidence_dir, subject).is_some_and(|e| e.verdict == "ACCEPTED")
        });
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
            format!("reviewer: qa\nverdict: {verdict}\nrationale: looks good\n"),
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
}
