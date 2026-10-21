//! Small validated helpers shared by the CLI and the explorer contract.

/// A budget is valid when it is non-zero and fits the pack accumulator.
pub fn valid_budget(budget: u64) -> bool {
    budget > 0 && budget <= u64::from(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::valid_budget;

    #[test]
    fn rejects_zero_and_oversized_budgets() {
        assert!(!valid_budget(0));
        assert!(!valid_budget(u64::from(u32::MAX) + 1));
    }

    #[test]
    fn accepts_a_normal_budget() {
        assert!(valid_budget(120_000));
    }
}
