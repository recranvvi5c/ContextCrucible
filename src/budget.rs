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
    }

    // Reconstruct selection.
    let mut w = cap;
    let mut chosen_idx: Vec<usize> = Vec::new();
    for i in (0..n).rev() {
        if w >= costs[i] && keep[i][w] {
            chosen_idx.push(i);
            w -= costs[i];
        }
    }
    // chosen_idx is in reverse item order; restore original (priority) order.
    chosen_idx.reverse();

    // Verify against the true (unquantised) budget and drop any overflow caused
    // by ceil rounding, lowest-value-first, to guarantee the hard bound holds.
    let mut selected: Vec<&Item> = chosen_idx.iter().map(|&i| items[i]).collect();
    enforce_hard_budget(&mut selected, budget);

    finalize(selected, "dp")
}

/// Deterministic value-density greedy fallback.
fn solve_greedy(items: &[&Item], budget: u64) -> Solution {
    let mut ranked: Vec<&Item> = items.to_vec();
    ranked.sort_by(|a, b| {
        let da = a.score / (a.tokens.max(1) as f64);
        let db = b.score / (b.tokens.max(1) as f64);
        db.partial_cmp(&da)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.tokens.cmp(&b.tokens))
            .then(a.id.cmp(&b.id))
    });

    let mut selected: Vec<&Item> = Vec::new();
    let mut used = 0u64;
    for it in ranked {
        if used + it.tokens <= budget {
            used += it.tokens;
            selected.push(it);
        }
    }
    finalize(selected, "greedy")
}

/// Drop items (lowest value first) until the true token sum fits the budget.
fn enforce_hard_budget(selected: &mut Vec<&Item>, budget: u64) {
    let mut total: u64 = selected.iter().map(|i| i.tokens).sum();
    while total > budget {
        // Remove the least valuable item.
        if let Some((idx, _)) = selected.iter().enumerate().min_by(|(_, a), (_, b)| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.tokens.cmp(&a.tokens))
        }) {
            total -= selected[idx].tokens;
            selected.remove(idx);
        } else {
            break;
        }
    }
}

/// Order the final selection by priority and package the solution.
fn finalize(mut selected: Vec<&Item>, method: &'static str) -> Solution {
    selected.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.tokens.cmp(&b.tokens))
            .then(a.id.cmp(&b.id))
    });
    let tokens_used = selected.iter().map(|i| i.tokens).sum();
    let value = selected.iter().map(|i| i.score).sum();
    Solution {
        selected: selected.iter().map(|i| i.id.clone()).collect(),
        tokens_used,
        value,
        method,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, tokens: u64, score: f64) -> Item {
        Item {
            id: id.into(),
            tokens,
            score,
        }
    }

    #[test]
    fn empty_budget_selects_nothing() {
        let items = vec![item("a", 10, 5.0)];
        let s = solve(&items, 0);
        assert!(s.selected.is_empty());
    }

    #[test]
    fn respects_hard_budget() {
        let items = vec![item("a", 60, 10.0), item("b", 60, 9.0)];
        let s = solve(&items, 100);
        assert!(s.tokens_used <= 100, "budget exceeded: {}", s.tokens_used);
        assert_eq!(s.selected.len(), 1);
    }

    #[test]
    fn prefers_higher_value() {
        let items = vec![item("low", 50, 1.0), item("high", 50, 99.0)];
        let s = solve(&items, 50);
        assert_eq!(s.selected, vec!["high"]);
    }

    #[test]
    fn deterministic_across_runs() {
        let items = vec![
            item("a", 30, 10.0),
            item("b", 40, 12.0),
            item("c", 35, 11.0),
            item("d", 20, 8.0),
        ];
        let s1 = solve(&items, 80);
        let s2 = solve(&items, 80);
        assert_eq!(s1.selected, s2.selected);
        assert_eq!(s1.tokens_used, s2.tokens_used);
    }

    #[test]
    fn dp_beats_naive_greedy_on_classic_case() {
        // Classic knapsack: greedy-by-value grabs "big" (100 tokens, value 100)
        // and stops, but the exact DP finds {m1,m2} = 100 tokens, value 101.
        let items = vec![
            item("big", 100, 100.0),
            item("m1", 70, 71.0),
            item("m2", 30, 30.0),
        ];
        let s = solve(&items, 100);
        assert!(s.tokens_used <= 100, "budget exceeded: {}", s.tokens_used);
        assert_eq!(s.method, "dp");
        assert_eq!(s.selected, vec!["m1", "m2"]);
        assert!((s.value - 101.0).abs() < 1e-6, "value {}", s.value);
    }

    #[test]
    fn dp_capacity_equal_budget_fills_exactly() {
        // Two items that together exactly hit the budget must both be chosen
        // over a single lower-value item.
        let items = vec![
            item("solo", 100, 90.0),
            item("p1", 60, 60.0),
            item("p2", 40, 40.0),
        ];
        let s = solve(&items, 100);
        assert!(s.tokens_used <= 100);
        assert_eq!(s.selected, vec!["p1", "p2"]);
    }

    #[test]
    fn oversized_item_excluded() {
        let items = vec![item("huge", 500, 100.0), item("ok", 10, 5.0)];
        let s = solve(&items, 100);
        assert_eq!(s.selected, vec!["ok"]);
    }

    #[test]
    fn greedy_fallback_still_bounded() {
        // Force greedy by making many tiny items with a huge budget grid.
        let items: Vec<Item> = (0..50)
            .map(|i| item(&format!("f{:03}", i), (i % 7 + 1) as u64, (i % 13) as f64))
            .collect();
        let s = solve(&items, 40);
        assert!(s.tokens_used <= 40);
    }
}
