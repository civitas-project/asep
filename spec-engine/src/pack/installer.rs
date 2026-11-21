//! Interactive `inquire` TUI installer and headless packaging engine.

use crate::config::settings::load_config;
use crate::pack::embedded::{
    get_default_mcp_config_json, get_default_plugin_json, get_embedded_skills,
};
use crate::pack::manifest::{scaffold_opencode_agents, update_opencode_json};
use crate::pack::targets::{default_global_home, resolve_target_paths, InstallScope, TargetAgent};
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select};
use std::fs;
use std::path::PathBuf;

// @impl:FR-PACK-INSTALL-01 @impl:FR-PACK-02
// @impl:FR-PACK-INSTALL-02

#[derive(Debug, Clone, Default)]
pub struct PackOptions {
    pub scope: Option<InstallScope>,
    pub agents: Vec<TargetAgent>,
    pub non_interactive: bool,
    pub target_dir: Option<PathBuf>,
    pub skills_filter: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeChoice {
    Local,
    Global,
}

impl std::fmt::Display for ScopeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeChoice::Local => {
                write!(f, "Current Project Workspace (local to current directory)")
            }
            ScopeChoice::Global => {
                write!(
                    f,
                    "Global User Profile (machine-wide across all detected agentic AI environments)"
                )
            }
        }
    }
}

/// Helper display item for interactive agent selection checklist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetAgentChoice {
    pub agent: TargetAgent,
    pub detected: bool,
}

impl std::fmt::Display for TargetAgentChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.agent.display_name().to_string();
        if self.detected {
            s.push_str(" [Detected]");
        }
        write!(f, "{}", s)
    }
}

