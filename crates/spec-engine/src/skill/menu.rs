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
    let global_asep = crate::skill::distribution::targets::default_global_home()
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join("asep")
        .join("skills");
    if global_asep.exists() {
        return global_asep;
    }

    let global_antigravity = crate::skill::distribution::targets::default_global_home()
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

        // Guard-clause mandate: no if/else; match on bool instead.
        #[allow(clippy::match_bool)]
        let loc_str = match skill.status == SkillStatus::Missing {
            true => "(not on disk)".dimmed().to_string(),
            false => skill.file_path.display().to_string(),
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

/// Tag computed from `choice` with the exact same `starts_with("N.")` predicates
/// used by the original match guards; no guards, no else, no nested ifs here.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuTag {
    List,
    Create,
    Edit,
    Delete,
    Restore,
    Export,
    Import,
    Settings,
    #[allow(dead_code)] // matched in the run loop; menu_tag never returns it (no "9." predicate)
    Exit,
    Unknown,
}

fn menu_tag(choice: &str) -> MenuTag {
    if choice.starts_with("1.") {
        return MenuTag::List;
    }
    if choice.starts_with("2.") {
        return MenuTag::Create;
    }
    if choice.starts_with("3.") {
        return MenuTag::Edit;
    }
    if choice.starts_with("4.") {
        return MenuTag::Delete;
    }
    if choice.starts_with("5.") {
        return MenuTag::Restore;
    }
    if choice.starts_with("6.") {
        return MenuTag::Export;
    }
    if choice.starts_with("7.") {
        return MenuTag::Import;
    }
    if choice.starts_with("8.") {
        return MenuTag::Settings;
    }
    MenuTag::Unknown
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

        match menu_tag(choice) {
            MenuTag::List => list_skills(skills_dir),
            MenuTag::Create => create_skill(skills_dir, config_dir),
            MenuTag::Edit => edit_skill(skills_dir, config_dir),
            MenuTag::Delete => delete_skill_flow(skills_dir),
            MenuTag::Restore => restore_skill_flow(skills_dir),
            MenuTag::Export => export_skill_flow(skills_dir),
            MenuTag::Import => import_skill_flow(skills_dir),
            MenuTag::Settings => settings_flow(config_dir),
            MenuTag::Exit | MenuTag::Unknown => break,
        }
    }

    Ok(())
}

/// Lists installed skills and their drift status.
fn list_skills(skills_dir: &Path) {
    let skills = inspect_skills(skills_dir);
    print_skills_table(&skills);
}

/// Flow for creating a new custom skill (menu option "2.").
fn create_skill(skills_dir: &Path, config_dir: &Path) {
    let id =
        match Text::new("Skill Identifier (e.g. my-custom-skill, or empty to cancel):").prompt() {
            Ok(val) => val.trim().to_string(),
            Err(_) => {
                println!("{}", "Skill creation cancelled.".yellow());
                return;
            }
        };
    if id.is_empty() {
        println!("{}", "Skill creation cancelled.".yellow());
        return;
    }

    let name = match Text::new("Display Name:").with_default(&id).prompt() {
        Ok(val) => val.trim().to_string(),
        Err(_) => {
            println!("{}", "Skill creation cancelled.".yellow());
            return;
        }
    };

    let description = match Text::new("Description:").prompt() {
        Ok(val) => val.trim().to_string(),
        Err(_) => {
            println!("{}", "Skill creation cancelled.".yellow());
            return;
        }
    };

    let confirmed = Confirm::new(&format!("Create custom skill '{}'?", id))
        .with_default(true)
        .prompt()
        .unwrap_or(false);

    if !confirmed {
        println!("{}", "Skill creation cancelled.".yellow());
        return;
    }

    match add_skill(skills_dir, &id, &name, &description, "") {
        Ok(path) => {
            println!(
                "{}",
                format!("Successfully created skill at {}", path.display()).green()
            );
            prompt_open_editor(&path, config_dir);
        }
        Err(e) => eprintln!("{}", format!("Failed to create skill: {}", e).red()),
    }
}

/// Asks whether to open the just-created skill in the editor.
fn prompt_open_editor(path: &Path, config_dir: &Path) {
    let edit_now = Confirm::new("Open skill in editor now?")
        .with_default(true)
        .prompt()
        .unwrap_or(false);
    if edit_now {
        let _ = dispatch_editor(path, config_dir, false);
    }
}

