// A small budget solver fixture used by contextcrucible's integration tests.
// It is intentionally relevant to the query "budget solver knapsack".

use std::collections::HashMap;

/// Solve a fractional budget allocation across weighted tasks.
pub fn allocate_budget(total: u64, weights: &HashMap<String, f64>) -> HashMap<String, u64> {
    let sum: f64 = weights.values().copied().sum();
    let mut out = HashMap::new();
    if sum <= 0.0 {
        return out;
