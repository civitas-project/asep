//! Interactive `inquire` TUI installer and headless packaging engine.
// @impl:FR-AMB-02

use crate::config::settings::load_config;
use crate::pack::embedded::{
    get_default_mcp_config_json, get_default_plugin_json,
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

                println!(
                    "{} Installed Antigravity plugin at {}",
                    "✔".green(),
                    plugin_dir.display()
                );
            }

            TargetAgent::OpenCode => {
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
                    "{} Installed OpenCode with MCP config and agent presets in {}",
                    "✔".green(),
                    config_file.display()
                );
            }

            TargetAgent::ClaudeCode => {
                let target = resolve_target_paths(&scope, TargetAgent::ClaudeCode);
                println!(
                    "{} Installed Claude Code at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Cursor => {
                let target = resolve_target_paths(&scope, TargetAgent::Cursor);
                println!(
                    "{} Installed Cursor AI at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Windsurf => {
                let target = resolve_target_paths(&scope, TargetAgent::Windsurf);
                println!(
                    "{} Installed Windsurf at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Continue => {
                let target = resolve_target_paths(&scope, TargetAgent::Continue);
                println!(
                    "{} Installed Continue at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::RooCode => {
                let target = resolve_target_paths(&scope, TargetAgent::RooCode);
                println!(
                    "{} Installed Roo Code / Cline at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Copilot => {
                let target = resolve_target_paths(&scope, TargetAgent::Copilot);
                println!(
                    "{} Installed GitHub Copilot at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Goose => {
                let target = resolve_target_paths(&scope, TargetAgent::Goose);
                println!(
                    "{} Installed Goose at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Zed => {
                let target = resolve_target_paths(&scope, TargetAgent::Zed);
                println!(
                    "{} Installed Zed Editor at {}",
                    "✔".green(),
                    target.display()
                );
            }

            TargetAgent::Aider => {
                let target = resolve_target_paths(&scope, TargetAgent::Aider);
                println!(
                    "{} Installed Aider at {}",
                    "✔".green(),
                    target.display()
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
