// @impl:FR-BUDGET-01 // @impl:FR-BUDGET-02 // @impl:FR-BUDGET-03

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ResourceBudget {
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub max_tool_calls: Option<u64>,
    #[serde(default)]
    pub max_cost_usd: Option<f64>,
    #[serde(default)]
    pub max_latency_ms: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct BudgetTracker {
    pub budget: ResourceBudget,
    pub tokens: u64,
    pub tool_calls: u64,
    pub cost_usd: f64,
    pub latency_ms: u64,
}

impl BudgetTracker {
    pub fn record_tokens(&mut self, n: u64) {
        self.tokens = self.tokens.saturating_add(n);
    }

    pub fn record_tool_call(&mut self) {
        self.tool_calls = self.tool_calls.saturating_add(1);
    }

    pub fn record_cost(&mut self, usd: f64) {
        self.cost_usd += usd;
    }

    pub fn record_latency(&mut self, ms: u64) {
        self.latency_ms = self.latency_ms.saturating_add(ms);
    }

    pub fn exceeded(&self) -> Option<&'static str> {
        if self
            .budget
            .max_tokens
            .map(|m| self.tokens > m)
            .unwrap_or(false)
        {
            return Some("tokens");
        }
        if self
            .budget
            .max_tool_calls
            .map(|m| self.tool_calls > m)
            .unwrap_or(false)
        {
            return Some("tool_calls");
        }
        if self
            .budget
            .max_cost_usd
            .map(|m| self.cost_usd > m)
            .unwrap_or(false)
        {
            return Some("cost");
        }
        if self
            .budget
            .max_latency_ms
            .map(|m| self.latency_ms > m)
            .unwrap_or(false)
        {
            return Some("latency");
        }
        None
    }

    pub fn blocked_message(&self) -> Option<String> {
        self.exceeded()
            .map(|dim| format!("BLOCKED: budget exceeded ({dim})"))
    }
}

#[cfg(test)]
mod tests {
    // @test:FR-BUDGET-01
    // @test:FR-BUDGET-02
    // @test:FR-BUDGET-03
    use super::*;

    #[test]
    fn budget_round_trip() {
        let b = ResourceBudget {
            max_tokens: Some(100),
            max_tool_calls: None,
            max_cost_usd: Some(1.5),
            max_latency_ms: None,
        };
        let json = serde_json::to_string(&b).unwrap();
        let back: ResourceBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn default_unlimited() {
        let b = ResourceBudget::default();
        assert!(b.max_tokens.is_none());
        assert!(b.max_tool_calls.is_none());
        assert!(b.max_cost_usd.is_none());
        assert!(b.max_latency_ms.is_none());
    }

    #[test]
    fn exceeded_first_dimension() {
        let mut t = BudgetTracker {
            budget: ResourceBudget {
                max_tokens: Some(10),
                max_tool_calls: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };
        t.record_tokens(11);
        assert_eq!(t.exceeded(), Some("tokens"));
        assert_eq!(
            t.blocked_message().as_deref(),
            Some("BLOCKED: budget exceeded (tokens)")
        );
    }

    #[test]
    fn unlimited_never_exceeds() {
        let mut t = BudgetTracker::default();
        t.record_tokens(1_000_000);
        assert!(t.exceeded().is_none());
        assert!(t.blocked_message().is_none());
    }
}
