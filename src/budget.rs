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
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.tokens.cmp(&b.tokens))
            .then(a.id.cmp(&b.id))
    });

    // Drop items that individually exceed the budget.
    let feasible: Vec<&Item> = ordered.into_iter().filter(|i| i.tokens <= budget).collect();
    if feasible.is_empty() {
        return Solution {
            selected: Vec::new(),
            tokens_used: 0,
            value: 0.0,
            method: "empty",
        };
    }

    let bucket = (budget / DP_BUCKETS).max(1);
    let cap_buckets = budget / bucket;
    let cells = (feasible.len() as u64).saturating_mul(cap_buckets + 1);

    if cells <= MAX_DP_CELLS {
        solve_dp(&feasible, budget, bucket)
    } else {
        solve_greedy(&feasible, budget)
    }
}

/// Exact 0/1 knapsack over a quantised capacity grid.
fn solve_dp(items: &[&Item], budget: u64, bucket: u64) -> Solution {
    let n = items.len();
    let cap = (budget / bucket) as usize; // number of buckets
                                          // Scale each item's cost up to bucket units (ceil so we never over-fill).
    let costs: Vec<usize> = items
        .iter()
        .map(|i| (i.tokens.div_ceil(bucket)) as usize)
        .collect();
    // Value scaled to integers for stable comparison (score has 4 dp precision).
    let values: Vec<i64> = items.iter().map(|i| (i.score * 10_000.0) as i64).collect();

    // dp[w] = best value achievable with capacity w buckets.
    let mut dp = vec![0i64; cap + 1];
    // keep[i][w] = whether item i is taken at capacity w (bitset via Vec<bool>).
    let mut keep = vec![vec![false; cap + 1]; n];

    for i in 0..n {
        let ci = costs[i];
        let vi = values[i];
        // Iterate capacity downward for 0/1 semantics.
        for w in (0..=cap).rev() {
            if ci <= w {
                let candidate = dp[w - ci] + vi;
                if candidate > dp[w] {
                    dp[w] = candidate;
                    keep[i][w] = true;
                } else {
                    keep[i][w] = false;
                }
            }
        }
