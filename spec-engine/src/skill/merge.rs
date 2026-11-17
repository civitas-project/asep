//! 3-way merge resolution for skills.

use similar::TextDiff;

/// Performs a 3-way merge between base, local, and remote content.
/// Returns (merged_string, has_conflict).
pub fn three_way_merge(base: &str, local: &str, remote: &str) -> (String, bool) {
    if local == remote {
        return (local.to_string(), false);
    }
    if local == base {
        return (remote.to_string(), false);
    }
    if remote == base {
        return (local.to_string(), false);
    }

    // Line-by-line diff between base and local, and base and remote
    let local_diff = TextDiff::from_lines(base, local);
    let remote_diff = TextDiff::from_lines(base, remote);

    let base_lines: Vec<&str> = base.lines().collect();
    let local_lines: Vec<&str> = local.lines().collect();
    let remote_lines: Vec<&str> = remote.lines().collect();

    let local_changed = local != base;
    let remote_changed = remote != base;

    if local_changed && !remote_changed {
        return (local.to_string(), false);
    }
    if remote_changed && !local_changed {
        return (remote.to_string(), false);
    }

    // Both changed: check if they changed different parts
    let mut out = String::new();
    let mut has_conflict = false;

    let max_len = base_lines.len().max(local_lines.len()).max(remote_lines.len());
    let mut i = 0;

    while i < max_len {
        let b = base_lines.get(i).copied();
        let l = local_lines.get(i).copied();
        let r = remote_lines.get(i).copied();

        if l == r {
            if let Some(line) = l {
                out.push_str(line);
                out.push('\n');
            }
        } else if l == b {
            if let Some(line) = r {
                out.push_str(line);
                out.push('\n');
            }
        } else if r == b {
            if let Some(line) = l {
                out.push_str(line);
                out.push('\n');
            }
        } else {
            // Conflict
            has_conflict = true;
            out.push_str("<<<<<<< LOCAL\n");
            if let Some(line) = l {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("=======\n");
            if let Some(line) = r {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str(">>>>>>> REMOTE\n");
        }
        i += 1;
    }

    let _ = (local_diff, remote_diff);
    (out, has_conflict)
}

/// Resolves conflict chunks programmatically by passing local and remote blocks to a resolver closure.
pub fn resolve_conflict_chunks<F>(conflicted_text: &str, mut resolver: F) -> String
where
    F: FnMut(&str, &str) -> String,
{
    let mut out = String::new();
    let mut in_conflict = false;
    let mut in_remote = false;
    let mut local_buf = String::new();
    let mut remote_buf = String::new();

    for line in conflicted_text.lines() {
        if line.starts_with("<<<<<<< ") {
            in_conflict = true;
            in_remote = false;
            local_buf.clear();
            remote_buf.clear();
        } else if line.starts_with("=======") && in_conflict {
            in_remote = true;
        } else if line.starts_with(">>>>>>> ") && in_conflict {
            in_conflict = false;
            let resolved = resolver(local_buf.trim_end(), remote_buf.trim_end());
            out.push_str(&resolved);
            out.push('\n');
        } else if in_conflict {
            if in_remote {
                remote_buf.push_str(line);
                remote_buf.push('\n');
            } else {
                local_buf.push_str(line);
                local_buf.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    out
}

/// Interactively prompts the developer via terminal for each conflict chunk.
pub fn interactive_resolve_conflicts(conflicted_text: &str) -> Result<String, String> {
    let mut chunk_idx = 1;
    let resolved = resolve_conflict_chunks(conflicted_text, |local, remote| {
        println!("\nConflict #{}", chunk_idx);
        println!("<<<<<<< LOCAL\n{}\n=======\n{}\n>>>>>>> REMOTE", local, remote);
        chunk_idx += 1;

        let options = vec![
            "Keep LOCAL",
            "Keep REMOTE",
            "Keep BOTH (Local first)",
            "Keep BOTH (Remote first)",
        ];

        let ans = inquire::Select::new("Choose resolution:", options)
            .prompt()
            .unwrap_or("Keep LOCAL");

        match ans {
            "Keep REMOTE" => remote.to_string(),
            "Keep BOTH (Local first)" => format!("{}\n{}", local, remote),
            "Keep BOTH (Remote first)" => format!("{}\n{}", remote, local),
            _ => local.to_string(),
        }
    });

    Ok(resolved)
}
