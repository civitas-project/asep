//! Target software agent discovery path resolvers and installation scopes.

use std::path::PathBuf;

// @impl:FR-PACK-TARGETS-01
// @impl:FR-PACK-TARGETS-02

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetAgent {
    Antigravity,
    OpenCode,
    ClaudeCode,
    Cursor,
    Windsurf,
    Continue,
    RooCode,
    Copilot,
    Goose,
    Zed,
    Aider,
}

impl TargetAgent {
    pub const ALL: [TargetAgent; 11] = [
        TargetAgent::Antigravity,
        TargetAgent::OpenCode,
        TargetAgent::ClaudeCode,
        TargetAgent::Cursor,
        TargetAgent::Windsurf,
        TargetAgent::Continue,
        TargetAgent::RooCode,
        TargetAgent::Copilot,
        TargetAgent::Goose,
        TargetAgent::Zed,
        TargetAgent::Aider,
    ];

    pub fn all() -> &'static [TargetAgent] {
        &Self::ALL
    }

    pub fn id(&self) -> &'static str {
        match self {
            TargetAgent::Antigravity => "antigravity",
            TargetAgent::OpenCode => "opencode",
            TargetAgent::ClaudeCode => "claudecode",
            TargetAgent::Cursor => "cursor",
            TargetAgent::Windsurf => "windsurf",
            TargetAgent::Continue => "continue",
            TargetAgent::RooCode => "roo",
            TargetAgent::Copilot => "copilot",
            TargetAgent::Goose => "goose",
            TargetAgent::Zed => "zed",
            TargetAgent::Aider => "aider",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TargetAgent::Antigravity => "Google Antigravity (.agents/ / ~/.gemini/)",
            TargetAgent::OpenCode => "OpenCode (.opencode/ / ~/.config/opencode/)",
            TargetAgent::ClaudeCode => "Claude Code / agentskills.io (.claude/)",
            TargetAgent::Cursor => "Cursor AI (.cursor/rules/)",
            TargetAgent::Windsurf => "Windsurf / Codeium (.windsurf/rules/ / ~/.codeium/)",
            TargetAgent::Continue => "Continue (.continue/rules/)",
            TargetAgent::RooCode => "Roo Code / Cline (.roo/rules/)",
            TargetAgent::Copilot => "GitHub Copilot (.github/skills/ / ~/.config/github-copilot/)",
            TargetAgent::Goose => "Goose (.goose/skills/ / ~/.config/goose/)",
            TargetAgent::Zed => "Zed Editor (.zed/rules/ / ~/.config/zed/)",
            TargetAgent::Aider => "Aider (.aider/conventions/)",
        }
    }

    /// Checks whether this software agent is installed / detected on the system.
    pub fn is_detected(&self, scope: &InstallScope) -> bool {
        match scope {
            InstallScope::Local(base) => match self {
                TargetAgent::Antigravity => base.join(".agents").exists(),
                TargetAgent::OpenCode => {
                    base.join(".opencode").exists() || base.join("opencode.json").exists()
                }
                TargetAgent::ClaudeCode => base.join(".claude").exists(),
                TargetAgent::Cursor => {
                    base.join(".cursor").exists() || base.join(".cursorrules").exists()
                }
                TargetAgent::Windsurf => {
                    base.join(".windsurf").exists() || base.join(".codeium").exists()
                }
                TargetAgent::Continue => base.join(".continue").exists(),
                TargetAgent::RooCode => base.join(".roo").exists() || base.join(".cline").exists(),
                TargetAgent::Copilot => base.join(".github").exists(),
                TargetAgent::Goose => base.join(".goose").exists(),
                TargetAgent::Zed => base.join(".zed").exists(),
                TargetAgent::Aider => {
                    base.join(".aider").exists() || base.join(".aider.conf.yml").exists()
                }
            },
            InstallScope::Global(home) => match self {
                TargetAgent::Antigravity => {
                    home.join(".gemini").exists() || is_binary_in_path("agy")
                }
                TargetAgent::OpenCode => {
                    home.join(".config").join("opencode").exists() || is_binary_in_path("opencode")
                }
                TargetAgent::ClaudeCode => {
                    home.join(".claude").exists() || is_binary_in_path("claude")
                }
                TargetAgent::Cursor => {
                    home.join(".cursor").exists()
                        || home.join(".cursor-tutor").exists()
                        || is_binary_in_path("cursor")
                }
                TargetAgent::Windsurf => {
                    home.join(".codeium").exists()
                        || home.join(".windsurf").exists()
                        || is_binary_in_path("windsurf")
                }
                TargetAgent::Continue => {
                    home.join(".continue").exists() || is_binary_in_path("continue")
                }
                TargetAgent::RooCode => home.join(".roo").exists() || home.join(".cline").exists(),
                TargetAgent::Copilot => {
                    home.join(".config").join("github-copilot").exists()
                        || home.join(".copilot").exists()
                        || is_binary_in_path("github-copilot-cli")
                }
                TargetAgent::Goose => {
                    home.join(".config").join("goose").exists()
                        || home.join(".local").join("share").join("goose").exists()
                        || is_binary_in_path("goose")
                }
                TargetAgent::Zed => {
                    home.join(".config").join("zed").exists() || is_binary_in_path("zed")
                }
                TargetAgent::Aider => {
                    home.join(".aider").exists()
                        || home.join(".aider.conf.yml").exists()
                        || is_binary_in_path("aider")
                }
            },
        }
    }
}

