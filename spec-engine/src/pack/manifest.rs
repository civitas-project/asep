// @impl:FR-INT-01
//! Manifest generation and configuration merging for Antigravity and OpenCode.
// @impl:FR-CAP-06 // @impl:FR-CAP-07

use crate::config::settings::CapabilityProfile;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// @impl:FR-PACK-MANIFEST-01 @impl:FR-PACK-01
// @impl:FR-PACK-MANIFEST-02

/// Generates the canonical `plugin.json` manifest for Antigravity plugins.
pub fn generate_plugin_json() -> String {
    serde_json::json!({
        "name": "asep",
        "description": "Axiomatic Systems Engineering Protocol (ASEP): Deterministic SDLC validator, traceability graph checker, and living spec engine"
    })
    .to_string()
}

/// Generates the canonical `mcp_config.json` manifest.
pub fn generate_mcp_config_json(command: &str) -> String {
    serde_json::json!({
        "mcpServers": {
            "asep": {
                "command": command,
                "args": ["mcp"]
            }
        }
    })
    .to_string()
}

/// Creates the parent directory of `config_path` when it can be determined,
/// without failing when there is no parent (e.g. a bare file name).
fn ensure_parent_dir(config_path: &Path) -> std::io::Result<()> {
    match config_path.parent() {
        Some(parent) => fs::create_dir_all(parent),
        None => Ok(()),
    }
}

/// Merges `asep` MCP configuration into an existing or new `opencode.json`.
///
/// Safety: reads and parses the existing file first; if parsing fails, the
/// original content is preserved in a `.bak` file and an error is returned
/// instead of clobbering the user's configuration. A backup is also written
/// before any successful write.
pub fn update_opencode_json(config_path: &Path, command: &str) -> Result<(), std::io::Error> {
    // Read existing content (if any) without losing it.
    let existing_raw = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(_) if !config_path.exists() => {
            ensure_parent_dir(config_path)?;
            String::new()
        }
        Err(e) => return Err(e),
    };

    // Parse existing JSON. If the file is non-empty but unparseable, back up
    // the raw content and return an error instead of overwriting.
    let mut config: serde_json::Value = serde_json::json!({});
    if !existing_raw.trim().is_empty() {
        match serde_json::from_str(&existing_raw) {
            Ok(v) => config = v,
            Err(e) => {
                // Preserve the user's file verbatim in a backup.
                let backup_path = config_path.with_extension("json.bak");
                let _ = fs::write(&backup_path, existing_raw.as_bytes());
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "existing opencode.json is not valid JSON ({}); backup written to {}",
                        e,
                        backup_path.display()
                    ),
                ));
            }
        }
    }

    if !config.is_object() {
        config = serde_json::json!({});
    }

    // Ensure mcp is an object without clobbering existing entries.
    if config.get("mcp").is_none() || !config["mcp"].is_object() {
        config["mcp"] = serde_json::json!({});
    }

    // Remove legacy `mcp.asep` entry written by a previous merge, if present.
    config
        .get_mut("mcp")
        .and_then(|mcp| mcp.as_object_mut())
        .map(|mcp_obj| mcp_obj.remove("asep"));

    // Ensure mcp.servers is an object without clobbering existing server
    // entries, then write the ASEP server in OpenCode v2 shape: command is
    // the array [command, "mcp"].
    let mcp_obj = config
        .get_mut("mcp")
        .and_then(|v| v.as_object_mut())
        .expect("mcp value is an object");
    if mcp_obj.get("servers").is_none() || !mcp_obj["servers"].is_object() {
        mcp_obj.insert("servers".to_string(), serde_json::json!({}));
    }
    let servers_obj = mcp_obj
        .get_mut("servers")
        .and_then(|v| v.as_object_mut())
        .expect("servers is an object");
    servers_obj.insert(
        "asep".to_string(),
        serde_json::json!({
            "type": "local",
            "command": [command, "mcp"],
        }),
    );

    // Back up the original content before overwriting (best effort).
    if !existing_raw.is_empty() {
        let backup_path = config_path.with_extension("json.bak");
        let _ = fs::write(&backup_path, existing_raw.as_bytes());
    }

    let formatted = serde_json::to_string_pretty(&config).map_err(std::io::Error::other)?;

    // Sanity check: the merged result must still contain any previously
    // present top-level keys (e.g. "provider", "model", "plugins").
    check_no_dropped_keys(config_path, &existing_raw, &config)?;

    fs::write(config_path, formatted)?;
    Ok(())
}