/// Flow for editing an existing skill (menu option "3.").
fn edit_skill(skills_dir: &Path, config_dir: &Path) {
    let skills = inspect_skills(skills_dir);
    let available: Vec<&SkillEntry> = skills
        .iter()
        .filter(|s| s.status != SkillStatus::Missing)
        .collect();

    if available.is_empty() {
        println!("{}", "No active skills available to edit.".yellow());
        return;
    }

    let mut labels: Vec<String> = vec![BACK_TO_MENU.to_string()];
    labels.extend(available.iter().map(|s| format!("{} ({})", s.id, s.name)));

    let selected = match Select::new("Select skill to edit:", labels).prompt() {
        Ok(s) => s,
        Err(_) => return,
    };

    if selected == BACK_TO_MENU {
        return;
    }

    let id = selected.split_whitespace().next().unwrap_or("");
    // Guard-clause mandate: `if let` would nest an `if` inside it (depth 2),
    // so keep the explicit match; the empty None arm is intentional.
    #[allow(clippy::single_match)]
    match available.iter().find(|s| s.id == id) {
        Some(target) => {
            let _ = dispatch_editor(&target.file_path, config_dir, false);
        }
        None => {}
    }
}

/// Flow for deleting a skill (menu option "4.").
fn delete_skill_flow(skills_dir: &Path) {
    let skills = inspect_skills(skills_dir);
    let available: Vec<&SkillEntry> = skills
        .iter()
        .filter(|s| s.status != SkillStatus::Missing)
        .collect();

    if available.is_empty() {
        println!("{}", "No active skills available to delete.".yellow());
        return;
    }

    let mut labels: Vec<String> = vec![BACK_TO_MENU.to_string()];
    labels.extend(
        available
            .iter()
            .map(|s| format!("{} [{:?}]", s.id, s.status)),
    );

    let selected = match Select::new("Select skill to delete:", labels).prompt() {
        Ok(s) => s,
        Err(_) => return,
    };

    if selected == BACK_TO_MENU {
        return;
    }

    let id = selected.split_whitespace().next().unwrap_or("");
    let confirmed = Confirm::new(&format!(
        "Are you sure you want to delete '{}'? (Yes/No)",
        id
    ))
    .with_default(false)
    .prompt()
    .unwrap_or(false);

    if !confirmed {
        println!("{}", "Deletion cancelled.".yellow());
        return;
    }

    match delete_skill(skills_dir, id) {
        Ok(_) => println!("{}", format!("Deleted skill '{}'", id).green()),
        Err(e) => eprintln!("{}", format!("Failed to delete: {}", e).red()),
    }
}

/// Flow for restoring built-in skills to factory defaults (menu option "5.").
fn restore_skill_flow(skills_dir: &Path) {
    let skills = inspect_skills(skills_dir);
    let candidates: Vec<&SkillEntry> = skills
        .iter()
        .filter(|s| s.status == SkillStatus::Modified || s.status == SkillStatus::Missing)
        .collect();

    if candidates.is_empty() {
        println!(
            "{}",
            "All built-in skills are already at factory default.".green()
        );
        return;
    }

    let restore_options = vec![
        "1. Restore ALL built-in skills to factory defaults",
        "2. Select specific skills to restore",
        BACK_TO_MENU,
    ];

    let mode = match Select::new("Factory Restore Mode:", restore_options).prompt() {
        Ok(m) => m,
        Err(_) => return,
    };

    if mode == BACK_TO_MENU {
        return;
    }

    if mode.starts_with("1.") {
        restore_all_flow(skills_dir);
    }
    if mode.starts_with("2.") {
        restore_select_flow(skills_dir, &candidates);
    }
}

/// Restore ALL built-in skills to factory defaults.
fn restore_all_flow(skills_dir: &Path) {
    let confirmed = Confirm::new(
        "Are you sure you want to restore ALL built-in skills to factory defaults? (Yes/No)",
    )
    .with_default(false)
    .prompt()
    .unwrap_or(false);

    if !confirmed {
        println!("{}", "Restoration cancelled.".yellow());
        return;
    }

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
}

