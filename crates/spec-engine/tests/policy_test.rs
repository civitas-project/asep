use asep::policy::{load_policy, CoverageRule, Policy, Severity, TierId};
use regex::Regex;

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

#[test]
fn test_policy_loads_antislop_rules_for_four_domains() {
    let policy = load_policy();
    let rules = policy.antislop_rules();
    assert_eq!(rules.len(), 13);

    let else_rule = rules
        .iter()
        .find(|r| r.id == "AS-CODE-ELSE")
        .expect("AS-CODE-ELSE present in policy");
    assert_eq!(else_rule.severity, Severity::Warning);

    let mut domains: Vec<&str> = rules.iter().map(|r| r.domain.as_str()).collect();
    domains.sort_unstable();
    domains.dedup();
    assert_eq!(domains, vec!["code", "copy", "design", "docs"]);

    let banner = rules
        .iter()
        .find(|r| r.id == "AS-CODE-BANNER")
        .expect("AS-CODE-BANNER present in policy");
    assert_eq!(banner.severity, Severity::Error);

    let boxdraw = rules
        .iter()
        .find(|r| r.id == "AS-CODE-BOXDRAW")
        .expect("AS-CODE-BOXDRAW present in policy");
    assert_eq!(boxdraw.severity, Severity::Error);

    let nesting = rules
        .iter()
        .find(|r| r.id == "AS-CODE-NESTING")
        .expect("AS-CODE-NESTING present in policy");
    assert_eq!(nesting.severity, Severity::Warning);
    assert_eq!(nesting.max_nesting, Some(0));

    let hype = rules
        .iter()
        .find(|r| r.id == "AS-COPY-HYPE")
        .expect("AS-COPY-HYPE present in policy");
    assert_eq!(hype.severity, Severity::Warning);

    let buzzword = rules
        .iter()
        .find(|r| r.id == "AS-COPY-BUZZWORD")
        .expect("AS-COPY-BUZZWORD present in policy");
    assert_eq!(buzzword.severity, Severity::Error);

    for rule in rules {
        if !rule.pattern.is_empty() {
            Regex::new(&rule.pattern).expect("antislop pattern must compile as regex");
        }
    }
}

#[test]
// @test:R11
fn rigor_levels_map_to_canonical_tiers() {
    use asep::policy::tier_for_rigor_level;
    assert_eq!(tier_for_rigor_level("Lightweight"), Some(asep::policy::TierId::T1));
    assert_eq!(tier_for_rigor_level("Structured"), Some(asep::policy::TierId::T2));
    assert_eq!(tier_for_rigor_level("High Assurance"), Some(asep::policy::TierId::T4));
    assert_eq!(tier_for_rigor_level("unknown"), None);
}
