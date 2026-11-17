use crate::config::load_config;
use crate::config::save_config;
use crate::config::settings::EditorPreference;
use colored::Colorize;
use inquire::{Confirm, Select};
use std::env;
use std::error::Error;
use std::path::Path;
use std::process::Command;

/// Checks if a binary command is executable on PATH.
fn is_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Resolves the binary command and arguments for the chosen editor preference.
pub fn resolve_editor_command(pref: EditorPreference) -> (String, Vec<String>) {
    match pref {
        EditorPreference::Terminal => {
            if let Ok(editor) = env::var("EDITOR") {
                let parts: Vec<&str> = editor.split_whitespace().collect();
                if let Some(cmd) = parts.first() {
                    let args = parts[1..].iter().map(|s| s.to_string()).collect();
                    return (cmd.to_string(), args);
                }
            }
            if is_command_available("nano") {
                ("nano".to_string(), Vec::new())
            } else if is_command_available("vim") {
                ("vim".to_string(), Vec::new())
            } else {
                ("vi".to_string(), Vec::new())
            }
        }
        EditorPreference::Gui => {
            if is_command_available("code") {
                ("code".to_string(), vec!["--wait".to_string()])
            } else if is_command_available("cursor") {
                ("cursor".to_string(), vec!["--wait".to_string()])
            } else if cfg!(target_os = "macos") {
                ("open".to_string(), Vec::new())
            } else if cfg!(target_os = "windows") {
                (
                    "cmd".to_string(),
                    vec!["/c".to_string(), "start".to_string()],
                )
            } else {
                ("xdg-open".to_string(), Vec::new())
            }
        }
        EditorPreference::Ask => resolve_editor_command(EditorPreference::Terminal),
    }
}

/// Dispatches an editor to edit a target file.
/// If preference is Ask, prompts the user and allows persisting the decision in config.toml.
pub fn dispatch_editor(
    file_path: &Path,
    config_dir: &Path,
    non_interactive: bool,
) -> Result<(), Box<dyn Error>> {
    let mut config = load_config(config_dir);
    let mut selected_pref = config.editor.preference;

    if selected_pref == EditorPreference::Ask {
        if non_interactive {
            println!(
                "{}",
                format!(
                    "[Non-interactive] Editor dispatch skipped for {}",
                    file_path.display()
                )
                .yellow()
            );
            return Ok(());
        }

        let options = vec![
            "Terminal Editor (Vim / Nano / $EDITOR)",
            "System Default / GUI (VS Code, Cursor, OS default)",
        ];
        let ans = Select::new("Choose an editor to edit skill:", options).prompt()?;

        selected_pref = if ans.starts_with("Terminal") {
            EditorPreference::Terminal
        } else {
            EditorPreference::Gui
        };

        let remember = Confirm::new("Remember this choice as default?")
            .with_default(false)
            .prompt()?;

        if remember {
            config.editor.preference = selected_pref;
            if let Err(e) = save_config(config_dir, &config) {
                eprintln!(
                    "{}",
                    format!("Warning: failed to save editor preference: {}", e).yellow()
                );
            }
        }
    }

    if non_interactive {
        return Ok(());
    }

    // Auto-backup before edit
    if let Some(skills_dir) = file_path.parent().and_then(|p| p.parent()) {
        let id = file_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !id.is_empty() {
            if let Ok(ts) = crate::skill::rollback::backup_skill(skills_dir, id) {
                println!("{}", format!("Backup created: {}", ts).dimmed());
            }
        }
    }

    let (cmd, args) = resolve_editor_command(selected_pref);
    println!(
        "{}",
        format!("Opening {} with {}...", file_path.display(), cmd).cyan()
    );

    let mut process = Command::new(&cmd);
    process.args(&args);
    process.arg(file_path);

    let status = process.status();
    match status {
        Ok(s) => {
            if !s.success() {
                eprintln!("{}", format!("Editor exited with status: {}", s).yellow());
            }
        }
        Err(e) => {
            eprintln!(
                "{}",
                format!(
                    "Failed to spawn editor '{}': {}. Falling back to vi...",
                    cmd, e
                )
                .red()
            );
            let _ = Command::new("vi").arg(file_path).status();
        }
    }

    Ok(())
}
