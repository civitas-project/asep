//! Source tree walking with ignore rules, extension/glob filters,
//! circular-symlink guards, and binary-file detection.

use anyhow::Result;
use glob::Pattern;
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::fs::{self, symlink_metadata};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Configuration for a source tree walk.
pub struct WalkConfig {
    pub root: PathBuf,
    pub follow_symlinks: bool,
    pub gitignore: bool,
    pub ignore_file: Option<PathBuf>,
    pub max_depth: Option<usize>,
    pub include_ext: Vec<String>,
    pub exclude_ext: Vec<String>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            follow_symlinks: false,
            gitignore: true,
            ignore_file: None,
            max_depth: None,
            include_ext: Vec::new(),
            exclude_ext: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

impl WalkConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ..Self::default()
        }
    }
}

/// Build an [`ignore::Walk`] from a [`WalkConfig`].
pub fn build_walker(config: &WalkConfig) -> ignore::Walk {
    let mut builder = WalkBuilder::new(&config.root);
    builder.follow_links(config.follow_symlinks);
    builder.ignore(config.gitignore);
    builder.parents(config.gitignore);
    if let Some(path) = &config.ignore_file {
        builder.add_ignore(path);
    }
    builder.max_depth(config.max_depth);
    builder.build()
}

/// Returns true if the first 8KB of `path` contain a NUL byte.
pub fn is_binary_file(path: &Path) -> Result<bool> {
    const LIMIT: usize = 8 * 1024;
    let mut file = fs::File::open(path)?;
    let mut buf = [0u8; LIMIT];
    let mut read = 0;
    while read < buf.len() {
        let n = file.read(&mut buf[read..])?;
        if n == 0 {
            break;
        }
        read += n;
    }
    Ok(buf[..read].contains(&0))
}

/// Walk `config.root` and return the matching regular files.
///
/// Symbolic-link directories are tracked by canonical path so circular
/// symlinks cannot loop. Directories, non-regular files, and binary files
/// are filtered out. Extensions and glob patterns are matched against the
/// path relative to `config.root`.
pub fn walk_files(config: &WalkConfig) -> Result<Vec<PathBuf>> {
    let walker = build_walker(config);
    let inc_ext = &config.include_ext;
    let exc_ext = &config.exclude_ext;
    let inc_pat: Vec<Pattern> = config
        .include_patterns
        .iter()
        .map(|p| Pattern::new(p).map_err(|e| anyhow::anyhow!("invalid include pattern {p:?}: {e}")))
        .collect::<Result<Vec<_>>>()?;
    let exc_pat: Vec<Pattern> = config
        .exclude_patterns
        .iter()
        .map(|p| Pattern::new(p).map_err(|e| anyhow::anyhow!("invalid exclude pattern {p:?}: {e}")))
        .collect::<Result<Vec<_>>>()?;

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out: Vec<PathBuf> = Vec::new();

    for result in walker {
        let entry = result?;
        let path = entry.path();
        let meta = match symlink_metadata(path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        let ft = meta.file_type();

        if ft.is_dir() {
            let canon = match path.canonicalize() {
                Ok(canon) => canon,
                Err(_) => continue,
            };
            seen.insert(canon);
            continue;
        }

        if !ft.is_file() {
            continue;
        }

        let ext = path.extension().map(|ext| ext.to_string_lossy());
        let ext = ext.as_deref().unwrap_or("");
        if !inc_ext.is_empty()
            && !inc_ext
                .iter()
                .any(|e| e.trim_start_matches('.') == ext.trim_start_matches('.'))
        {
            continue;
        }
        if exc_ext
            .iter()
            .any(|e| e.trim_start_matches('.') == ext.trim_start_matches('.'))
        {
            continue;
        }

        let rel = path.strip_prefix(&config.root).unwrap_or(path);
        if !inc_pat.is_empty() && !inc_pat.iter().any(|p| p.matches_path(rel)) {
            continue;
        }
        if exc_pat.iter().any(|p| p.matches_path(rel)) {
            continue;
        }

        if is_binary_file(path)? {
            continue;
        }

        out.push(path.to_path_buf());
    }

    out.sort();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("srcwalk-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn write_file(path: &Path, contents: &[u8]) {
        let mut file = fs::File::create(path).expect("create test file");
        file.write_all(contents).expect("write test file");
    }

    #[test]
    fn walk_plain_files() {
        let dir = test_dir("plain");
        fs::create_dir_all(dir.join("sub")).expect("create subdir");
        write_file(&dir.join("a.rs"), b"fn main() {}\n");
        write_file(&dir.join("b.txt"), b"hello\n");
        write_file(&dir.join("sub/c.rs"), b"fn foo() {}\n");

        let config = WalkConfig::new(&dir);
        let files = walk_files(&config).unwrap();
        assert_eq!(
            files,
            vec![dir.join("a.rs"), dir.join("b.txt"), dir.join("sub/c.rs")]
        );
    }

    #[test]
    fn exclude_ext_and_patterns() {
        let dir = test_dir("exclude");
        write_file(&dir.join("a.rs"), b"fn a() {}\n");
        write_file(&dir.join("b.txt"), b"text\n");
        write_file(&dir.join("c.bin"), b"bin\n");

        let mut config = WalkConfig::new(&dir);
        config.exclude_ext.push("txt".to_string());
        config.exclude_patterns.push("c.bin".to_string());

        let files = walk_files(&config).unwrap();
        assert_eq!(files, vec![dir.join("a.rs")]);
    }

    #[test]
    fn binary_file_detection() {
        let dir = test_dir("binary");
        write_file(&dir.join("text.txt"), b"hello world\n");
        write_file(&dir.join("bin.dat"), &[0u8, 1, 2, 3, 0]);

        assert!(is_binary_file(&dir.join("bin.dat")).unwrap());
        assert!(!is_binary_file(&dir.join("text.txt")).unwrap());

        let config = WalkConfig::new(&dir);
        let files = walk_files(&config).unwrap();
        assert_eq!(files, vec![dir.join("text.txt")]);
    }
}
