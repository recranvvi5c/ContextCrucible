//! End-to-end integration tests over the bundled `fixtures/sample-repo`.
//!
//! These exercise the whole foundry pipeline: scan → assay → grade → secret
//! quarantine → budget pour → manifest, plus pack comparison.

use contextcrucible::compare;
use contextcrucible::pack::{self, PackOptions};
use contextcrucible::scan::ScanConfig;
use contextcrucible::Decision;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("fixtures");
    p.push("sample-repo");
    p
}

fn compile(budget: u64, query: &str, label: &str) -> pack::Pack {
    let opts = PackOptions {
        budget,
        query: query.to_string(),
        min_score: 0.0,
        scan: ScanConfig::default(),
        label: label.to_string(),
    };
    pack::compile(&fixture_root(), &opts).expect("compile fixture")
}

#[test]
fn scanner_rejects_generated_and_keeps_source() {
    let cfg = ScanConfig::default();
    let result = contextcrucible::scan::scan(&fixture_root(), &cfg).expect("scan");

    let kept: Vec<&str> = result.kept.iter().map(|f| f.rel_path.as_str()).collect();
    assert!(kept.iter().any(|p| p.ends_with("budget_solver.rs")));
    assert!(kept.iter().any(|p| p.ends_with("main.py")));

    // The minified bundle must be rejected as generated.
    let rejected: Vec<&str> = result
        .rejected
        .iter()
        .map(|r| r.rel_path.as_str())
        .collect();
    assert!(
        rejected.iter().any(|p| p.ends_with("bundle.min.js")),
        "min.js should be rejected, rejected = {:?}",
        rejected
    );
}

#[test]
fn secret_file_is_quarantined() {
    let pack = compile(50_000, "", "secret-check");
    let entry = pack
        .entries
        .iter()
        .find(|e| e.candidate.rel_path.ends_with("secrets.yaml"))
        .expect("secrets.yaml must be a candidate");
    assert_eq!(entry.decision, Decision::ExcludedSecret);
    assert!(!entry.candidate.secrets.is_empty());
}

#[test]
fn query_prioritises_budget_solver_under_tight_budget() {
    // Tight budget: budget_solver.rs (~546 tokens, grade 83) is by far the most
    // relevant to "budget solver knapsack" and is the only file that fits.
    let pack = compile(600, "budget solver knapsack", "tight");
    let included: Vec<String> = pack
        .included()
        .iter()
        .map(|e| e.candidate.rel_path.clone())
        .collect();
    assert!(
        included.iter().any(|p| p.ends_with("budget_solver.rs")),
        "budget_solver.rs should win the tight budget, got {:?}",
        included
    );
    assert!(
        pack.tokens_used <= 600,
        "hard budget breached: {}",
        pack.tokens_used
    );
}

#[test]
fn budget_is_a_hard_bound() {
    for budget in [10u64, 100, 500, 2000] {
        let pack = compile(budget, "budget", "bound");
        assert!(
            pack.tokens_used <= budget,
            "budget {} breached with {} tokens",
            budget,
            pack.tokens_used
        );
    }
}

#[test]
fn compilation_is_deterministic() {
    let a = compile(1500, "budget solver", "det-a");
    let b = compile(1500, "budget solver", "det-b");
    let ain: Vec<String> = a
        .included()
        .iter()
        .map(|e| e.candidate.rel_path.clone())
        .collect();
    let bin: Vec<String> = b
        .included()
        .iter()
        .map(|e| e.candidate.rel_path.clone())
        .collect();
    assert_eq!(ain, bin);
    assert_eq!(a.tokens_used, b.tokens_used);
}

#[test]
fn manifest_explains_every_candidate() {
    let pack = compile(1500, "budget", "manifest");
    let manifest = pack::render_manifest(&pack);
    // Every kept candidate path should appear in the manifest.
    for entry in &pack.entries {
        assert!(
            manifest.contains(&entry.candidate.rel_path),
            "manifest missing {}",
            entry.candidate.rel_path
        );
    }
    assert!(manifest.contains("\"explanation\""));
    assert!(manifest.contains("\"solver_method\""));
}

#[test]
fn larger_budget_captures_at_least_as_much_value() {
    let small = compile(600, "budget solver", "small");
    let large = compile(50_000, "budget solver", "large");
    assert!(
        large.captured_value >= small.captured_value,
        "more budget should not reduce captured value: {} < {}",
        large.captured_value,
        small.captured_value
    );
    let cmp = compare::compare(&small, &large);
    // Growing the budget can only add or retain files, never spontaneously
    // remove one that still fits.
    assert!(
        cmp.removed.is_empty(),
        "unexpected removals: {:?}",
        cmp.removed
    );
}

#[test]
fn comparison_report_renders() {
    let a = compile(600, "budget", "a");
    let b = compile(3000, "budget", "b");
    let cmp = compare::compare(&a, &b);
    let report = cmp.to_report();
    assert!(report.contains("crucible compare"));
    let json = cmp.to_json();
    assert!(json.contains("\"token_delta\""));
}
