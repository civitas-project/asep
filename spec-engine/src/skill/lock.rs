//! Skills lock file: records installed skill versions + hashes.

use crate::skill::inspector::inspect_skills;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLockEntry {
    pub id: String,
    pub version: Option<String>,
    pub hash: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLockFile {
    pub generated_at: String,
    pub asep_version: String,
    pub skills: Vec<SkillLockEntry>,
}

const LOCK_FILENAME: &str = "skills.lock";

fn lock_path(skills_dir: &Path) -> std::path::PathBuf {
    skills_dir.join(LOCK_FILENAME)
}

pub fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("sha256:{:x}", result))
}

pub fn generate_lock(skills_dir: &Path) -> Result<SkillLockFile, std::io::Error> {
    let entries = inspect_skills(skills_dir);
    let mut lock_entries = Vec::new();
    for e in &entries {
        let mut hash = "missing".to_string();
        if e.file_path.exists() {
            hash = sha256_file(&e.file_path).unwrap_or_else(|_| "missing".to_string());
        }
        lock_entries.push(SkillLockEntry {
            id: e.id.clone(),
            version: e.version.clone(),
            hash,
            status: e.status.to_string(),
        });
    }
    let lock = SkillLockFile {
        generated_at: chrono_like_now(),
        asep_version: "2.1.0".to_string(),
        skills: lock_entries,
    };
    let json =
        serde_json::to_string_pretty(&lock).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(lock_path(skills_dir), json)?;
    Ok(lock)
}

pub fn read_lock(skills_dir: &Path) -> Option<SkillLockFile> {
    let path = lock_path(skills_dir);
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

#[derive(Debug)]
pub struct LockDrift {
    pub id: String,
    pub kind: String, // "modified", "missing", "new", "version_changed"
    pub detail: String,
}

pub fn verify_lock(skills_dir: &Path) -> Result<Vec<LockDrift>, std::io::Error> {
    let lock = match read_lock(skills_dir) {
        Some(l) => l,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no skills.lock found",
            ))
        }
    };
    let entries = inspect_skills(skills_dir);
    let mut drifts = Vec::new();

    // Check each locked skill
    for le in &lock.skills {
        match entries.iter().find(|e| e.id == le.id) {
            Some(e) => {
                if !e.file_path.exists() {
                    drifts.push(LockDrift {
                        id: le.id.clone(),
                        kind: "missing".to_string(),
                        detail: "file missing on disk".to_string(),
                    });
                }
                let current_hash = sha256_file(&e.file_path).unwrap_or_default();
                if current_hash != le.hash && e.file_path.exists() {
                    drifts.push(LockDrift {
                        id: le.id.clone(),
                        kind: "modified".to_string(),
                        detail: format!("hash changed: {} -> {}", le.hash, current_hash),
                    });
                }
                if e.version != le.version {
                    drifts.push(LockDrift {
                        id: le.id.clone(),
                        kind: "version_changed".to_string(),
                        detail: format!("version changed: {:?} -> {:?}", le.version, e.version),
                    });
                }
            }
            None => {
                drifts.push(LockDrift {
                    id: le.id.clone(),
                    kind: "missing".to_string(),
                    detail: "skill removed".to_string(),
                });
            }
        }
    }

    // Check for new skills not in lock
    for e in &entries {
        if !lock.skills.iter().any(|le| le.id == e.id) {
            drifts.push(LockDrift {
                id: e.id.clone(),
                kind: "new".to_string(),
                detail: "skill not in lock file".to_string(),
            });
        }
    }

    Ok(drifts)
}

fn chrono_like_now() -> String {
    // Simple ISO-ish timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
