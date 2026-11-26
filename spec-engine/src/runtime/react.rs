// @impl:FR-REACT-01 // @impl:FR-REACT-02 // @impl:FR-REACT-03
// @impl:FR-AUD-15

use serde::{Deserialize, Serialize};

use crate::config::settings::CapabilityProfile;
use crate::observability::TraceEvent;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub trace_id: String,
    #[serde(default)]
    pub task_id: String,
    pub seq: u64,
    pub tool: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub trace_id: String,
    #[serde(default)]
    pub task_id: String,
    pub seq: u64,
    pub ok: bool,
    #[serde(default)]
    pub output: serde_json::Value,
    #[serde(default)]
    pub error: Option<String>,
}

pub struct ActionContract;

impl ActionContract {
    pub fn validate(action: &Action, profile: &CapabilityProfile) -> Result<(), String> {
        if action.task_id.is_empty() {
            return Err("BLOCKED: missing task_id".to_string());
        }
        if action.trace_id.is_empty() {
            return Err("BLOCKED: missing trace_id".to_string());
        }
        if action.tool.is_empty() {
            return Err("BLOCKED: missing tool".to_string());
        }
        if let Some(first) = profile
            .missing(
                &action
                    .required_capabilities
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
            .into_iter()
            .next()
        {
            return Err(format!("BLOCKED: requires {first}"));
        }
        Ok(())
    }
}

impl From<Observation> for TraceEvent {
    fn from(obs: Observation) -> Self {
        TraceEvent {
            schema_version: 1,
            trace_id: obs.trace_id,
            parent_id: None,
            task_id: obs.task_id,
            seq: obs.seq,
            ts_wall: crate::agent_protocol::now_rfc3339(),
            event: "observation".to_string(),
            duration_ms: None,
            cost: None,
            missing: Vec::new(),
            detail: Some(serde_json::json!({
                "ok": obs.ok,
                "output": obs.output,
                "error": obs.error,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    // @test:FR-REACT-01
    // @test:FR-REACT-02
    // @test:FR-REACT-03
    // @test:FR-AUD-15
    use super::*;
    use crate::config::settings::CapabilityProfile;

    #[test]
    fn action_observation_round_trip() {
        let action = Action {
            trace_id: "t1".to_string(),
            task_id: "task-1".to_string(),
            seq: 2,
            tool: "read".to_string(),
            args: serde_json::json!({"path": "a.rs"}),
            required_capabilities: vec!["file_read".to_string()],
        };
        let json = serde_json::to_string(&action).unwrap();
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);

        let obs = Observation {
            trace_id: "t1".to_string(),
            task_id: "task-obs".to_string(),
            seq: 2,
            ok: true,
            output: serde_json::json!({"lines": 3}),
            error: None,
        };
        let json = serde_json::to_string(&obs).unwrap();
        let back: Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(back, obs);
    }

    #[test]
    fn validate_rejects_missing() {
        let profile = CapabilityProfile::default();
        let bad = Action {
            trace_id: String::new(),
            task_id: "task-bad".to_string(),
            seq: 1,
            tool: String::new(),
            args: serde_json::Value::Null,
            required_capabilities: vec![],
        };
        let err = ActionContract::validate(&bad, &profile).unwrap_err();
        assert!(err.contains("BLOCKED"));

        let missing_cap = Action {
            trace_id: "t".to_string(),
            task_id: "task-cap".to_string(),
            seq: 1,
            tool: "write".to_string(),
            args: serde_json::Value::Null,
            required_capabilities: vec!["file_write".to_string()],
        };
        let mut p = CapabilityProfile::default();
        p.file_write = false;
        let err = ActionContract::validate(&missing_cap, &p).unwrap_err();
        assert!(err.contains("BLOCKED: requires file_write"));
    }

    #[test]
    fn observation_converts_to_trace_event() {
        let obs = Observation {
            trace_id: "t1".to_string(),
            task_id: "task-obs-1".to_string(),
            seq: 7,
            ok: false,
            output: serde_json::Value::Null,
            error: Some("boom".to_string()),
        };
        let event: TraceEvent = obs.into();
        assert_eq!(event.event, "observation");
        assert_eq!(event.trace_id, "t1");
        assert_eq!(event.seq, 7);
        assert_eq!(event.task_id, "task-obs-1");
        assert!(event.detail.is_some());
    }

    #[test]
    fn observation_preserves_task_id() {
        let a = Observation {
            trace_id: "t1".to_string(),
            task_id: "task-a".to_string(),
            seq: 1,
            ok: true,
            output: serde_json::Value::Null,
            error: None,
        };
        let b = Observation {
            trace_id: "t1".to_string(),
            task_id: "task-b".to_string(),
            seq: 2,
            ok: true,
            output: serde_json::Value::Null,
            error: None,
        };
        let ea: TraceEvent = a.into();
        let eb: TraceEvent = b.into();
        assert_ne!(ea.task_id, eb.task_id);
        assert!(!ea.task_id.is_empty());
        assert!(!eb.task_id.is_empty());
    }

    #[test]
    fn validate_rejects_empty_task_id() {
        let profile = CapabilityProfile::default();
        let bad = Action {
            trace_id: "t".to_string(),
            task_id: String::new(),
            seq: 1,
            tool: "read".to_string(),
            args: serde_json::Value::Null,
            required_capabilities: vec![],
        };
        let err = ActionContract::validate(&bad, &profile).unwrap_err();
        assert_eq!(err, "BLOCKED: missing task_id");
    }

    #[test]
    fn old_json_without_task_id_defaults_empty() {
        let json =
            r#"{"trace_id":"t","seq":1,"tool":"read","args":null,"required_capabilities":[]}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action.task_id, String::new());
        let profile = CapabilityProfile::default();
        let err = ActionContract::validate(&action, &profile).unwrap_err();
        assert_eq!(err, "BLOCKED: missing task_id");
    }
}
