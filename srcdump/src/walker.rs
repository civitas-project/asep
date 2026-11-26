//! File walking logic for srcdump.

use anyhow::Result;
use glob::Pattern;
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
        let safe = match is_safe_entry(path, &config.root, config.follow_symlinks) {
            Ok(safe) => safe,
            Err(err) => {
                eprintln!("Walk safety check skipped {}: {}", path.display(), err);
                false
            }
        };
        if !safe {
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

// @impl:FR-AUD-30
pub fn is_safe_entry(path: &Path, root: &Path, follow: bool) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            return Err(format!(
                "cannot read metadata for {}: {}",
                path.display(),
                err
            ))
        }
    };
    let file_type = metadata.file_type();

    // Symlinks are not followed by default.
    if file_type.is_symlink() && !follow {
        return Ok(false);
    }
    if file_type.is_symlink() {
        return symlink_target_is_safe(path, root);
    }
    if !file_type.is_file() && !file_type.is_dir() {
        return Ok(false);
    }
    Ok(true)
}

/// Canonicalize the symlink target and require it to be a regular file or
/// directory inside `root`. Broken or circular links yield `Ok(false)`
/// instead of hanging.
fn symlink_target_is_safe(path: &Path, root: &Path) -> Result<bool, String> {
    let target = match fs::read_link(path) {
        Ok(target) => target,
        Err(err) => {
            return Err(format!(
                "cannot read symlink target for {}: {}",
                path.display(),
                err
            ))
        }
    };
    let target_path = path.parent().unwrap_or(Path::new("")).join(target);
    let canonical_target = match target_path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => return Ok(false),
    };
    let canonical_root = match root.canonicalize() {
        Ok(canonical) => canonical,
        Err(err) => {
            return Err(format!(
                "cannot canonicalize root {}: {}",
                root.display(),
                err
            ))
        }
    };
    if !is_within_root(&canonical_target, &canonical_root) {
        return Ok(false);
    }
    let target_type = match fs::symlink_metadata(&canonical_target) {
        Ok(metadata) => metadata.file_type(),
        Err(_) => return Ok(false),
    };
    if !target_type.is_file() && !target_type.is_dir() {
        return Ok(false);
    }
    Ok(true)
}

/// Component-wise prefix check: `target` must live inside `root`.
fn is_within_root(target: &Path, root: &Path) -> bool {
    target.starts_with(root)
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

#[cfg(test)]
mod tests {
    use super::*;

    // @test:FR-AUD-30
    #[test]
    fn symlink_safety_defaults_to_not_following() {
        let root = std::env::temp_dir().join(format!(
            "srcdump-fr-aud-30-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test root");

        let target = root.join("target.txt");
        fs::write(&target, b"hello").expect("write target file");

        let link = root.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("create internal symlink");

        // Default: follow = false → symlink is skipped.
        assert_eq!(is_safe_entry(&link, &root, false), Ok(false));

        // Internal symlink target accepted only when follow = true.
        assert_eq!(is_safe_entry(&link, &root, true), Ok(true));

        // External symlink target rejected even when follow = true.
        let external = root.join("external.txt");
        std::os::unix::fs::symlink("/tmp/opencode", &external).expect("create external symlink");
        assert_eq!(is_safe_entry(&external, &root, true), Ok(false));

        // Circular symlink handled without hanging.
        let circular = root.join("circular.txt");
        std::os::unix::fs::symlink(&circular, &circular).expect("create circular symlink");
        assert_eq!(is_safe_entry(&circular, &root, true), Ok(false));

        // Directory alias rejected when follow = false.
        let sub = root.join("sub");
        fs::create_dir_all(&sub).expect("create subdir");
        let alias = root.join("alias.txt");
        std::os::unix::fs::symlink(&sub, &alias).expect("create directory alias");
        assert_eq!(is_safe_entry(&alias, &root, false), Ok(false));

        // Regular file still accepted.
        assert_eq!(is_safe_entry(&target, &root, false), Ok(true));

        let _ = fs::remove_dir_all(&root);
    }
}
