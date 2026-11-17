use crate::error::AsepError;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static ADR_FILENAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-(.*)\.md$").expect("valid adr regex"));

// @impl:FR-CLI-ADR-01
/// Computes next sequential number and scaffolds a 5-part Rich ADR document.
///
/// # Errors: Returns `AsepError::Io` if directory creation or file write fails.
pub fn create_new_adr(project_root: &Path, title: &str) -> Result<PathBuf, AsepError> {
    let adr_dir = project_root.join("docs").join("adr");
    fs::create_dir_all(&adr_dir)?;

    // 1. Find max existing ADR number
    let mut max_num = 0;
    if let Ok(entries) = fs::read_dir(&adr_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if let Some(caps) = ADR_FILENAME_REGEX.captures(file_name) {
                    if let Some(num_str) = caps.get(1) {
                        if let Ok(num) = num_str.as_str().parse::<u32>() {
                            if num > max_num {
                                max_num = num;
                            }
                        }
                    }
                }
            }
        }
    }

    let next_num = max_num + 1;
    let slug = slugify(title);
    let filename = format!("{:04}-{}.md", next_num, slug);
    let target_path = adr_dir.join(filename);

    let template = format!(
        r#"# ADR-{:04}: {}

- **Status:** Proposed
- **Date:** {}
- **Deciders:** Technical Architecture Board
- **Traceability:** @adr:{:04}

---

## 1. Context & Problem Statement
<!-- What is the technical context, operational limitation, or business driver requiring this decision? -->

---

## 2. Decision Drivers
- Performance, latency, and memory safety requirements
- Integration compatibility with existing system architecture
- Developer ergonomics and maintenance sustainability

---

## 3. Considered Alternatives
- **Alternative 1:** [Description & reason for rejection]
- **Alternative 2:** [Description & reason for rejection]

---

## 4. Decision Outcome
Chosen option: **{}**, because [primary justification].

---

## 5. Pros and Cons of the Options

### Positive Consequences
- [Positive consequence 1]
- [Positive consequence 2]

### Negative Consequences / Trade-offs
- [Trade-off or new operational constraint 1]
"#,
        next_num, title, "2026-10-15", next_num, title
    );

    fs::write(&target_path, template)?;

    Ok(target_path)
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for c in title.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}