/// Verifies the merged config still contains every top-level key present in
/// the original input; aborts with a backup if any key would be silently dropped.
fn check_no_dropped_keys(
    config_path: &Path,
    existing_raw: &str,
    config: &serde_json::Value,
) -> Result<(), std::io::Error> {
    if existing_raw.trim().is_empty() {
        return Ok(());
    }
    let prev = match serde_json::from_str::<serde_json::Value>(existing_raw) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    let prev_obj = match prev.as_object() {
        Some(o) => o,
        None => return Ok(()),
    };
    let new_obj = match config.as_object() {
        Some(o) => o,
        None => return Ok(()),
    };
    for key in prev_obj.keys() {
        if !new_obj.contains_key(key) {
            // Should not happen, but guard against silent loss.
            let backup_path = config_path.with_extension("json.bak");
            let _ = fs::write(&backup_path, existing_raw.as_bytes());
            return Err(std::io::Error::other(format!(
                "merge would drop top-level key '{}'; aborted, backup at {}",
                key,
                backup_path.display()
            )));
        }
    }
    Ok(())
}

/// Builds the permission rules for a single agent from the capability
/// profile, starting from a deny-by-default ceiling and allowing only the
/// declared capabilities. `extra` holds agent-specific rules that are always
/// kept (e.g. `*.env` denies for read-only agents, explorer tool allows).
fn agent_permissions(
    profile: &CapabilityProfile,
    extra: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut rules: Vec<serde_json::Value> =
        vec![json!({"action": "*", "resource": "*", "effect": "deny"})];

    let mut push = |action: &str, effect: &str| {
        rules.push(json!({"action": action, "resource": "*", "effect": effect}))
    };

    if profile.file_read {
        push("read", "allow");
        push("glob", "allow");
        push("grep", "allow");
    }
    if !profile.file_read {
        push("read", "deny");
        push("glob", "deny");
        push("grep", "deny");
    }
    if profile.file_write {
        push("edit", "allow");
        push("write", "allow");
    }
    if !profile.file_write {
        push("edit", "deny");
        push("write", "deny");
    }
    if profile.terminal {
        push("shell", "allow");
    }
    if !profile.terminal {
        push("shell", "deny");
    }
    if profile.mcp {
        push("mcp", "allow");
    }
    if !profile.mcp {
        push("mcp", "deny");
    }
    if profile.subagents {
        push("subagent", "allow");
    }
    if !profile.subagents {
        push("subagent", "deny");
    }
    if profile.web {
        push("webfetch", "allow");
        push("websearch", "allow");
    }
    if !profile.web {
        push("webfetch", "deny");
        push("websearch", "deny");
    }

    rules.extend(extra.iter().cloned());
    rules
}

/// Read-only agents keep their `*.env` denies regardless of the profile.
fn read_only_env_denies() -> Vec<serde_json::Value> {
    vec![
        json!({"action": "read", "resource": "*.env", "effect": "deny"}),
        json!({"action": "read", "resource": "*.env.*", "effect": "deny"}),
    ]
}

