//! The pour: deterministic hard-budget selection.
//!
//! Given a set of graded candidates each with a token mass and a relevance
//! grade, choose a subset whose total token mass does not exceed a hard budget
//! while maximising captured relevance. This is a 0/1 knapsack.
//!
//! We solve it exactly with dynamic programming when the budget (scaled to a
//! coarse grid) is small enough, and fall back to a deterministic
//! value-density greedy for very large inputs. Both paths are fully
//! deterministic: ties are broken by `(‑score, tokens, rel_path)` so the same
//! inputs always yield the same pour.

/// One item presented to the solver.
#[derive(Debug, Clone)]
pub struct Item {
    /// Stable identity (repo-relative path).
    pub id: String,
    /// Token cost of including the item.
    pub tokens: u64,
    /// Relevance grade (value) in `0.0..=100.0`.
    pub score: f64,
}

/// Result of a solve.
#[derive(Debug, Clone)]
pub struct Solution {
    /// Selected item ids in fill order (highest priority first).
    pub selected: Vec<String>,
    /// Total tokens consumed.
    pub tokens_used: u64,
    /// Total captured relevance.
    pub value: f64,
    /// Which algorithm produced the result: `"dp"` or `"greedy"`.
    pub method: &'static str,
}

/// Maximum number of DP grid cells (items × buckets) before we fall back to
/// greedy. Keeps worst-case memory and time bounded.
const MAX_DP_CELLS: u64 = 4_000_000;

/// Number of token buckets the budget is quantised into for the DP. Finer
/// buckets = more precise, but more cells.
const DP_BUCKETS: u64 = 2_000;

/// Solve the hard-budget selection.
///
/// `budget` is the maximum total tokens. Items exceeding the budget on their
/// own are automatically infeasible and excluded.
pub fn solve(items: &[Item], budget: u64) -> Solution {
    if budget == 0 || items.is_empty() {
        return Solution {
            selected: Vec::new(),
            tokens_used: 0,
            value: 0.0,
            method: "empty",
        };
    }

    // Deterministic canonical ordering of the input.
    let mut ordered: Vec<&Item> = items.iter().collect();
    ordered.sort_by(|a, b| {
