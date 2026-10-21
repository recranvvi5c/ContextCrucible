use contextcrucible::budget::{solve, Item};

fn item(id: &str, tokens: u64, score: f64) -> Item {
    Item { id: id.to_string(), tokens, score }
}

#[test]
fn exact_fit_is_poured() {
    let items = vec![item("a", 100, 10.0)];
    let solution = solve(&items, 100);
    assert_eq!(solution.selected, vec!["a".to_string()]);
    assert_eq!(solution.tokens_used, 100);
}

#[test]
fn one_token_over_is_skipped() {
    let items = vec![item("a", 101, 10.0)];
    let solution = solve(&items, 100);
    assert!(solution.selected.is_empty());
    assert_eq!(solution.tokens_used, 0);
}

#[test]
fn a_huge_budget_does_not_wrap_the_running_total() {
    let items = vec![item("a", u64::MAX - 5, 10.0), item("b", 10, 9.0), item("c", 6, 8.0)];
    let solution = solve(&items, u64::MAX);
    assert!(solution.tokens_used <= u64::MAX);
    for id in &solution.selected {
        assert!(id == "a" || id == "b" || id == "c");
    }
}
