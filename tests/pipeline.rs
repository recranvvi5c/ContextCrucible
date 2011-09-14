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
