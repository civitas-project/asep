use asep::skill::semver_resolver::{match_version_constraint, resolve_skill_dependency_version};
use asep::skill::remote::{parse_skill_specifier, SkillSource};

#[test]
fn test_parse_skill_specifier() {
    // Git URL with branch/tag
    let spec = parse_skill_specifier("https://github.com/foo/bar.git#v1.2.0");
    match spec {
        SkillSource::Git { url, tag_or_branch } => {
            assert_eq!(url, "https://github.com/foo/bar.git");
            assert_eq!(tag_or_branch.as_deref(), Some("v1.2.0"));
        }
        _ => panic!("Expected Git source"),
    }

    // Direct HTTP URL
    let spec = parse_skill_specifier("https://example.com/skills/custom/SKILL.md");
    match spec {
        SkillSource::Http { url } => {
            assert_eq!(url, "https://example.com/skills/custom/SKILL.md");
        }
        _ => panic!("Expected Http source"),
    }

    // Registry ID with semver
    let spec = parse_skill_specifier("code-contracts@^2.0.0");
    match spec {
        SkillSource::Registry { id, version_req } => {
            assert_eq!(id, "code-contracts");
            assert_eq!(version_req.as_deref(), Some("^2.0.0"));
        }
        _ => panic!("Expected Registry source"),
    }

    // Bare ID
    let spec = parse_skill_specifier("code-contracts");
    match spec {
        SkillSource::Registry { id, version_req } => {
            assert_eq!(id, "code-contracts");
            assert_eq!(version_req, None);
        }
        _ => panic!("Expected Registry source"),
    }
}

#[test]
fn test_semver_constraint_matching() {
    assert!(match_version_constraint("2.1.0", "^2.0.0"));
    assert!(match_version_constraint("2.0.1", "~2.0.0"));
    assert!(!match_version_constraint("3.0.0", "^2.0.0"));
    assert!(match_version_constraint("1.5.0", ">=1.0.0, <2.0.0"));
}

#[test]
fn test_resolve_skill_dependency_version() {
    let available_versions = vec![
        "1.0.0".to_string(),
        "1.1.0".to_string(),
        "2.0.0".to_string(),
        "2.1.0".to_string(),
        "2.2.0".to_string(),
    ];

    let resolved = resolve_skill_dependency_version(&available_versions, "^2.0.0");
    assert_eq!(resolved.as_deref(), Some("2.2.0"));

    let resolved = resolve_skill_dependency_version(&available_versions, "~1.0.0");
    assert_eq!(resolved.as_deref(), Some("1.0.0"));

    let resolved = resolve_skill_dependency_version(&available_versions, ">=3.0.0");
    assert_eq!(resolved, None);
}
