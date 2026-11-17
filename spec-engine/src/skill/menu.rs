use crate::config::{load_config, save_config, EditorPreference};
use crate::skill::editor::dispatch_editor;
use crate::skill::inspector::inspect_skills;
use crate::skill::manager::{add_skill, delete_skill, export_skill, import_skill};
use crate::skill::model::{SkillEntry, SkillStatus};
use crate::skill::restore::{restore_all_builtin_skills, restore_skills};
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const BACK_TO_MENU: &str = "← Back to Main Menu";

/// Resolves the candidate skills directory within a project workspace.
pub fn resolve_skills_dir(base_dir: &Path) -> PathBuf {
    let candidates = [
        base_dir
            .join(".agents")
            .join("plugins")
            .join("asep")
            .join("skills"),
        base_dir
            .join(".agents")
            .join("plugins")
            .join("spec-engine")
            .join("skills"),
        base_dir.join(".agents").join("skills"),
        base_dir.join(".opencode").join("skills"),
        base_dir.join(".claude").join("skills"),
        base_dir.join("skills"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    // Fall back to global user Antigravity plugin path if local workspace has no skills installed
    let global_asep = crate::pack::targets::default_global_home()
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join("asep")
        .join("skills");
    if global_asep.exists() {
        return global_asep;
    }

    let global_antigravity = crate::pack::targets::default_global_home()
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join("spec-engine")
        .join("skills");
    if global_antigravity.exists() {
        return global_antigravity;
    }

    // Default to canonical Antigravity plugin path
    candidates[0].clone()
}

/// Prints a formatted, colorized overview table of all skills.
pub fn print_skills_table(skills: &[SkillEntry]) {
    println!("\n{}", "Registered Workspace Skills:".bold().underline());
    println!(
        "{:<25} {:<12} {:<25} {}",
        "ID".bold(),
        "STATUS".bold(),
        "NAME".bold(),
        "LOCATION".bold()
    );
    println!("{}", "-".repeat(80).dimmed());

    if skills.is_empty() {
        println!("{}", "No skills found in workspace.".dimmed());
        return;
    }

    for skill in skills {
        let status_str = match skill.status {
            SkillStatus::Builtin => "Builtin".green(),
            SkillStatus::Modified => "Modified".yellow(),
            SkillStatus::Custom => "Custom".blue(),
            SkillStatus::Missing => "Missing".red(),
        };

        let loc_str = if skill.status == SkillStatus::Missing {
            "(not on disk)".dimmed().to_string()
        } else {
            skill.file_path.display().to_string()
        };

        println!(
            "{:<25} {:<12} {:<25} {}",
            skill.id.cyan(),
            status_str,
            skill.name,
            loc_str
        );
    }
    println!();
}

/// Runs the interactive TUI menu for managing workspace skills.
pub fn run_interactive_skill_menu(
    skills_dir: &Path,
    config_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(skills_dir)?;

    loop {
        let options = vec![
            "1. List skills & status",
            "2. Create new custom skill",
            "3. Edit skill in editor",
            "4. Delete skill",
            "5. Restore built-in skills to factory defaults",
            "6. Export skill to markdown (.md)",
            "7. Import skill from markdown (.md)",
            "8. Settings (editor preference)",
            "9. Exit",
        ];

        let choice = match Select::new("Spec-Engine Skill Lifecycle Manager", options).prompt() {
            Ok(c) => c,
            Err(_) => {
                // If running in headless or non-TTY environment, list and break
                let skills = inspect_skills(skills_dir);
                print_skills_table(&skills);
                return Ok(());
            }
        };

        match choice {
            c if c.starts_with("1.") => {
                let skills = inspect_skills(skills_dir);
                print_skills_table(&skills);
            }
            c if c.starts_with("2.") => {
                let id =
                    match Text::new("Skill Identifier (e.g. my-custom-skill, or empty to cancel):")
                        .prompt()
                    {
                        Ok(val) => val.trim().to_string(),
                        Err(_) => {
                            println!("{}", "Skill creation cancelled.".yellow());
                            continue;
                        }
                    };

                if id.is_empty() {
                    println!("{}", "Skill creation cancelled.".yellow());
                    continue;
                }

                let name = match Text::new("Display Name:").with_default(&id).prompt() {
                    Ok(val) => val.trim().to_string(),
                    Err(_) => {
                        println!("{}", "Skill creation cancelled.".yellow());
                        continue;
                    }
                };

                let description = match Text::new("Description:").prompt() {
                    Ok(val) => val.trim().to_string(),
                    Err(_) => {
                        println!("{}", "Skill creation cancelled.".yellow());
                        continue;
                    }
                };

                let confirmed = Confirm::new(&format!("Create custom skill '{}'?", id))
                    .with_default(true)
                    .prompt()
                    .unwrap_or(false);

                if !confirmed {
                    println!("{}", "Skill creation cancelled.".yellow());
                    continue;
                }

                match add_skill(skills_dir, &id, &name, &description, "") {
                    Ok(path) => {
                        println!(
                            "{}",
                            format!("Successfully created skill at {}", path.display()).green()
                        );
                        let edit_now = Confirm::new("Open skill in editor now?")
                            .with_default(true)
                            .prompt()
                            .unwrap_or(false);
                        if edit_now {
                            let _ = dispatch_editor(&path, config_dir, false);
                        }
                    }
                    Err(e) => eprintln!("{}", format!("Failed to create skill: {}", e).red()),
                }
            }
            c if c.starts_with("3.") => {
                let skills = inspect_skills(skills_dir);
                let available: Vec<&SkillEntry> = skills
                    .iter()
                    .filter(|s| s.status != SkillStatus::Missing)
                    .collect();

                if available.is_empty() {
                    println!("{}", "No active skills available to edit.".yellow());
                    continue;
                }

                let mut labels: Vec<String> = vec![BACK_TO_MENU.to_string()];
                labels.extend(available.iter().map(|s| format!("{} ({})", s.id, s.name)));

                let selected = match Select::new("Select skill to edit:", labels).prompt() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                if selected == BACK_TO_MENU {
                    continue;
                }

                let id = selected.split_whitespace().next().unwrap_or("");
                if let Some(target) = available.iter().find(|s| s.id == id) {
                    let _ = dispatch_editor(&target.file_path, config_dir, false);
                }
            }
            c if c.starts_with("4.") => {
                let skills = inspect_skills(skills_dir);
                let available: Vec<&SkillEntry> = skills
                    .iter()
                    .filter(|s| s.status != SkillStatus::Missing)
                    .collect();

                if available.is_empty() {
                    println!("{}", "No active skills available to delete.".yellow());
                    continue;
                }

                let mut labels: Vec<String> = vec![BACK_TO_MENU.to_string()];
                labels.extend(
                    available
                        .iter()
                        .map(|s| format!("{} [{:?}]", s.id, s.status)),
                );

                let selected = match Select::new("Select skill to delete:", labels).prompt() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                if selected == BACK_TO_MENU {
                    continue;
                }

                let id = selected.split_whitespace().next().unwrap_or("");
                let confirmed = Confirm::new(&format!(
                    "Are you sure you want to delete '{}'? (Yes/No)",
                    id
                ))
                .with_default(false)
                .prompt()
                .unwrap_or(false);

                if confirmed {
                    match delete_skill(skills_dir, id) {
                        Ok(_) => println!("{}", format!("Deleted skill '{}'", id).green()),
                        Err(e) => eprintln!("{}", format!("Failed to delete: {}", e).red()),
                    }
                } else {
                    println!("{}", "Deletion cancelled.".yellow());
                }
            }
            c if c.starts_with("5.") => {
                let skills = inspect_skills(skills_dir);
                let candidates: Vec<&SkillEntry> = skills
                    .iter()
                    .filter(|s| {
                        s.status == SkillStatus::Modified || s.status == SkillStatus::Missing
                    })
                    .collect();

                if candidates.is_empty() {
                    println!(
                        "{}",
                        "All built-in skills are already at factory default.".green()
                    );
                    continue;
                }

                let restore_options = vec![
                    "1. Restore ALL built-in skills to factory defaults",
                    "2. Select specific skills to restore",
                    BACK_TO_MENU,
                ];

                let mode = match Select::new("Factory Restore Mode:", restore_options).prompt() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if mode == BACK_TO_MENU {
                    continue;
                }

                if mode.starts_with("1.") {
                    let confirmed = Confirm::new("Are you sure you want to restore ALL built-in skills to factory defaults? (Yes/No)")
                        .with_default(false)
                        .prompt()
                        .unwrap_or(false);

                    if confirmed {
                        match restore_all_builtin_skills(skills_dir) {
                            Ok(restored) => println!(
                                "{}",
                                format!(
                                    "Restored {} skill(s) to factory defaults: {}",
                                    restored.len(),
                                    restored.join(", ")
                                )
                                .green()
                            ),
                            Err(e) => eprintln!("{}", format!("Failed to restore: {}", e).red()),
                        }
                    } else {
                        println!("{}", "Restoration cancelled.".yellow());
                    }
                } else if mode.starts_with("2.") {
                    let labels: Vec<String> = candidates
                        .iter()
                        .map(|s| format!("{} ({:?})", s.id, s.status))
                        .collect();

                    let default_indices: Vec<usize> = (0..labels.len()).collect();
                    let selected =
                        match MultiSelect::new("Select built-in skills to restore:", labels)
                            .with_default(&default_indices)
                            .prompt()
                        {
                            Ok(s) => s,
                            Err(_) => {
                                println!("{}", "Restoration cancelled.".yellow());
                                continue;
                            }
                        };

                    if selected.is_empty() {
                        println!("{}", "No skills selected for restore.".yellow());
                        continue;
                    }

                    let ids: Vec<&str> = selected
                        .iter()
                        .map(|s| s.split_whitespace().next().unwrap_or(""))
                        .collect();

                    let confirmed = Confirm::new(&format!(
                        "Are you sure you want to restore {} selected skill(s) ({}) to factory defaults? (Yes/No)",
                        ids.len(),
                        ids.join(", ")
                    ))
                    .with_default(false)
                    .prompt()
                    .unwrap_or(false);

                    if confirmed {
                        match restore_skills(skills_dir, &ids) {
                            Ok(restored) => println!(
                                "{}",
                                format!(
                                    "Restored {} skill(s): {}",
                                    restored.len(),
                                    restored.join(", ")
                                )
                                .green()
                            ),
                            Err(e) => eprintln!("{}", format!("Failed to restore: {}", e).red()),
                        }
                    } else {
                        println!("{}", "Restoration cancelled.".yellow());
                    }
                }
            }
            c if c.starts_with("6.") => {
                let skills = inspect_skills(skills_dir);
                let available: Vec<&SkillEntry> = skills
                    .iter()
                    .filter(|s| s.status != SkillStatus::Missing)
                    .collect();

                if available.is_empty() {
                    println!("{}", "No active skills available to export.".yellow());
                    continue;
                }

                let export_options = vec![
                    "1. Export a single skill to Markdown (.md)",
                    "2. Export ALL active skills to Markdown files",
                    BACK_TO_MENU,
                ];

                let mode = match Select::new("Export Mode:", export_options).prompt() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if mode == BACK_TO_MENU {
                    continue;
                }

                if mode.starts_with("1.") {
                    let mut labels = vec![BACK_TO_MENU.to_string()];
                    labels.extend(available.iter().map(|s| s.id.clone()));

                    let selected = match Select::new("Select skill to export:", labels).prompt() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    if selected == BACK_TO_MENU {
                        continue;
                    }

                    let id = selected;
                    let default_path = format!("{}.md", id);
                    let dest = match Text::new("Destination file path:")
                        .with_default(&default_path)
                        .prompt()
                    {
                        Ok(d) => d.trim().to_string(),
                        Err(_) => {
                            println!("{}", "Export cancelled.".yellow());
                            continue;
                        }
                    };

                    if dest.is_empty() {
                        println!("{}", "Export cancelled.".yellow());
                        continue;
                    }

                    let confirmed =
                        Confirm::new(&format!("Export skill '{}' to '{}'? (Yes/No)", id, dest))
                            .with_default(true)
                            .prompt()
                            .unwrap_or(false);

                    if confirmed {
                        match export_skill(skills_dir, &id, Path::new(&dest)) {
                            Ok(_) => {
                                println!("{}", format!("Exported '{}' to {}", id, dest).green())
                            }
                            Err(e) => eprintln!("{}", format!("Export failed: {}", e).red()),
                        }
                    } else {
                        println!("{}", "Export cancelled.".yellow());
                    }
                } else if mode.starts_with("2.") {
                    let dest_dir_str =
                        match Text::new("Destination directory for markdown exports:")
                            .with_default("./exported-skills")
                            .prompt()
                        {
                            Ok(d) => d.trim().to_string(),
                            Err(_) => {
                                println!("{}", "Export cancelled.".yellow());
                                continue;
                            }
                        };

                    if dest_dir_str.is_empty() {
                        println!("{}", "Export cancelled.".yellow());
                        continue;
                    }

                    let dest_dir = Path::new(&dest_dir_str);
                    let confirmed = Confirm::new(&format!(
                        "Export all {} active skills to directory '{}'? (Yes/No)",
                        available.len(),
                        dest_dir.display()
                    ))
                    .with_default(true)
                    .prompt()
                    .unwrap_or(false);

                    if confirmed {
                        if let Err(e) = fs::create_dir_all(dest_dir) {
                            eprintln!("{}", format!("Failed to create directory: {}", e).red());
                            continue;
                        }

                        let mut count = 0;
                        for skill in &available {
                            let out_path = dest_dir.join(format!("{}.md", skill.id));
                            if export_skill(skills_dir, &skill.id, &out_path).is_ok() {
                                count += 1;
                            }
                        }
                        println!(
                            "{}",
                            format!(
                                "Successfully exported {} skill(s) into '{}'",
                                count,
                                dest_dir.display()
                            )
                            .green()
                        );
                    } else {
                        println!("{}", "Export cancelled.".yellow());
                    }
                }
            }
            c if c.starts_with("7.") => {
                let source =
                    match Text::new("Path to markdown (.md) file to import (or empty to cancel):")
                        .prompt()
                    {
                        Ok(s) => s.trim().to_string(),
                        Err(_) => {
                            println!("{}", "Import cancelled.".yellow());
                            continue;
                        }
                    };

                if source.is_empty() {
                    println!("{}", "Import cancelled.".yellow());
                    continue;
                }

                let src_path = Path::new(&source);
                if !src_path.exists() {
                    eprintln!("{}", format!("File not found: {}", source).red());
                    continue;
                }

                let confirmed = Confirm::new(&format!("Import skill from '{}'? (Yes/No)", source))
                    .with_default(true)
                    .prompt()
                    .unwrap_or(false);

                if confirmed {
                    match import_skill(skills_dir, src_path) {
                        Ok(id) => println!(
                            "{}",
                            format!("Imported skill '{}' successfully", id).green()
                        ),
                        Err(e) => eprintln!("{}", format!("Import failed: {}", e).red()),
                    }
                } else {
                    println!("{}", "Import cancelled.".yellow());
                }
            }
            c if c.starts_with("8.") => {
                let mut config = load_config(config_dir);
                println!(
                    "Current editor preference: {}",
                    config.editor.preference.to_string().cyan()
                );

                let options = vec![
                    "Always Ask (prompt every time)",
                    "Terminal Editor ($EDITOR / nano / vim)",
                    "System Default / GUI (VS Code / Cursor / OS default)",
                    BACK_TO_MENU,
                ];

                let sel = match Select::new("Select default editor preference:", options).prompt() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                if sel == BACK_TO_MENU {
                    continue;
                }

                let new_pref = if sel.starts_with("Terminal") {
                    EditorPreference::Terminal
                } else if sel.starts_with("System") {
                    EditorPreference::Gui
                } else {
                    EditorPreference::Ask
                };

                let confirmed = Confirm::new(&format!(
                    "Save default editor preference as '{}'? (Yes/No)",
                    new_pref
                ))
                .with_default(true)
                .prompt()
                .unwrap_or(false);

                if confirmed {
                    config.editor.preference = new_pref;
                    match save_config(config_dir, &config) {
                        Ok(_) => println!("{}", format!("Saved preference: {}", new_pref).green()),
                        Err(e) => eprintln!("{}", format!("Failed to save config: {}", e).red()),
                    }
                } else {
                    println!("{}", "Preference update cancelled.".yellow());
                }
            }
            _ => break,
        }
    }

    Ok(())
}