impl std::fmt::Display for TargetAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallScope {
    Local(PathBuf),
    Global(PathBuf),
}

/// Helper to check if a binary exists in the system PATH.
pub fn is_binary_in_path(bin_name: &str) -> bool {
    match std::env::var_os("PATH") {
        Some(paths) => {
            for path in std::env::split_paths(&paths) {
                let candidate = path.join(bin_name);
                if candidate.is_file() {
                    return true;
                }
                #[cfg(windows)]
                {
                    let candidate_exe = path.join(format!("{}.exe", bin_name));
                    let candidate_cmd = path.join(format!("{}.cmd", bin_name));
                    if candidate_exe.is_file() || candidate_cmd.is_file() {
                        return true;
                    }
                }
            }
            false
        }
        None => false,
    }
}

/// Resolves the default global user home directory cross-platform.
pub fn default_global_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Resolves the canonical installation destination for a given scope and agent.
pub fn resolve_target_paths(scope: &InstallScope, agent: TargetAgent) -> PathBuf {
    match scope {
        InstallScope::Local(base) => match agent {
            TargetAgent::Antigravity => base.join(".agents").join("plugins").join("asep"),
            TargetAgent::OpenCode => base.join(".opencode").join("skills"),
            TargetAgent::ClaudeCode => base.join(".claude").join("skills"),
            TargetAgent::Cursor => base.join(".cursor").join("rules"),
            TargetAgent::Windsurf => base.join(".windsurf").join("rules"),
            TargetAgent::Continue => base.join(".continue").join("rules"),
            TargetAgent::RooCode => base.join(".roo").join("rules"),
            TargetAgent::Copilot => base.join(".github").join("skills"),
            TargetAgent::Goose => base.join(".goose").join("skills"),
            TargetAgent::Zed => base.join(".zed").join("rules"),
            TargetAgent::Aider => base.join(".aider").join("conventions"),
        },
        InstallScope::Global(home) => match agent {
            TargetAgent::Antigravity => home
                .join(".gemini")
                .join("config")
                .join("plugins")
                .join("asep"),
            TargetAgent::OpenCode => home.join(".config").join("opencode").join("skills"),
            TargetAgent::ClaudeCode => home.join(".claude").join("skills"),
            TargetAgent::Cursor => home.join(".cursor").join("rules"),
            TargetAgent::Windsurf => home.join(".codeium").join("windsurf").join("rules"),
            TargetAgent::Continue => home.join(".continue").join("rules"),
            TargetAgent::RooCode => home.join(".roo").join("rules"),
            TargetAgent::Copilot => home.join(".config").join("github-copilot").join("skills"),
            TargetAgent::Goose => home.join(".config").join("goose").join("skills"),
            TargetAgent::Zed => home.join(".config").join("zed").join("rules"),
            TargetAgent::Aider => home.join(".aider").join("conventions"),
        },
    }
}
