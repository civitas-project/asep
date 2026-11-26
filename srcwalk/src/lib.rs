//! Source tree walking with ignore rules, extension/glob filters,
//! circular-symlink guards, and binary-file detection.

use anyhow::{Error, Result};
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

// @impl:FR-AUD-31
/// Build an [`ignore::Walk`] from a [`WalkConfig`].
///
/// The `gitignore` switch controls every ignore source: `.ignore`,
/// `.gitignore`, `.git/exclude`, the global gitignore, and ignore files in
/// parent directories.
pub fn build_walker(config: &WalkConfig) -> ignore::Walk {
    let mut builder = WalkBuilder::new(&config.root);
    builder.follow_links(config.follow_symlinks);
    builder.ignore(config.gitignore);
    builder.git_ignore(config.gitignore);
    builder.git_global(config.gitignore);
    builder.git_exclude(config.gitignore);
    builder.parents(config.gitignore);
    builder.require_git(false);
    if let Some(path) = &config.ignore_file {
        let _ = builder.add_ignore(path);
    }
    builder.max_depth(config.max_depth);
    builder.build()
}

/// Validate the custom ignore file when one is configured.
///
/// Reads the file and reports unreadable files or invalid pattern lines as
/// errors instead of silently ignoring them.
pub fn validate_ignore_files(config: &WalkConfig) -> Result<(), Error> {
    match &config.ignore_file {
        Some(path) => {
            let contents = fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("cannot read ignore file {path:?}: {e}"))?;
            for line in contents.lines() {
                let pattern = line.trim();
                if pattern.is_empty() || pattern.starts_with('#') {
                    continue;
                }
                glob::Pattern::new(pattern)
                    .map_err(|e| {
                        anyhow::anyhow!("invalid ignore pattern {pattern:?} in {path:?}: {e}")
                    })?;
            }
        }
        None => {},
    }
    Ok(())
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

// @impl:FR-AUD-32
/// Walk `config.root` and return the matching regular files.
///
/// Symbolic links to regular files inside `config.root` are followed when
/// `follow_symlinks` is set and skipped otherwise. Every accepted file is
/// canonicalized; duplicates are pruned with a canonical-path set, paths
/// outside `config.root` are rejected, and special files are refused.
pub fn walk_files(config: &WalkConfig) -> Result<Vec<PathBuf>> {
    validate_ignore_files(config)?;
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

    let root = match config.root.canonicalize() {
        Ok(root) => root,
        Err(_) => config.root.clone(),
    };

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out: Vec<PathBuf> = Vec::new();

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        let meta = match symlink_metadata(path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        let ft = meta.file_type();

        if ft.is_dir() {
            match path.canonicalize() {
                Ok(canon) => {
                    seen.insert(canon);
                }
                Err(_) => {},
            }
            continue;
        }

        if ft.is_symlink() && !config.follow_symlinks {
            continue;
        }

        let canon = match path.canonicalize() {
            Ok(canon) => canon,
            Err(_) => continue,
        };

        if !canon.starts_with(&root) {
            continue;
        }

        let mut canon_ft = ft;
        if ft.is_symlink() {
            canon_ft = match symlink_metadata(&canon) {
                Ok(meta) => meta.file_type(),
                Err(_) => continue,
            };
        }

        if !canon_ft.is_file() {
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

        if !seen.insert(canon) {
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

    // @test:FR-AUD-31
    #[test]
    fn gitignore_honored_only_when_enabled() {
        let dir = test_dir("gitignore");
        write_file(&dir.join(".gitignore"), b"*.log\n");
        write_file(&dir.join("keep.rs"), b"fn main() {}\n");
        write_file(&dir.join("drop.log"), b"junk\n");

        let mut disabled = WalkConfig::new(&dir);
        disabled.gitignore = false;
        let disabled_files = walk_files(&disabled).unwrap();
        assert!(disabled_files.iter().any(|path| path.ends_with("drop.log")));

        let enabled = WalkConfig::new(&dir);
        let enabled_files = walk_files(&enabled).unwrap();
        assert!(enabled_files.iter().any(|path| path.ends_with("keep.rs")));
        assert!(!enabled_files.iter().any(|path| path.ends_with("drop.log")));
    }

    #[test]
    fn unreadable_custom_ignore_file_errors() {
        let dir = test_dir("unreadable");
        write_file(&dir.join("a.rs"), b"fn a() {}\n");
        let missing = dir.join("does-not-exist.ignore");
        let mut config = WalkConfig::new(&dir);
        config.ignore_file = Some(missing);
        let error = walk_files(&config).unwrap_err();
        assert!(error.to_string().contains("does-not-exist.ignore"));
    }

    // @test:FR-AUD-32
    #[test]
    fn symlink_to_internal_regular_file_follows_only_when_enabled() {
        let dir = test_dir("symlink");
        write_file(&dir.join("target.txt"), b"hello\n");
        std::os::unix::fs::symlink(&dir.join("target.txt"), &dir.join("link.rs"))
            .expect("create internal symlink");
        std::os::unix::fs::symlink("/tmp/opencode-does-not-exist", &dir.join("external.rs"))
            .expect("create external symlink");

        let mut disabled = WalkConfig::new(&dir);
        disabled.include_ext.push("rs".to_string());
        let disabled_files = walk_files(&disabled).unwrap();
        assert!(disabled_files.is_empty());

        let mut enabled = WalkConfig::new(&dir);
        enabled.follow_symlinks = true;
        enabled.include_ext.push("rs".to_string());
        let enabled_files = walk_files(&enabled).unwrap();
        assert_eq!(enabled_files, vec![dir.join("link.rs")]);
    }

    #[test]
    fn two_aliases_of_same_canonical_file_produce_one_result() {
        let dir = test_dir("aliases");
        write_file(&dir.join("target.bin"), b"bin\n");
        std::os::unix::fs::symlink(&dir.join("target.bin"), &dir.join("a.rs"))
            .expect("create alias a");
        std::os::unix::fs::symlink(&dir.join("target.bin"), &dir.join("b.rs"))
            .expect("create alias b");

        let mut config = WalkConfig::new(&dir);
        config.follow_symlinks = true;
        config.include_ext.push("rs".to_string());

        let files = walk_files(&config).unwrap();
        assert_eq!(files.len(), 1);
        let name = files[0]
            .file_name()
            .and_then(|name| name.to_str())
            .expect("file name");
        assert!(name == "a.rs" || name == "b.rs");
    }
}