/// Role-scoped allows for the ASEP agent message tools (`asep_agent_*`).
/// `subagent` is never widened here: executor keeps an explicit subagent
/// deny, and judge/explorer keep theirs via `read_only_action_denies`.
fn leader_agent_message_rules() -> Vec<serde_json::Value> {
    vec![
        json!({"action": "asep_agent_send", "resource": "*", "effect": "allow"}),
        json!({"action": "asep_agent_recv", "resource": "*", "effect": "allow"}),
        json!({"action": "asep_agent_read", "resource": "*", "effect": "allow"}),
        json!({"action": "asep_agent_ack", "resource": "*", "effect": "allow"}),
    ]
}

/// Executor may only `asep_agent_send` with `kind=TASK_REPORT` and
/// `asep_agent_recv`; `subagent` stays denied.
fn executor_agent_message_rules() -> Vec<serde_json::Value> {
    vec![
        json!({"action": "subagent", "resource": "*", "effect": "deny"}),
        json!({"action": "asep_agent_send", "resource": "kind=TASK_REPORT", "effect": "allow"}),
        json!({"action": "asep_agent_recv", "resource": "*", "effect": "allow"}),
    ]
}

/// Judge may only `asep_agent_send` with `kind=VERDICT` and `asep_agent_read`.
fn judge_agent_message_rules() -> Vec<serde_json::Value> {
    vec![
        json!({"action": "asep_agent_send", "resource": "kind=VERDICT", "effect": "allow"}),
        json!({"action": "asep_agent_read", "resource": "*", "effect": "allow"}),
    ]
}

/// Explorer may only `asep_agent_send` with `kind=MAP_REPORT`.
fn explorer_agent_message_rules() -> Vec<serde_json::Value> {
    vec![json!({"action": "asep_agent_send", "resource": "kind=MAP_REPORT", "effect": "allow"})]
}

/// Explorer keeps its read-only tool allows (`execute`, `asep_srcdump`,
/// `asep_srcidx`) regardless of the profile.
fn explorer_tool_allows() -> Vec<serde_json::Value> {
    vec![
        json!({"action": "execute", "resource": "*", "effect": "allow"}),
        json!({"action": "asep_srcdump", "resource": "*", "effect": "allow"}),
        json!({"action": "asep_srcidx", "resource": "*", "effect": "allow"}),
    ]
}

/// Read-only agents (judge, explorer) must stay read-only regardless of the
/// capability profile. The profile is a ceiling, not a floor, for them, so
/// these deny rules are appended AFTER the capability allows (last match
/// wins), overriding any capability-derived allow for write/shell/web/mcp.
fn read_only_action_denies() -> Vec<serde_json::Value> {
    vec![
        json!({"action": "edit", "resource": "*", "effect": "deny"}),
        json!({"action": "write", "resource": "*", "effect": "deny"}),
        json!({"action": "shell", "resource": "*", "effect": "deny"}),
        json!({"action": "subagent", "resource": "*", "effect": "deny"}),
        json!({"action": "webfetch", "resource": "*", "effect": "deny"}),
        json!({"action": "websearch", "resource": "*", "effect": "deny"}),
        json!({"action": "mcp", "resource": "*", "effect": "deny"}),
    ]
}

/// Dedupe key for a permission rule: identical action+resource+effect means
/// the same rule, regardless of position (last-match-wins semantics).
fn permission_rule_key(rule: &serde_json::Value) -> (Option<&str>, Option<&str>, Option<&str>) {
    (
        rule.get("action").and_then(serde_json::Value::as_str),
        rule.get("resource").and_then(serde_json::Value::as_str),
        rule.get("effect").and_then(serde_json::Value::as_str),
    )
}

/// Merges default permission rules into an existing agent preset and
/// refreshes protocol content. `system`, `description`, `mode`, and `steps`
/// are overwritten from the canonical default preset because they are
/// Marker identifying a preset whose protocol fields ASEP may refresh.
const MANAGED_MARKER: &str = "asep-managed-v2";