/// Restore a user-selected subset of built-in skills.
fn restore_select_flow(skills_dir: &Path, candidates: &[&SkillEntry]) {
    let labels: Vec<String> = candidates
        .iter()
        .map(|s| format!("{} ({:?})", s.id, s.status))
        .collect();

    let default_indices: Vec<usize> = (0..labels.len()).collect();
    let selected = match MultiSelect::new("Select built-in skills to restore:", labels)
        .with_default(&default_indices)
        .prompt()
    {
        Ok(s) => s,
        Err(_) => {
            println!("{}", "Restoration cancelled.".yellow());
            return;
        }
    };

    if selected.is_empty() {
        println!("{}", "No skills selected for restore.".yellow());
        return;
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

    if !confirmed {
        println!("{}", "Restoration cancelled.".yellow());
        return;
    }

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
}

/// Flow for exporting skills to markdown (menu option "6.").
fn export_skill_flow(skills_dir: &Path) {
    let skills = inspect_skills(skills_dir);
    let available: Vec<&SkillEntry> = skills
        .iter()
        .filter(|s| s.status != SkillStatus::Missing)
        .collect();

    if available.is_empty() {
        println!("{}", "No active skills available to export.".yellow());
        return;
    }

    let export_options = vec![
        "1. Export a single skill to Markdown (.md)",
        "2. Export ALL active skills to Markdown files",
        BACK_TO_MENU,
    ];

    let mode = match Select::new("Export Mode:", export_options).prompt() {
        Ok(m) => m,
        Err(_) => return,
    };

    if mode == BACK_TO_MENU {
        return;
    }

    if mode.starts_with("1.") {
        export_single_flow(skills_dir, &available);
    }
    if mode.starts_with("2.") {
        export_all_flow(skills_dir, &available);
    }
}

/// Export a single skill to a markdown file.
fn export_single_flow(skills_dir: &Path, available: &[&SkillEntry]) {
    let mut labels = vec![BACK_TO_MENU.to_string()];
    labels.extend(available.iter().map(|s| s.id.clone()));

    let selected = match Select::new("Select skill to export:", labels).prompt() {
        Ok(s) => s,
        Err(_) => return,
    };

    if selected == BACK_TO_MENU {
        return;
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
            return;
        }
    };

    if dest.is_empty() {
        println!("{}", "Export cancelled.".yellow());
        return;
    }

    let confirmed = Confirm::new(&format!("Export skill '{}' to '{}'? (Yes/No)", id, dest))
        .with_default(true)
        .prompt()
        .unwrap_or(false);

    if !confirmed {
        println!("{}", "Export cancelled.".yellow());
        return;
    }

    match export_skill(skills_dir, &id, Path::new(&dest)) {
        Ok(_) => {
            println!("{}", format!("Exported '{}' to {}", id, dest).green())
        }
        Err(e) => eprintln!("{}", format!("Export failed: {}", e).red()),
    }
}

/// Export all active skills to a directory of markdown files.
fn export_all_flow(skills_dir: &Path, available: &[&SkillEntry]) {
    let dest_dir_str = match Text::new("Destination directory for markdown exports:")
        .with_default("./exported-skills")
        .prompt()
    {
        Ok(d) => d.trim().to_string(),
        Err(_) => {
            println!("{}", "Export cancelled.".yellow());
            return;
        }
    };

    if dest_dir_str.is_empty() {
        println!("{}", "Export cancelled.".yellow());
        return;
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

    if !confirmed {
        println!("{}", "Export cancelled.".yellow());
        return;
    }

    #[allow(clippy::single_match)]
    match fs::create_dir_all(dest_dir) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", format!("Failed to create directory: {}", e).red());
            return;
        }
    }

    let mut count = 0;
    for skill in available {
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
}

/// Flow for importing a skill from markdown (menu option "7.").
fn import_skill_flow(skills_dir: &Path) {
    let source =
        match Text::new("Path to markdown (.md) file to import (or empty to cancel):").prompt() {
            Ok(s) => s.trim().to_string(),
            Err(_) => {
                println!("{}", "Import cancelled.".yellow());
                return;
            }
        };

    if source.is_empty() {
        println!("{}", "Import cancelled.".yellow());
        return;
    }

    let src_path = Path::new(&source);
    if !src_path.exists() {
        eprintln!("{}", format!("File not found: {}", source).red());
        return;
    }

    let confirmed = Confirm::new(&format!("Import skill from '{}'? (Yes/No)", source))
        .with_default(true)
        .prompt()
        .unwrap_or(false);

    if !confirmed {
        println!("{}", "Import cancelled.".yellow());
        return;
    }

    match import_skill(skills_dir, src_path) {
        Ok(id) => println!(
            "{}",
            format!("Imported skill '{}' successfully", id).green()
        ),
        Err(e) => eprintln!("{}", format!("Import failed: {}", e).red()),
    }
}

/// Flow for saving the default editor preference (menu option "8.").
fn settings_flow(config_dir: &Path) {
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
        Err(_) => return,
    };

    if sel == BACK_TO_MENU {
        return;
    }

    // Sequential guard clauses preserve the original starts_with matching:
    // "Terminal ..." -> Terminal, "System ..." -> Gui, anything else -> Ask.
    let mut new_pref = EditorPreference::Ask;
    if sel.starts_with("Terminal") {
        new_pref = EditorPreference::Terminal;
    }
    if sel.starts_with("System") {
        new_pref = EditorPreference::Gui;
    }

    let confirmed = Confirm::new(&format!(
        "Save default editor preference as '{}'? (Yes/No)",
        new_pref
    ))
    .with_default(true)
    .prompt()
    .unwrap_or(false);

    if !confirmed {
        println!("{}", "Preference update cancelled.".yellow());
        return;
    }

    config.editor.preference = new_pref;
    match save_config(config_dir, &config) {
        Ok(_) => println!("{}", format!("Saved preference: {}", new_pref).green()),
        Err(e) => eprintln!("{}", format!("Failed to save config: {}", e).red()),
    }
}
