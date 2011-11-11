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
