//! Manifest generation and configuration merging for Antigravity and OpenCode.

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

/// Merges `asep` MCP configuration into an existing or new `opencode.json`.
///
/// Safety: reads and parses the existing file first; if parsing fails, the
/// original content is preserved in a `.bak` file and an error is returned
/// instead of clobbering the user's configuration. A backup is also written
/// before any successful write.
pub fn update_opencode_json(config_path: &Path, command: &str) -> Result<(), std::io::Error> {
    // Read existing content (if any) without losing it.
    let existing_raw = if config_path.exists() {
        fs::read_to_string(config_path)?
    } else {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        String::new()
    };

    // Parse existing JSON. If the file is non-empty but unparseable, back up
    // the raw content and return an error instead of overwriting.
    let mut config: serde_json::Value = if existing_raw.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(&existing_raw) {
            Ok(v) => v,
            Err(e) => {
                // Preserve the user's file verbatim in a backup.
                let backup_path = config_path.with_extension("json.bak");
                let _ = fs::write(&backup_path, &existing_raw);
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
    };

    if !config.is_object() {
        config = serde_json::json!({});
    }

    // Preserve any pre-existing mcp.servers (and other keys) by only touching
    // the "asep" entry under mcp.
    if config.get("mcp").is_none() || !config["mcp"].is_object() {
        config["mcp"] = serde_json::json!({});
    }

    config["mcp"]["asep"] = serde_json::json!({
        "type": "local",
        "command": command,
        "args": ["mcp"],
        "enabled": true
    });

    // Back up the original content before overwriting (best effort).
    if !existing_raw.is_empty() {
        let backup_path = config_path.with_extension("json.bak");
        let _ = fs::write(&backup_path, &existing_raw);
    }

    let formatted = serde_json::to_string_pretty(&config).map_err(std::io::Error::other)?;

    // Sanity check: the merged result must still contain any previously
    // present top-level keys (e.g. "provider", "model", "plugins").
    if !existing_raw.trim().is_empty() {
        if let Ok(prev) = serde_json::from_str::<serde_json::Value>(&existing_raw) {
            if let Some(prev_obj) = prev.as_object() {
                if let Some(new_obj) = config.as_object() {
                    for key in prev_obj.keys() {
                        if !new_obj.contains_key(key) {
                            // Should not happen, but guard against silent loss.
                            let backup_path = config_path.with_extension("json.bak");
                            let _ = fs::write(&backup_path, &existing_raw);
                            return Err(std::io::Error::other(format!(
                                "merge would drop top-level key '{}'; aborted, backup at {}",
                                key,
                                backup_path.display()
                            )));
                        }
                    }
                }
            }
        }
    }

    fs::write(config_path, formatted)?;
    Ok(())
}

/// Builds the permission rules for a single agent from the capability
/// profile, starting from a deny-by-default ceiling and allowing only the
/// declared capabilities. `extra` holds agent-specific rules that are always
/// kept (e.g. `*.env` denies for read-only agents, explorer tool allows).
fn agent_permissions(profile: &CapabilityProfile, extra: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut rules: Vec<serde_json::Value> =
        vec![json!({"action": "*", "resource": "*", "effect": "deny"})];

    let mut push = |action: &str, effect: &str| {
        rules.push(json!({"action": action, "resource": "*", "effect": effect}))
    };

    if profile.file_read {
        push("read", "allow");
        push("glob", "allow");
        push("grep", "allow");
    } else {
        push("read", "deny");
        push("glob", "deny");
        push("grep", "deny");
    }
    if profile.file_write {
        push("edit", "allow");
        push("write", "allow");
    } else {
        push("edit", "deny");
        push("write", "deny");
    }
    if profile.terminal {
        push("shell", "allow");
    } else {
        push("shell", "deny");
    }
    if profile.mcp {
        push("mcp", "allow");
    } else {
        push("mcp", "deny");
    }
    if profile.subagents {
        push("subagent", "allow");
    } else {
        push("subagent", "deny");
    }
    if profile.web {
        push("webfetch", "allow");
        push("websearch", "allow");
    } else {
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
fn lead_agent_message_rules() -> Vec<serde_json::Value> {
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
/// protocol content, not user customization. Existing permission rules are
/// kept and default rules missing from the existing set are appended
/// (last-match-wins), deduped by action+resource+effect. All other fields on
/// the agent are preserved.
fn merge_agent_permissions(
    existing: &serde_json::Value,
    default: &serde_json::Value,
) -> serde_json::Value {
    let mut merged = existing.clone();

    // Protocol content is canonical: always refresh from the default preset.
    for field in ["system", "description", "mode", "steps"] {
        if let Some(value) = default.get(field) {
            merged[field] = value.clone();
        }
    }

    let default_perms = match default.get("permissions").and_then(|v| v.as_array()) {
        Some(perms) => perms,
        None => return merged,
    };

    let merged_perms = match existing.get("permissions").and_then(|v| v.as_array()) {
        Some(existing_perms) => {
            let mut rules = existing_perms.clone();
            let mut seen: HashSet<(Option<&str>, Option<&str>, Option<&str>)> =
                existing_perms.iter().map(permission_rule_key).collect();
            for rule in default_perms {
                let key = permission_rule_key(rule);
                if seen.insert(key) {
                    rules.push(rule.clone());
                }
            }
            rules
        }
        None => default_perms.clone(),
    };

    merged["permissions"] = serde_json::Value::Array(merged_perms);
    merged
}

/// Installs default Lead-Judge-Executor agent presets into `opencode.json`
/// without clobbering pre-existing top-level keys or pre-existing agents.
/// Permission rules are derived from `profile`: each capability ceiling
/// allows or denies its corresponding actions, starting from a deny-all
/// rule, while preserving agent-specific `*.env` denies and tool allows.
/// For an agent preset that already exists, missing default permission
/// rules are merged in (appended after existing ones) and `system`,
/// `description`, `mode`, and `steps` are refreshed from the canonical
/// default preset (protocol content); all other fields on the agent are
/// preserved.
pub fn scaffold_opencode_agents(
    config_path: &Path,
    profile: &CapabilityProfile,
) -> Result<(), std::io::Error> {
    // Read existing content (if any) without losing it.
    let existing_raw = if config_path.exists() {
        fs::read_to_string(config_path)?
    } else {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        String::new()
    };

    // Parse existing JSON. If the file is non-empty but unparseable, back up
    // the raw content and return an error instead of overwriting.
    let mut config: serde_json::Value = if existing_raw.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(&existing_raw) {
            Ok(v) => v,
            Err(e) => {
                let backup_path = config_path.with_extension("json.bak");
                let _ = fs::write(&backup_path, &existing_raw);
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
    };

    if !config.is_object() {
        config = serde_json::json!({});
    }

    // Ensure the "agents" object exists, then insert only missing defaults.
    if !config.get("agents").is_some_and(|v| v.is_object()) {
        config["agents"] = serde_json::json!({});
    }

    let default_agents = serde_json::json!({
        "lead": {
            "description": "Orchestrator for the bounded executor-judge workflow.",
            "mode": "primary",
            "steps": 60,
            "system": r#"You are the lead orchestrator in a bounded executor-judge workflow. You own the loop; the executor implements and the judge reviews. Do not delegate the whole task in one call.

Before any implementation:
- Requirement phase: if the request leaves something material unsettled, ask the user ONE decisive question. Write the agreed behavior into `docs/PRD.md` as `@req:*:MUST` requirements tied to observable behavior. Do not invent requirements to make a gate green.
- Plan phase: write the design and the ordered implementation plan into `docs/TDD.md` (`@design:` entries, and for each task a file path, symbol, acceptance criteria, and verification command). Stop and get explicit user approval before dispatching any executor.

Never create `.superpowers/` or `docs/superpowers/`, and never add a new skill folder for this pattern. `docs/PRD.md`, `docs/TDD.md`, and `docs/TRACEABILITY.md` are the only specification artifacts. Executors never edit PRD/TDD.

Verification scope: run `asep affected <changed-paths>` to derive the exact tests and check targets from the traceability graph. Use its output as the verification command. Do not run the full suite by default; full suite is reserved for merges to `main` or when the user explicitly asks.

For every coding task:
1. After PRD/TDD approval, decompose the request into small units: one file or one function per unit. If the request is ambiguous, ask the user before delegating.
   If the request touches a project or code area you have not mapped yet, delegate the mapping to the `explorer` subagent first: it reads the project with `tools.asep.srcdump`, then returns a module list with file paths. Use that list to split the units.
2. Before delegating, check the unit: it must name a file path, a function or symbol, acceptance criteria, and a verification command. If any is missing, split the task or ask the user; do not delegate. Then delegate one unit to the `executor` subagent with that scope, files in scope, acceptance criteria, and the verification command.
3. After the executor returns, inspect the diff yourself. Run the relevant test, lint, typecheck, or build command. Capture the exact command and output.
4. Send the original task, the diff, and the verification output to the `judge` subagent. The judge is read-only and returns JSON with `verdict`, `blocking`, `non_blocking`, `missing_evidence`, and `next_action`.
5. If the verdict is FAIL, send only the blocking issues and the failing output back to the executor. Re-run verification, then invoke the judge again. Repeat for at most two fix rounds.
6. If the judge still returns FAIL after two rounds, stop and report the unresolved blocking issues to the user. Do not start another open-ended loop.
7. If the executor returns `UNCERTAIN` or `BLOCKED`, split the task further, gather the missing evidence, or ask the user. Do not force it to guess.
8. Keep the final report short: what changed, verification evidence, judge verdict, and remaining risk.

Rules:
- Tests and executable checks are the primary source of truth. The judge is a semantic second opinion, not a replacement for tests.
- Never claim completion without fresh verification output.
- Do not ask the judge to edit, run shell, or review the entire repository. Give it bounded evidence.
- Do not let the executor make architectural decisions. Decide architecture yourself, or ask the user."#,
            "permissions": agent_permissions(profile, &lead_agent_message_rules())
        },
        "executor": {
            "description": "Fast coding executor in a bounded executor-judge pipeline.",
            "mode": "subagent",
            "steps": 30,
            "system": r#"You are the coding executor in a bounded executor-judge pipeline. The lead decomposes tasks and verifies your output; you do not make architectural decisions.

Working limits:
1. Hard scope rule: a task must name at least one file path and at least one function or symbol. If either is missing, edit nothing and reply only with `BLOCKED: task needs smaller scope` plus the missing item. Never infer scope from vague wording such as "improve", "more robust", or "handle better". Prefer one file or one function per task.
2. Before editing, read the relevant files and grep for the symbols and APIs you will use. Do not rely on memory and do not invent APIs.
3. Keep the change minimal: edit only what the task requires. Do not refactor outside scope and do not touch unrelated files.
4. Attempt a failing action at most twice. If it still fails, stop and report the exact error; never repeat the same failing action.
5. If you are uncertain or the evidence is insufficient, report `UNCERTAIN: <reason>` and stop. Do not guess.
6. After editing, run the verification command the lead requested (test, lint, or typecheck). If none was provided, report that verification was not run.
7. Report concisely: files changed, diff or patch, verification command and short output, and any unresolved issue. Do not write long prose.
8. End the report with exactly one status line: `DONE` when the task scope is complete and verification ran and passed, `UNCERTAIN: <reason>`, or `BLOCKED: <reason>`. Never omit the status line.

Reply in Indonesian prose; code, comments, and identifiers stay in English."#,
            "permissions": agent_permissions(profile, &executor_agent_message_rules())
        },
        "judge": {
            "description": "Read-only reviewer verifying executor output against task, diff, and test evidence.",
            "mode": "subagent",
            "steps": 20,
            "system": r#"You are a read-only judge in a bounded executor-judge pipeline. Evaluate the executor's work; do not redo it.

Required input: the original task, the diff, and verification output (test, lint, or build). If any is missing, return it in `missing_evidence`; do not invent evidence.

Rules:
1. Do not edit, write, or run state-changing commands. You may only read files, glob, and grep.
2. Verify every finding against a concrete path and line; include path:line evidence.
3. Focus on behavioral correctness, regressions, scope creep, error handling, edge cases, and whether the tests actually prove the task.
4. Ignore cosmetic style preferences unless asked.
5. Use at most 20 steps. If the evidence is insufficient, stop and report the missing evidence.
6. Scope check: compare the diff against the named scope. Any changed line outside the named file or function is a blocking issue. A DONE status for a task that named no file or function is a blocking issue.
7. Output valid JSON without markdown:
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
            "description": "Fast explorer subagent for codebase exploration.",
            "mode": "subagent",
            "steps": 30,
            "system": r#"You are the explorer subagent. You are read-only: never edit files and never run shell commands.

Working limits:
1. Your first step is one call to `tools.asep.srcdump` through `execute`, with `dir` set to the project root and `mode` set to `signatures`. This gives the whole project in one call. Do not read files one by one before this call.
2. Only call tool names listed by `search` (namespace `asep`). Never guess tool names.
3. After the dump, read only the specific files the question needs. Batch independent reads into one step.
4. You have a limited number of steps. Plan the reads so that the final step is your report.
5. Coverage: if the question is about a crate or project module list, read the module root file (for example `lib.rs`) and make sure every declared module appears in your report. List any module you did not cover.
6. Evidence: describe a module or file only after you have read its file content. A description based on a signature dump alone must be marked `(from signatures only)`.
7. Report findings with file paths and line references, list what you did not cover, and state any limit you hit.

Reply in Indonesian prose; code, identifiers, and paths stay in English."#,
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
        }
    });

    let agents_obj = config
        .get_mut("agents")
        .expect("agents object was just ensured")
        .as_object_mut()
        .expect("agents value is an object");
    for (key, default_value) in default_agents.as_object().expect("default_agents is an object") {
        match agents_obj.get(key) {
            // Missing agent: insert the full default preset.
            None => {
                agents_obj.insert(key.clone(), default_value.clone());
            }
            // Existing agent: preserve system/steps/description/mode and
            // existing rules; merge in only the missing default permission
            // rules, appended after existing ones (last-match-wins).
            Some(existing) => {
                agents_obj.insert(key.clone(), merge_agent_permissions(existing, default_value));
            }
        }
    }

    // Back up the original content before overwriting (best effort).
    if !existing_raw.is_empty() {
        let backup_path = config_path.with_extension("json.bak");
        let _ = fs::write(&backup_path, &existing_raw);
    }

    let formatted = serde_json::to_string_pretty(&config).map_err(std::io::Error::other)?;
    fs::write(config_path, formatted)?;
    Ok(())
}
