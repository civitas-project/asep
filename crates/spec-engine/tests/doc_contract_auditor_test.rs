use asep::auditor::doc_contracts::{audit_doc_contracts, DocContractViolationKind};
use std::path::Path;

// @test:FR-CLI-DOC-01
#[test]
fn test_audit_doc_contract_parroting_detection() {
    let code = r#"
/// calculate_fee calculates the fee
pub fn calculate_fee(amount: u64) -> u64 {
    amount
}
"#;
    let violations = audit_doc_contracts(Path::new("src/fee.rs"), code);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        &violations[0].kind,
        DocContractViolationKind::Parroting { .. }
    ));
}

// @test:FR-CLI-DOC-02
#[test]
fn test_audit_doc_contract_phantom_sections_detection() {
    let code = r#"
/// Computes loyalty score.
///
/// # Panics: None
/// # Errors: N/A
pub fn calculate_score(points: u64) -> u64 {
    points * 2
}
"#;
    let violations = audit_doc_contracts(Path::new("src/score.rs"), code);
    assert_eq!(violations.len(), 2);
    assert!(violations.iter().any(
        |v| matches!(&v.kind, DocContractViolationKind::PhantomSection { section } if section == "Panics")
    ));
    assert!(violations.iter().any(
        |v| matches!(&v.kind, DocContractViolationKind::PhantomSection { section } if section == "Errors")
    ));
}

#[test]
fn test_audit_doc_contract_valid_compact_contract() {
    let code = r#"
// @impl:FR-POINTS-01
/// Computes loyalty reward points at base ratio of 1 point per 1000 currency units.
pub fn calculate_points(cents: u64) -> u64 {
    cents / 1000
}
"#;
    let violations = audit_doc_contracts(Path::new("src/points.rs"), code);
    assert!(
        violations.is_empty(),
        "Valid compact contract must have 0 violations"
    );
}
