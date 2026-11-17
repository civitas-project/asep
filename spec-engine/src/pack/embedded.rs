//! Embedded canonical skills and default plugin manifests.

// @impl:FR-PACK-EMBED-01
// @impl:FR-PACK-EMBED-02

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedSkill {
    pub id: &'static str,
    pub directory: &'static str,
    pub content: &'static str,
}

const SKILL_SYSTEM_ARCHITECTURE: &str = include_str!("../../../system-architecture/skill/SKILL.md");
const SKILL_CODE_CONTRACTS: &str = include_str!("../../../code-contracts/skill/SKILL.md");
const SKILL_ENGINEERING_SPECS: &str = include_str!("../../../engineering-specs/skill/SKILL.md");
const SKILL_REPOSITORY_GOVERNANCE: &str =
    include_str!("../../../repository-governance/skill/SKILL.md");
const SKILL_DESIGN_TOKENS: &str = include_str!("../../../design-tokens/skill/SKILL.md");
const SKILL_WORKFLOW_ROUTER: &str = include_str!("../../../workflow-router/skill/SKILL.md");

const DEFAULT_PLUGIN_JSON: &str = r#"{
  "name": "asep",
  "description": "Axiomatic Systems Engineering Protocol (ASEP): Deterministic SDLC validator, traceability graph checker, and living spec engine"
}"#;

const DEFAULT_MCP_CONFIG_JSON: &str = r#"{
  "mcpServers": {
    "asep": {
      "command": "asep",
      "args": ["mcp"]
    }
  }
}"#;

static EMBEDDED_SKILLS: &[EmbeddedSkill] = &[
    EmbeddedSkill {
        id: "workflow-router",
        directory: "workflow-router",
        content: SKILL_WORKFLOW_ROUTER,
    },
    EmbeddedSkill {
        id: "system-architecture",
        directory: "system-architecture",
        content: SKILL_SYSTEM_ARCHITECTURE,
    },
    EmbeddedSkill {
        id: "code-contracts",
        directory: "code-contracts",
        content: SKILL_CODE_CONTRACTS,
    },
    EmbeddedSkill {
        id: "engineering-specs",
        directory: "engineering-specs",
        content: SKILL_ENGINEERING_SPECS,
    },
    EmbeddedSkill {
        id: "repository-governance",
        directory: "repository-governance",
        content: SKILL_REPOSITORY_GOVERNANCE,
    },
    EmbeddedSkill {
        id: "design-tokens",
        directory: "design-tokens",
        content: SKILL_DESIGN_TOKENS,
    },
];

/// Returns the slice of canonical embedded skills.
pub fn get_embedded_skills() -> &'static [EmbeddedSkill] {
    EMBEDDED_SKILLS
}

/// Returns the default Antigravity plugin manifest JSON string.
pub fn get_default_plugin_json() -> &'static str {
    DEFAULT_PLUGIN_JSON
}

/// Returns the default MCP config JSON string.
pub fn get_default_mcp_config_json() -> &'static str {
    DEFAULT_MCP_CONFIG_JSON
}
