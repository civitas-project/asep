//! Skill content validation (lint).

use crate::skill::distribution::embedded::get_embedded_skills;
use crate::skill::inspector::extract_skill_full_metadata;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub level: ValidationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    Error,
    Warning,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.level {
            ValidationLevel::Error => "E",
            ValidationLevel::Warning => "W",
        };
        write!(f, "{}: {} ({})", self.code, self.message, prefix)
    }
}

/// Validates a skill's SKILL.md content. Returns list of errors/warnings.
pub fn validate_skill_content(content: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 1. Frontmatter must exist and be closed
    if !content.starts_with("---") {
        errors.push(ValidationError {
            code: "V001".to_string(),
            message: "missing YAML frontmatter (must start with ---)".to_string(),
            level: ValidationLevel::Error,
        });
        return errors;
    }
    let rest = &content[3..];
    let end_idx = match rest.find("\n---") {
        Some(i) => i,
        None => {
            errors.push(ValidationError {
                code: "V002".to_string(),
                message: "frontmatter not closed (missing closing ---)".to_string(),
                level: ValidationLevel::Error,
            });
            return errors;
        }
    };
    let frontmatter = &rest[..end_idx];
    let body = &rest[end_idx + 4..];

    let (name, description, version, depends) = extract_skill_full_metadata(content);

    // 2. name non-empty
    if name.trim().is_empty() {
        errors.push(ValidationError {
            code: "V003".to_string(),
            message: "missing or empty 'name' field".to_string(),
            level: ValidationLevel::Error,
        });
    }

    // 3. description non-empty
    if description.trim().is_empty() {
        errors.push(ValidationError {
            code: "V004".to_string(),
            message: "missing or empty 'description' field".to_string(),
            level: ValidationLevel::Error,
        });
    }
    if !description.trim().is_empty() && description.trim().len() < 20 {
        errors.push(ValidationError {
            code: "V005".to_string(),
            message: format!(
                "description too short ({} chars, min 20)",
                description.trim().len()
            ),
            level: ValidationLevel::Warning,
        });
    }

    // 4. version valid semver
    match &version {
        None => {
            errors.push(ValidationError {
                code: "V007".to_string(),
                message: "missing 'version' field (recommended)".to_string(),
                level: ValidationLevel::Warning,
            });
        }
        Some(v) if !is_semver(v) => {
            errors.push(ValidationError {
                code: "V006".to_string(),
                message: format!("invalid version format '{}', expected X.Y.Z", v),
                level: ValidationLevel::Error,
            });
        }
        Some(_) => {}
    }

    // 5. depends references must exist in embedded skills
    let embedded_ids: Vec<&str> = get_embedded_skills().iter().map(|s| s.id).collect();
    for dep in &depends {
        if !embedded_ids.contains(&dep.as_str()) {
            // allow custom deps? warn only
            errors.push(ValidationError {
                code: "V008".to_string(),
                message: format!("dependency '{}' not found in embedded skills", dep),
                level: ValidationLevel::Warning,
            });
        }
    }

    // 6. body not empty
    if body.trim().len() < 50 {
        errors.push(ValidationError {
            code: "V009".to_string(),
            message: "skill body too short or empty (min 50 chars)".to_string(),
            level: ValidationLevel::Warning,
        });
    }

    // 7. em-dash check (governance) - warn
    if content.contains('\u{2014}') {
        errors.push(ValidationError {
            code: "V010".to_string(),
            message: "em-dash character found (governance: avoid em-dashes)".to_string(),
            level: ValidationLevel::Warning,
        });
    }

    let _ = frontmatter;
    errors
}

fn is_semver(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty())
}

/// Validates and returns whether there are any errors (not warnings).
pub fn has_errors(errors: &[ValidationError]) -> bool {
    errors.iter().any(|e| e.level == ValidationLevel::Error)
}
