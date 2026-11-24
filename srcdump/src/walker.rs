//! File walking logic for srcdump.

use anyhow::Result;
use glob::Pattern;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use srcwalk::{build_walker, is_binary_file, WalkConfig};

/// Walk files according to config.
pub fn walk_files(config: &Config) -> Result<Vec<PathBuf>> {
    let walk_config = WalkConfig {
        root: config.root.clone(),
        follow_symlinks: config.follow_symlinks,
        gitignore: config.gitignore,
        ignore_file: config.ignore_file.clone(),
        max_depth: None,
        include_ext: config.include_ext.clone(),
        exclude_ext: config.exclude_ext.clone(),
        include_patterns: config.include_patterns.clone(),
        exclude_patterns: config.exclude_patterns.clone(),
    };
    let walker = build_walker(&walk_config);

    let mut files = Vec::new();
    let mut visited_paths = HashSet::new();

    // Pre-compile exclude patterns for post-filtering
    let exclude_patterns: Vec<Pattern> = config
        .exclude_patterns
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();
    let include_patterns: Vec<Pattern> = config
        .include_patterns
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Walk error: {}", err);
                continue;
            }
        };

        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Check symlink safety (cross-platform)
        if symlink_should_skip(path, config, &mut visited_paths) {
            continue;
        }

        // Check extension
        if extension_should_skip(path, config, &include_patterns) {
            continue;
        }

        // Check include patterns
        let path_str = path
            .strip_prefix(&config.root)
            .unwrap_or(path)
            .to_string_lossy();
        if !config.include_patterns.is_empty()
            && !include_patterns.iter().any(|p| p.matches(&path_str))
        {
            continue;
        }

        // Check exclude patterns (glob) - post-filter
        let rel_path = path
            .strip_prefix(&config.root)
            .unwrap_or(path)
            .to_string_lossy();
        if exclude_patterns.iter().any(|p| p.matches(&rel_path)) {
            continue;
        }

        // Skip binary files (contains NUL bytes)
        if is_binary_file(path).unwrap_or(true) {
            continue;
        }

        files.push(path.to_path_buf());
    }

    // Sort for deterministic output
    files.sort();
    Ok(files)
}

/// Returns true when the symlink at `path` must be skipped.
fn symlink_should_skip(path: &Path, config: &Config, visited_paths: &mut HashSet<String>) -> bool {
    if !config.follow_symlinks {
        return false;
    }
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if !metadata.file_type().is_symlink() {
        return false;
    }

    // Check circular symlinks via canonical path tracking
    let mut circular = false;
    if let Ok(canonical) = path.canonicalize() {
        let canonical_str = canonical.to_string_lossy().to_string();
        circular = !visited_paths.insert(canonical_str);
    }
    if circular {
        eprintln!(
            "Warning: Circular symlink detected, skipping: {}",
            path.display()
        );
        return true;
    }

    // Check if symlink target escapes root
    let target = match fs::read_link(path) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let target_path = path.parent().unwrap_or(Path::new("")).join(target);
    let canonical_target = match target_path.canonicalize() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let canonical_root = match config.root.canonicalize() {
        Ok(c) => c,
        Err(_) => return false,
    };
    if !canonical_target.starts_with(&canonical_root) {
        eprintln!(
            "Warning: Symlink escapes root, skipping: {}",
            path.display()
        );
        return true;
    }
    false
}

/// Returns true when `path` must be skipped due to extension/include rules.
fn extension_should_skip(path: &Path, config: &Config, include_patterns: &[Pattern]) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => !config.is_extension_included(ext),
        None => {
            // No extension - check if it's in include patterns
            if config.include_patterns.is_empty() {
                return false;
            }
            let path_str = path
                .strip_prefix(&config.root)
                .unwrap_or(path)
                .to_string_lossy();
            !include_patterns.iter().any(|p| p.matches(&path_str))
        }
    }
}
