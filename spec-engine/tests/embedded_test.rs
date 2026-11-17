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
