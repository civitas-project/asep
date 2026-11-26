// @impl:FR-BUDGET-01 // @impl:FR-BUDGET-02 // @impl:FR-BUDGET-03 // @impl:FR-AUD-22

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

    pub fn try_record_cost(&mut self, usd: f64) -> Result<(), String> {
        if !usd.is_finite() || usd < 0.0 {
            return Err(format!("invalid cost: {usd} (must be finite and non-negative)"));
        }
        self.cost_usd += usd;
        Ok(())
    }

    pub fn reserve_cost(&mut self, usd: f64) -> Result<(), String> {
        if !usd.is_finite() || usd < 0.0 {
            return Err(format!("invalid cost: {usd} (must be finite and non-negative)"));
        }
        let limit = match self.budget.max_cost_usd {
            Some(limit) => limit,
            None => return self.try_record_cost(usd),
        };
        if self.cost_usd + usd > limit {
            return Err(format!(
                "budget exhausted: cost {} + {usd} > {limit}",
                self.cost_usd
            ));
        }
        self.cost_usd += usd;
        Ok(())
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

    // @test:FR-AUD-22
    #[test]
    fn cost_rejects_negative_nan_inf() {
        let mut t = BudgetTracker::default();
        assert!(t.try_record_cost(-1.0).is_err());
        assert!(t.try_record_cost(f64::NAN).is_err());
        assert!(t.try_record_cost(f64::INFINITY).is_err());
        assert!(t.try_record_cost(f64::NEG_INFINITY).is_err());
        assert_eq!(t.cost_usd, 0.0);
        assert!(t.reserve_cost(-0.5).is_err());
        assert!(t.reserve_cost(f64::NAN).is_err());
        assert_eq!(t.cost_usd, 0.0);
    }

    #[test]
    fn cost_accumulates_normally() {
        let mut t = BudgetTracker::default();
        t.try_record_cost(1.0).unwrap();
        t.try_record_cost(2.5).unwrap();
        assert_eq!(t.cost_usd, 3.5);
    }

    #[test]
    fn reserve_blocks_exhausted_budget() {
        let mut t = BudgetTracker {
            budget: ResourceBudget {
                max_cost_usd: Some(5.0),
                ..Default::default()
            },
            ..Default::default()
        };
        t.reserve_cost(5.0).unwrap();
        assert_eq!(t.exceeded(), None);
        assert_eq!(t.cost_usd, 5.0);
        assert!(t.reserve_cost(0.1).is_err());
        assert_eq!(t.cost_usd, 5.0);
        t.try_record_cost(1.0).unwrap();
        assert_eq!(t.exceeded(), Some("cost"));
    }

    #[test]
    fn reserve_records_within_budget() {
        let mut t = BudgetTracker {
            budget: ResourceBudget {
                max_cost_usd: Some(5.0),
                ..Default::default()
            },
            ..Default::default()
        };
        t.reserve_cost(2.0).unwrap();
        assert_eq!(t.exceeded(), None);
        t.reserve_cost(2.0).unwrap();
        assert_eq!(t.exceeded(), None);
        assert_eq!(t.cost_usd, 4.0);
    }
}