/// protocol content, not user customization. Existing permission rules are
/// kept and default rules missing from the existing set are appended
/// (last-match-wins), deduped by action+resource+effect. All other fields on
/// the agent are preserved.
fn merge_agent_permissions(
    existing: &serde_json::Value,
    default: &serde_json::Value,
) -> serde_json::Value {
    let mut merged = existing.clone();
    let managed = existing
        .get("asep_managed")
        .and_then(|v| v.as_str())
        .map(|v| v == MANAGED_MARKER)
        .unwrap_or(false);

    if managed {
        // Only ASEP-managed presets are refreshed from canonical content.
        for field in ["system", "description", "mode", "steps", "asep_managed"] {
            if let Some(value) = default.get(field) {
                merged[field] = value.clone();
            }
        }
    } else if existing.get("system").and_then(|v| v.as_str())
        != default.get("system").and_then(|v| v.as_str())
    {
        eprintln!(
            "asep: preserved user-authored agent prompt; managed defaults were not applied"
        );
    }

    let default_perms = match default.get("permissions").and_then(|v| v.as_array()) {
        Some(perms) => perms,
        None => return merged,
    };

    let merged_perms = match existing.get("permissions").and_then(|v| v.as_array()) {
        // Managed presets derive permissions from the current capability
        // profile, so a restrictive upgrade cannot retain previously granted
        // permissions.
        Some(_) if managed => default_perms.clone(),
        Some(existing_perms) => existing_perms.clone(),
        None if managed => default_perms.clone(),
        None => Vec::new(),
    };

    merged["permissions"] = serde_json::Value::Array(merged_perms);
    merged
}

