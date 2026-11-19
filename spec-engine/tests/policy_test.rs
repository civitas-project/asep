use asep::policy::{CoverageRule, Policy, Severity, TierId, load_policy};

fn rule_by_id<'a>(policy: &'a Policy, id: &str) -> &'a CoverageRule {
    policy
        .coverage_rules()
        .iter()
        .find(|r| r.id == id)
        .expect("coverage rule present in policy")
}

#[test]
fn coverage_rules_have_canonical_ids_cov_01_to_cov_06() {
    let policy = load_policy();
    let ids: Vec<&str> = policy
        .coverage_rules()
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["COV-01", "COV-02", "COV-03", "COV-04", "COV-05", "COV-06"]
    );
}

#[test]
fn cov_01_min_one_and_error_severity() {
    let policy = load_policy();
    let rule = rule_by_id(&policy, "COV-01");
    assert_eq!(rule.min, 1);
    assert_eq!(rule.severity, Severity::Error);
}

#[test]
fn cov_02_severity_is_warning() {
    let policy = load_policy();
    let rule = rule_by_id(&policy, "COV-02");
    assert_eq!(rule.severity, Severity::Warning);
}

#[test]
fn tier_t2_requires_semantic_evidence() {
    let policy = load_policy();
    assert!(policy.tier(TierId::T2).semantic_evidence_required);
}

#[test]
fn tier_t0_does_not_require_semantic_evidence() {
    let policy = load_policy();
    assert!(!policy.tier(TierId::T0).semantic_evidence_required);
}
