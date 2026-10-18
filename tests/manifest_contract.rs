use std::fs;
use std::path::Path;

#[test]
fn demo_manifest_carries_the_summary_and_file_decisions() {
    let manifest = fs::read_to_string(Path::new("examples/demo-manifest.json")).expect("manifest");
    for key in ["summary", "files", "budget_tokens", "decision"] {
        assert!(manifest.contains(key), "manifest is missing {key}");
    }
}

#[test]
fn docs_pack_example_documents_the_pack_and_budget() {
    let docs = fs::read_to_string(Path::new("docs/PACK.md")).expect("docs");
    for key in ["budget", "pack", "files"] {
        assert!(docs.contains(key), "PACK.md is missing {key}");
    }
}