/// Installs default Leader-Judge-Executor agent presets into `opencode.json`
/// without clobbering pre-existing top-level keys or pre-existing agents.
/// Permission rules are derived from `profile`: each capability ceiling
/// allows or denies its corresponding actions, starting from a deny-all
/// rule, while preserving agent-specific `*.env` denies and tool allows.
/// For an agent preset that already exists, missing default permission
/// rules are merged in (appended after existing ones) and `system`,
/// `description`, `mode`, and `steps` are refreshed from the canonical
/// default preset (protocol content); all other fields on the agent are
/// preserved.
// @impl:FR-LEADER-01
pub fn scaffold_opencode_agents(
    config_path: &Path,
    profile: &CapabilityProfile,
) -> Result<(), std::io::Error> {
    // Read existing content (if any) without losing it.
    let existing_raw = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(_) if !config_path.exists() => {
            ensure_parent_dir(config_path)?;
            String::new()
        }
        Err(e) => return Err(e),
    };

    // Parse existing JSON. If the file is non-empty but unparseable, back up
    // the raw content and return an error instead of overwriting.
    let mut config: serde_json::Value = serde_json::json!({});
    if !existing_raw.trim().is_empty() {
        match serde_json::from_str(&existing_raw) {
            Ok(v) => config = v,
            Err(e) => {
                let backup_path = config_path.with_extension("json.bak");
                let _ = fs::write(&backup_path, existing_raw.as_bytes());
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "existing opencode.json is not valid JSON ({}); backup written to {}",
                        e,
                        backup_path.display()
                    ),
                ));
            }
        }
    }

    if !config.is_object() {
        config = serde_json::json!({});
    }

    // Ensure the "agents" object exists, then insert only missing defaults.
    if !config.get("agents").is_some_and(|v| v.is_object()) {
        config["agents"] = serde_json::json!({});
    }

    let default_agents = serde_json::json!({
        "leader": {
            "asep_managed": "asep-managed-v2",
            "description": "Orchestrator for the bounded executor-judge workflow.",
            "mode": "primary",
            "steps": 60,
            "system": r#"You are the primary agent. Execute the user objective directly with native tools: read, search, edit, compile and test as the task requires. ASEP capabilities are optional and activate only when `protocol/CORE.md` says they apply or the user asks explicitly. A general question, one file read, a typo or a local bug needs no ASEP tool, audit, PRD/TDD or delegation; a local bug uses a targeted regression test.

Activation is conditional, never a fixed sequence. Load a domain skill only when its scope matches the task. Ask the user only for material unresolved architecture decisions, breaking changes, irreversible operations or out-of-scope work. Do not create speculative documents, engines or dependencies.

Delegate selectively: one primary agent by default; use a subagent only when independent review is required or a bounded task materially benefits. Preserve scope and exact verification evidence. Agent judgement never grants ACCEPTED; only the trusted local runner does, and local acceptance does not authorize a merge. Never fabricate verification output or hide missing evidence."#,
            "permissions": agent_permissions(profile, &leader_agent_message_rules())
        },
        "executor": {
            "asep_managed": "asep-managed-v2",
            "description": "Fast coding executor in a bounded executor-judge pipeline.",
            "mode": "subagent",
            "steps": 30,
            "system": r#"You are the coding executor in a bounded executor-judge pipeline. The leader decomposes tasks and verifies your output; you do not make architectural decisions.

Working limits:
1. Hard scope rule: a task must name at least one file path and at least one function or symbol. If either is missing, edit nothing and reply only with `BLOCKED: task needs smaller scope` plus the missing item. Never infer scope from vague wording such as "improve", "more robust", or "handle better". Prefer one file or one function per task.
2. Before editing, read the relevant files and grep for the symbols and APIs you will use. Do not rely on memory and do not invent APIs.
3. Keep the change minimal: edit only what the task requires. Do not refactor outside scope and do not touch unrelated files.
4. Attempt a failing action at most twice. If it still fails, stop and report the exact error; never repeat the same failing action.
5. If you are uncertain or the evidence is insufficient, report `UNCERTAIN: <reason>` and stop. Do not guess.
6. After editing, run the verification command the leader requested (test, lint, or typecheck). If none was provided, report that verification was not run.
7. Report concisely: files changed, diff or patch, verification command and short output, and any unresolved issue. Do not write long prose.
8. End the report with exactly one status line: `DONE` when the task scope is complete and verification ran and passed, `UNCERTAIN: <reason>`, or `BLOCKED: <reason>`. Never omit the status line.
9. Handoff: every `TASK_REPORT` must carry the snapshot or diff hash, the command run, the exit code, and the output location.

Reply in Indonesian prose; code, comments, and identifiers stay in English."#,
            "permissions": agent_permissions(profile, &executor_agent_message_rules())
        },
        "judge": {
            "asep_managed": "asep-managed-v2",
            "description": "Read-only reviewer verifying executor output against task, diff, and test evidence.",
            "mode": "subagent",
            "steps": 20,
            "system": r#"You are a read-only judge in a bounded executor-judge pipeline. Evaluate the executor's work; do not redo it.

Required input: the original task, the diff, and verification output (test, lint, or build), plus explicit evidence referencing the tested snapshot and worktree. If any is missing, return it in `missing_evidence`; do not invent evidence.

Rules:
1. Do not edit, write, or run state-changing commands. You may only read files, glob, and grep.
2. Verify every finding against a concrete path and line; include path:line evidence.
3. Focus on behavioral correctness, regressions, scope creep, error handling, edge cases, and whether the tests actually prove the task.
4. Ignore cosmetic style preferences unless asked.
5. Use at most 20 steps. If the evidence is insufficient, stop and report the missing evidence.
6. Scope check: compare the diff against the named scope. Any changed line outside the named file or function is a blocking issue. A DONE status for a task that named no file or function is a blocking issue.
7. Evidence rule: judge only explicit snapshot/worktree evidence — every finding must cite path:line evidence inside the tested snapshot or worktree; unsupported claims are not evidence.
8. Trajectory review: when TASK_REPORT includes `tool_call_count` or `duplicate_calls`, flag duplicate or unnecessary tool calls as a blocking issue with call-sequence evidence.
9. Output valid JSON without markdown:
{"verdict":"PASS|FAIL","blocking":[{"file":"...","line":0,"issue":"...","evidence":"..."}],"non_blocking":[],"missing_evidence":[],"next_action":"..."}

Return PASS only when there are no blocking issues and the verification evidence is sufficient. If in doubt, return FAIL with a testable reason."#,
            "permissions": agent_permissions(
                profile,
                &[
                    read_only_env_denies(),
                    read_only_action_denies(),
                    judge_agent_message_rules(),
                ]
                .concat()
            )
        },
        "explorer": {
            "asep_managed": "asep-managed-v2",
            "description": "Fast explorer subagent for codebase exploration.",
            "mode": "subagent",
            "steps": 30,
            "system": r#"You are the explorer subagent, read-only. Use native `read`/`grep` on the specific files the question needs; batch independent reads. Do not require `srcdump` or any ASEP tool: structural extraction is optional and only chosen when native search is insufficient. Never guess tool names. Report findings with file paths and line references, list what you did not cover, and state any limits. Reply in Indonesian prose; code, identifiers and paths stay in English."#,
            "permissions": agent_permissions(
                profile,
                &[
                    read_only_env_denies(),
                    explorer_tool_allows(),
                    read_only_action_denies(),
                    explorer_agent_message_rules(),
                ]
                .concat()
            )
        },
        "code-worker": {
            "asep_managed": "asep-managed-v2",
            "description": "Code-focused worker agent for implementation tasks.",
            "mode": "subagent",
            "steps": 20,
            "system": r#"You are the code-worker subagent. Capability domain: code. Tool filter: read, glob, grep, edit, write, shell."#,
            "permissions": agent_permissions(profile, &[])
        },
        "git-worker": {
            "asep_managed": "asep-managed-v2",
            "description": "Git-focused worker agent for version-control tasks.",
            "mode": "subagent",
            "steps": 20,
            "system": r#"You are the git-worker subagent. Capability domain: git. Tool filter: shell, read, glob, grep."#,
            "permissions": agent_permissions(profile, &[])
        },
        "media-worker": {
            "asep_managed": "asep-managed-v2",
            "description": "Media-focused worker agent for image and audio assets.",
            "mode": "subagent",
            "steps": 20,
            "system": r#"You are the media-worker subagent. Capability domain: media. Tool filter: read, glob, grep, write."#,
            "permissions": agent_permissions(profile, &[])
        },
        "verifier": {
            "asep_managed": "asep-managed-v2",
            "description": "Verification worker agent that checks output against acceptance criteria.",
            "mode": "subagent",
            "steps": 20,
            "system": r#"You are the verifier subagent. Capability domain: verification. Tool filter: read, glob, grep, shell."#,
            "permissions": agent_permissions(profile, &[])
        }
    });

    let agents_obj = config
        .get_mut("agents")
        .expect("agents object was just ensured")
        .as_object_mut()
        .expect("agents value is an object");
    for (key, default_value) in default_agents
        .as_object()
        .expect("default_agents is an object")
    {
        match agents_obj.get(key) {
            // Missing agent: insert the full default preset.
            None => {
                agents_obj.insert(key.clone(), default_value.clone());
            }
            // Existing agent: preserve system/steps/description/mode and
            // existing rules; merge in only the missing default permission
            // rules, appended after existing ones (last-match-wins).
            Some(existing) => {
                agents_obj.insert(
                    key.clone(),
                    merge_agent_permissions(existing, default_value),
                );
            }
        }
    }

    // Back up the original content before overwriting (best effort).
    if !existing_raw.is_empty() {
        let backup_path = config_path.with_extension("json.bak");
        let _ = fs::write(&backup_path, existing_raw.as_bytes());
    }

    let formatted = serde_json::to_string_pretty(&config).map_err(std::io::Error::other)?;
    fs::write(config_path, formatted)?;
    Ok(())
}
