//! Semantic version matching and dependency range resolution.

use semver::{Version, VersionReq};

/// Checks if a semver version string matches a requirement range.
pub fn match_version_constraint(version_str: &str, req_str: &str) -> bool {
    let clean_v = version_str.trim().trim_start_matches('v');
    let v = match Version::parse(clean_v) {
        Ok(val) => val,
        Err(_) => return false,
    };

    let req = match VersionReq::parse(req_str.trim()) {
        Ok(r) => r,
        Err(_) => return false,
    };

    req.matches(&v)
}

/// Resolves the highest compatible version from a list of available versions for a given constraint.
pub fn resolve_skill_dependency_version(available: &[String], req_str: &str) -> Option<String> {
    let req = VersionReq::parse(req_str.trim()).ok()?;

    let mut matched_versions: Vec<Version> = available
        .iter()
        .filter_map(|s| {
            let clean = s.trim().trim_start_matches('v');
            Version::parse(clean).ok()
        })
        .filter(|v| req.matches(v))
        .collect();

    matched_versions.sort();
    matched_versions.last().map(|v| v.to_string())
}
