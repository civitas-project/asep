// @test:FR-AMB-03 // @test:FR-AMB-04
use asep::pack::embedded::{
    get_default_mcp_config_json, get_default_plugin_json, get_embedded_skills,
};

// @test:FR-PACK-EMBED-01
#[test]
fn test_get_embedded_skills_returns_all_canonical_protocols() {
    let skills = get_embedded_skills();
    assert_eq!(skills.len(), 7, "Expected 7 canonical engineering skills");

    let ids: Vec<&str> = skills.iter().map(|s| s.id).collect();
    assert!(ids.contains(&"using-asep"));
    assert!(ids.contains(&"workflow-router"));
    assert!(ids.contains(&"system-architecture"));
    assert!(ids.contains(&"code-contracts"));
    assert!(ids.contains(&"engineering-specs"));
    assert!(ids.contains(&"repository-governance"));
    assert!(ids.contains(&"design-tokens"));

    for skill in skills {
        assert!(
            skill.content.contains("---"),
            "Skill {} must have YAML frontmatter",
            skill.id
        );
        assert!(
            skill.content.contains(&format!("name: {}", skill.id)),
            "Skill {} frontmatter must declare its name",
            skill.id
        );
        assert!(
            skill.content.contains("description:"),
            "Skill {} frontmatter must have description",
            skill.id
        );
    }
}

// @test:FR-PACK-EMBED-03
#[test]
fn test_skill_text_references_canonical_rule_ids() {
    let skills = get_embedded_skills();
    assert_eq!(skills.len(), 7, "Expected 7 canonical embedded skills");

    for skill in skills {
        let content = skill.content;
        let references_canonical_rules =
            content.contains("COV-0") || content.contains("Tier T") || content.contains("exit 3");
        assert!(
            references_canonical_rules,
            "Skill {} must reference canonical rules (COV-0 / Tier T / exit 3)",
            skill.id
        );

        let lines: Vec<&str> = content.lines().collect();
        for pair in lines.windows(2) {
            assert!(
                !(pair[0].contains("COV-0") && pair[1].contains("COV-0")),
                "Skill {} must not restate a full coverage table (no consecutive COV-0 lines)",
                skill.id
            );
        }
    }
}

// @test:FR-PACK-EMBED-05
#[test]
fn test_skills_reference_antislop_rule_ids() {
    let skills = get_embedded_skills();
    assert_eq!(skills.len(), 7, "Expected 7 canonical embedded skills");

    let expected: &[(&str, &[&str])] = &[
        (
            "engineering-specs",
            &[
                "AS-DESIGN-VAGUE",
                "AS-DESIGN-TBD",
                "AS-DOCS-PLACEHOLDER",
                "AS-DOCS-WORDY",
            ],
        ),
        (
            "code-contracts",
            &[
                "AS-CODE-BANNER",
                "AS-CODE-NARRATION",
                "AS-CODE-BUZZWORD",
                "AS-CODE-BOXDRAW",
                "AS-CODE-NESTING",
            ],
        ),
        (
            "repository-governance",
            &[
                "AS-COPY-FILLER",
                "AS-COPY-HYPE",
                "AS-COPY-BUZZWORD",
                "AS-DOCS-WORDY",
            ],
        ),
        ("design-tokens", &["AS-COPY-HYPE", "AS-DOCS-PLACEHOLDER"]),
        (
            "workflow-router",
            &[
                "AS-DESIGN-VAGUE",
                "AS-DESIGN-TBD",
                "AS-DOCS-PLACEHOLDER",
                "AS-DOCS-WORDY",
                "AS-CODE-BANNER",
                "AS-CODE-NARRATION",
                "AS-CODE-BUZZWORD",
                "AS-CODE-BOXDRAW",
                "AS-CODE-NESTING",
                "AS-COPY-FILLER",
                "AS-COPY-HYPE",
                "AS-COPY-BUZZWORD",
            ],
        ),
    ];

    for (id, rule_ids) in expected {
        let skill = skills
            .iter()
            .find(|s| s.id == *id)
            .expect("embedded skill must exist");
        assert!(
            skill.content.contains("Canonical Anti-Slop Rules"),
            "Skill {} must contain 'Canonical Anti-Slop Rules' section",
            id
        );
        let has_rule_id = rule_ids.iter().any(|rid| skill.content.contains(rid));
        assert!(
            has_rule_id,
            "Skill {} must reference at least one canonical anti-slop AS- rule id",
            id
        );
        for rule_id in rule_ids.iter() {
            assert!(
                skill.content.contains(*rule_id),
                "Skill {} must reference canonical anti-slop rule {}",
                id,
                rule_id
            );
        }
    }
}

// @test:FR-PACK-EMBED-04
#[test]
fn test_workflow_router_defines_requirement_and_plan_phase() {
    let skills = get_embedded_skills();
    let router = skills
        .iter()
        .find(|s| s.id == "workflow-router")
        .expect("embedded workflow-router skill must exist");

    for required in [
        "Requirement and Plan Phase",
        "docs/PRD.md",
        "docs/TDD.md",
        "Never create .superpowers/",
    ] {
        assert!(
            router.content.contains(required),
            "workflow-router skill must define requirement and plan phase: missing {:?}",
            required
        );
    }
}

// @test:FR-PACK-EMBED-05
#[test]
fn test_workflow_router_uses_targeted_verification() {
    let skills = get_embedded_skills();
    let router = skills
        .iter()
        .find(|s| s.id == "workflow-router")
        .expect("embedded workflow-router skill must exist");

    for required in ["Targeted Verification", "asep affected"] {
        assert!(
            router.content.contains(required),
            "workflow-router skill must document targeted verification: missing {:?}",
            required
        );
    }
}

// @test:FR-PACK-EMBED-USING-ASEP
#[test]
fn test_using_asep_skill_present_and_bounded() {
    let skills = get_embedded_skills();
    let skill = skills
        .iter()
        .find(|s| s.id == "using-asep")
        .expect("embedded using-asep skill must exist");

    assert_eq!(skill.directory, "using-asep");
    assert!(
        skill.content.contains("name: using-asep"),
        "using-asep frontmatter must declare name: using-asep"
    );

    let desc_line = skill
        .content
        .lines()
        .find(|l| l.trim_start().starts_with("description:"))
        .expect("using-asep frontmatter must have description:");
    assert!(
        desc_line.trim_start().starts_with("description: Use when"),
        "using-asep description must start with 'Use when', got: {:?}",
        desc_line
    );

    let rest = skill.content.strip_prefix("---\n").unwrap_or(skill.content);
    let body = rest.splitn(2, "\n---\n").nth(1).unwrap_or(rest);
    let word_count = body.split_whitespace().count();
    assert!(
        word_count <= 200,
        "using-asep body must be <= 200 words, got {}",
        word_count
    );
}

// @test:FR-PACK-EMBED-02
#[test]
fn test_get_default_manifests_valid_json() {
    let plugin_json = get_default_plugin_json();
    let parsed_plugin: serde_json::Value =
        serde_json::from_str(plugin_json).expect("plugin.json must be valid JSON");
    assert_eq!(parsed_plugin["name"], "asep");

    let mcp_config = get_default_mcp_config_json();
    let parsed_mcp: serde_json::Value =
        serde_json::from_str(mcp_config).expect("mcp_config.json must be valid JSON");
    assert!(parsed_mcp["mcpServers"]["asep"].is_object());
    assert_eq!(parsed_mcp["mcpServers"]["asep"]["command"], "asep");
}
