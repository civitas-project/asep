use asep::pack::embedded::{
    get_default_mcp_config_json, get_default_plugin_json, get_embedded_skills,
};

// @test:FR-PACK-EMBED-01
#[test]
fn test_get_embedded_skills_returns_all_canonical_protocols() {
    let skills = get_embedded_skills();
    assert_eq!(skills.len(), 6, "Expected 6 canonical engineering skills");

    let ids: Vec<&str> = skills.iter().map(|s| s.id).collect();
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
    assert_eq!(skills.len(), 6, "Expected 6 canonical embedded skills");

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
