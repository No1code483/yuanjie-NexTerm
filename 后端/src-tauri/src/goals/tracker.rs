pub enum BudgetStatus {
    Safe,
    Warning,
    Critical,
    Exhausted,
}

pub struct TokenBudget {
    total: i64,
    used: i64,
    warn_threshold_pct: f64,
    critical_threshold_pct: f64,
}

impl TokenBudget {
    pub fn new(total: i64) -> Self {
        Self {
            total,
            used: 0,
            warn_threshold_pct: 70.0,
            critical_threshold_pct: 90.0,
        }
    }

    pub fn with_thresholds(total: i64, warn_pct: f64, critical_pct: f64) -> Self {
        Self {
            total,
            used: 0,
            warn_threshold_pct: warn_pct,
            critical_threshold_pct: critical_pct,
        }
    }

    pub fn total(&self) -> i64 {
        self.total
    }

    pub fn used(&self) -> i64 {
        self.used
    }

    pub fn remaining(&self) -> i64 {
        (self.total - self.used).max(0)
    }

    pub fn pct_used(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.used as f64 / self.total as f64) * 100.0
    }

    pub fn consume(&mut self, tokens: i64) -> BudgetStatus {
        self.used += tokens;
        self.status()
    }

    pub fn status(&self) -> BudgetStatus {
        if self.total == 0 {
            return BudgetStatus::Safe;
        }
        let pct = self.pct_used();
        if pct >= 100.0 {
            BudgetStatus::Exhausted
        } else if pct >= self.critical_threshold_pct {
            BudgetStatus::Critical
        } else if pct >= self.warn_threshold_pct {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Safe
        }
    }

    pub fn set_total(&mut self, total: i64) {
        self.total = total;
    }

    pub fn reset_used(&mut self) {
        self.used = 0;
    }

    pub fn can_continue(&self) -> bool {
        !matches!(self.status(), BudgetStatus::Exhausted)
    }
}

pub struct BudgetTracker {
    budgets: std::collections::HashMap<i64, TokenBudget>,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self {
            budgets: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, goal_id: i64, budget: i64, tokens_used: i64) {
        let mut tb = TokenBudget::new(budget);
        tb.used = tokens_used;
        self.budgets.insert(goal_id, tb);
    }

    pub fn consume(&mut self, goal_id: i64, tokens: i64) -> Option<BudgetStatus> {
        self.budgets
            .get_mut(&goal_id)
            .map(|tb| tb.consume(tokens))
    }

    pub fn status(&self, goal_id: i64) -> Option<BudgetStatus> {
        self.budgets.get(&goal_id).map(|tb| tb.status())
    }

    pub fn can_continue(&self, goal_id: i64) -> bool {
        self.budgets
            .get(&goal_id)
            .map(|tb| tb.can_continue())
            .unwrap_or(true)
    }

    pub fn remaining(&self, goal_id: i64) -> Option<i64> {
        self.budgets.get(&goal_id).map(|tb| tb.remaining())
    }

    pub fn remove(&mut self, goal_id: i64) {
        self.budgets.remove(&goal_id);
    }

    pub fn update_budget(&mut self, goal_id: i64, new_budget: i64) {
        if let Some(tb) = self.budgets.get_mut(&goal_id) {
            tb.set_total(new_budget);
        }
    }
}