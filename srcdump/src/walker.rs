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
        if config.follow_symlinks {
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.file_type().is_symlink() {
                    // Check circular symlinks via canonical path tracking
                    if let Ok(canonical) = path.canonicalize() {
                        let canonical_str = canonical.to_string_lossy().to_string();
                        if !visited_paths.insert(canonical_str) {
                            eprintln!("Warning: Circular symlink detected, skipping: {}", path.display());
                            continue;
                        }
                    }

                    // Check if symlink target escapes root
                    if let Ok(target) = fs::read_link(path) {
                        let target_path = if target.is_absolute() {
                            target
                        } else {
                            path.parent().unwrap_or(Path::new("")).join(target)
                        };
                        if let Ok(canonical_target) = target_path.canonicalize() {
                            if let Ok(canonical_root) = config.root.canonicalize() {
                                if !canonical_target.starts_with(&canonical_root) {
                                    eprintln!("Warning: Symlink escapes root, skipping: {}", path.display());
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !config.is_extension_included(ext) {
                continue;
            }
        } else {
            // No extension - check if it's in include patterns
            if !config.include_patterns.is_empty() {
                let path_str = path.strip_prefix(&config.root).unwrap_or(path).to_string_lossy();
                if !include_patterns.iter().any(|p| p.matches(&path_str)) {
                    continue;
                }
            }
        }

        // Check include patterns
        if !config.include_patterns.is_empty() {
            let path_str = path.strip_prefix(&config.root).unwrap_or(path).to_string_lossy();
            if !include_patterns.iter().any(|p| p.matches(&path_str)) {
                continue;
            }
        }

        // Check exclude patterns (glob) - post-filter
        let rel_path = path.strip_prefix(&config.root).unwrap_or(path).to_string_lossy();
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