/// Executes the pack installer either interactively via `inquire` or headlessly.
pub fn run_pack_installer(opts: PackOptions) -> Result<(), Box<dyn std::error::Error>> {
    let target_dir = opts
        .target_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    // 1. Resolve Scope
    let scope = match (opts.scope.is_some(), opts.non_interactive) {
        (true, _) => opts.scope.unwrap(),
        (false, true) => InstallScope::Local(target_dir.clone()),
        (false, false) => {
            println!(
                "\n{}",
                "============================================================"
                    .bold()
                    .cyan()
            );
            println!(
                "{}",
                "        Spec-Engine Universal Agent Plugin Packager         "
                    .bold()
                    .cyan()
            );
            println!(
                "{}\n",
                "============================================================"
                    .bold()
                    .cyan()
            );

            let scope_choices = vec![ScopeChoice::Local, ScopeChoice::Global];
            let chosen_scope = Select::new("Select installation scope:", scope_choices)
                .with_help_message("Use arrow keys to navigate, Enter to select")
                .prompt()?;

            match chosen_scope {
                ScopeChoice::Local => InstallScope::Local(target_dir.clone()),
                ScopeChoice::Global => InstallScope::Global(default_global_home()),
            }
        }
    };

    // 2. Resolve Target Agents
    let agents: Vec<TargetAgent> = match (!opts.agents.is_empty(), opts.non_interactive) {
        (true, _) => opts.agents,
        (false, true) => {
            let mut agents: Vec<TargetAgent> = TargetAgent::ALL
                .iter()
                .copied()
                .filter(|a| a.is_detected(&scope))
                .collect();
            if agents.is_empty() {
                agents = vec![TargetAgent::Antigravity, TargetAgent::OpenCode];
            }
            agents
        }
        (false, false) => {
            let agent_choices: Vec<TargetAgentChoice> = TargetAgent::ALL
                .iter()
                .map(|&agent| TargetAgentChoice {
                    agent,
                    detected: agent.is_detected(&scope),
                })
                .collect();

            let mut default_indices: Vec<usize> = agent_choices
                .iter()
                .enumerate()
                .filter(|(_, c)| c.detected)
                .map(|(i, _)| i)
                .collect();
            if default_indices.is_empty() {
                default_indices = vec![0, 1];
            }

            let selected = MultiSelect::new(
                "Select target software agents to install plugins into:",
                agent_choices,
            )
            .with_default(&default_indices)
            .with_help_message("Space to toggle, Enter to confirm selection")
            .prompt()?;

            if selected.is_empty() {
                println!("{}", "No agents selected. Aborting installation.".yellow());
                return Ok(());
            }
            selected.into_iter().map(|c| c.agent).collect()
        }
    };

    // 3. Confirm prompt if interactive
    if !opts.non_interactive {
        println!("\n{}", "Summary of Installation Targets:".bold());
        for agent in &agents {
            let path = resolve_target_paths(&scope, *agent);
            println!(
                "   - {}: {}",
                agent.display_name(),
                path.display().to_string().cyan()
            );
        }
    }
    let mut confirmed = true;
    if !opts.non_interactive {
        confirmed = Confirm::new("Proceed with installation to these locations?")
            .with_default(true)
            .prompt()?;
    }
    if !confirmed {
        println!("{}", "Installation cancelled by user.".yellow());
        return Ok(());
    }

    // 4. Resolve Skills to extract
    let all_embedded = get_embedded_skills();
    let skills_to_install: Vec<&crate::pack::embedded::EmbeddedSkill> = match (
        !opts.non_interactive,
        opts.skills_filter.is_some(),
    ) {
        (_, true) => {
            let filter = opts.skills_filter.as_ref().unwrap();
            all_embedded
                .iter()
                .filter(|s| filter.iter().any(|f| f == s.id || f == s.directory))
                .collect()
        }
        (true, false) => {
            let skill_names: Vec<&str> = all_embedded.iter().map(|s| s.id).collect();
            let default_indices: Vec<usize> = (0..all_embedded.len()).collect();
            let selected_skills = MultiSelect::new("Select skills to unpack:", skill_names)
                .with_default(&default_indices)
                .with_help_message("Space to toggle, Enter to confirm selection")
                .prompt()
                .unwrap_or_else(|_| all_embedded.iter().map(|s| s.id).collect());

            all_embedded
                .iter()
                .filter(|s| selected_skills.contains(&s.id))
                .collect()
        }
        (false, false) => all_embedded.iter().collect(),
    };

    let skills = skills_to_install;

    for agent in agents {
        match agent {
            TargetAgent::Antigravity => {
                let plugin_dir = resolve_target_paths(&scope, TargetAgent::Antigravity);
                fs::create_dir_all(&plugin_dir)?;

                // Write plugin.json
                fs::write(plugin_dir.join("plugin.json"), get_default_plugin_json())?;

                // Write mcp_config.json
                fs::write(
                    plugin_dir.join("mcp_config.json"),
                    get_default_mcp_config_json(),
                )?;

                // Write skills
                let skills_dir = plugin_dir.join("skills");
                for skill in &skills {
                    let s_dir = skills_dir.join(skill.directory);
                    fs::create_dir_all(&s_dir)?;
                    fs::write(s_dir.join("SKILL.md"), skill.content)?;
                }

                println!(
                    "{} Installed Antigravity plugin with {} skills at {}",
                    "✔".green(),
                    skills.len(),
                    plugin_dir.display()
                );
            }

            TargetAgent::OpenCode => {
                let skills_dir = resolve_target_paths(&scope, TargetAgent::OpenCode);
                fs::create_dir_all(&skills_dir)?;

                for skill in &skills {
                    let s_dir = skills_dir.join(skill.directory);
                    fs::create_dir_all(&s_dir)?;
                    fs::write(s_dir.join("SKILL.md"), skill.content)?;
                }

                // Update opencode.json
                let config_file = match &scope {
                    InstallScope::Local(dir) => dir.join("opencode.json"),
                    InstallScope::Global(home) => {
                        home.join(".config").join("opencode").join("opencode.json")
                    }
                };

                update_opencode_json(&config_file, "asep")?;
                let profile = load_config(&target_dir).capabilities;
                scaffold_opencode_agents(&config_file, &profile)?;

                println!(
                    "{} Installed OpenCode with {} skills, MCP, and agent presets in {}",
                    "✔".green(),
                    skills.len(),
                    config_file.display()
                );
            }

            TargetAgent::ClaudeCode => {
                let skills_dir = resolve_target_paths(&scope, TargetAgent::ClaudeCode);
                fs::create_dir_all(&skills_dir)?;

                for skill in &skills {
                    let s_dir = skills_dir.join(skill.directory);
                    fs::create_dir_all(&s_dir)?;
                    fs::write(s_dir.join("SKILL.md"), skill.content)?;
                }

                println!(
                    "{} Installed Claude Code with {} skills at {}",
                    "✔".green(),
                    skills.len(),
                    skills_dir.display()
                );
            }

            TargetAgent::Cursor => {
                let rules_dir = resolve_target_paths(&scope, TargetAgent::Cursor);
                fs::create_dir_all(&rules_dir)?;

                for skill in &skills {
                    let rule_file = rules_dir.join(format!("{}.mdc", skill.id));
                    fs::write(rule_file, skill.content)?;
                }

                println!(
                    "{} Installed Cursor AI with {} rules (.mdc) at {}",
                    "✔".green(),
                    skills.len(),
                    rules_dir.display()
                );
            }

            TargetAgent::Windsurf => {
                let rules_dir = resolve_target_paths(&scope, TargetAgent::Windsurf);
                fs::create_dir_all(&rules_dir)?;

                for skill in &skills {
                    let rule_file = rules_dir.join(format!("{}.md", skill.id));
                    fs::write(rule_file, skill.content)?;
                }

                println!(
                    "{} Installed Windsurf with {} rules at {}",
                    "✔".green(),
                    skills.len(),
                    rules_dir.display()
                );
            }

            TargetAgent::Continue => {
                let rules_dir = resolve_target_paths(&scope, TargetAgent::Continue);
                fs::create_dir_all(&rules_dir)?;

                for skill in &skills {
                    let rule_file = rules_dir.join(format!("{}.md", skill.id));
                    fs::write(rule_file, skill.content)?;
                }

                println!(
                    "{} Installed Continue with {} rules at {}",
                    "✔".green(),
                    skills.len(),
                    rules_dir.display()
                );
            }

            TargetAgent::RooCode => {
                let rules_dir = resolve_target_paths(&scope, TargetAgent::RooCode);
                fs::create_dir_all(&rules_dir)?;

                for skill in &skills {
                    let rule_file = rules_dir.join(format!("{}.md", skill.id));
                    fs::write(rule_file, skill.content)?;
                }

                println!(
                    "{} Installed Roo Code / Cline with {} rules at {}",
                    "✔".green(),
                    skills.len(),
                    rules_dir.display()
                );
            }

            TargetAgent::Copilot => {
                let skills_dir = resolve_target_paths(&scope, TargetAgent::Copilot);
                fs::create_dir_all(&skills_dir)?;

                for skill in &skills {
                    let skill_file = skills_dir.join(format!("{}.md", skill.id));
                    fs::write(skill_file, skill.content)?;
                }

                println!(
                    "{} Installed GitHub Copilot with {} skills at {}",
                    "✔".green(),
                    skills.len(),
                    skills_dir.display()
                );
            }

            TargetAgent::Goose => {
                let skills_dir = resolve_target_paths(&scope, TargetAgent::Goose);
                fs::create_dir_all(&skills_dir)?;

                for skill in &skills {
                    let s_dir = skills_dir.join(skill.directory);
                    fs::create_dir_all(&s_dir)?;
                    fs::write(s_dir.join("SKILL.md"), skill.content)?;
                }

                println!(
                    "{} Installed Goose with {} skills at {}",
                    "✔".green(),
                    skills.len(),
                    skills_dir.display()
                );
            }

            TargetAgent::Zed => {
                let rules_dir = resolve_target_paths(&scope, TargetAgent::Zed);
                fs::create_dir_all(&rules_dir)?;

                for skill in &skills {
                    let rule_file = rules_dir.join(format!("{}.md", skill.id));
                    fs::write(rule_file, skill.content)?;
                }

                println!(
                    "{} Installed Zed Editor with {} rules at {}",
                    "✔".green(),
                    skills.len(),
                    rules_dir.display()
                );
            }

            TargetAgent::Aider => {
                let conventions_dir = resolve_target_paths(&scope, TargetAgent::Aider);
                fs::create_dir_all(&conventions_dir)?;

                for skill in &skills {
                    let conv_file = conventions_dir.join(format!("{}.md", skill.id));
                    fs::write(conv_file, skill.content)?;
                }

                println!(
                    "{} Installed Aider with {} conventions at {}",
                    "✔".green(),
                    skills.len(),
                    conventions_dir.display()
                );
            }
        }
    }

    println!(
        "\n{} Universal plugin packaging and installation completed successfully!",
        "✨".bold().green()
    );

    Ok(())
}
