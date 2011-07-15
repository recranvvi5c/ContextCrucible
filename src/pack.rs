//! The stamping mill: run the full pipeline and cast a context pack + manifest.
//!
//! A *pack* is the concatenated, delimited content of the selected files, ready
//! to paste into an agent's context window. A *manifest* is a JSON document
//! explaining every include / exclude decision so the pour is auditable.

use crate::budget::{self, Item};
use crate::json::Json;
use crate::scan::{self, ScanConfig, ScanResult};
use crate::score::{self, ScoreParts};
use crate::secrets;
use crate::tokens;
use crate::{Candidate, Decision, VERSION};
use std::io;
use std::path::Path;

/// Options controlling a compile.
#[derive(Debug, Clone)]
pub struct PackOptions {
    /// Hard token budget for the pack.
    pub budget: u64,
    /// Free-text relevance query (may be empty).
    pub query: String,
    /// Minimum grade a file must reach to be eligible (0.0 disables the floor).
    pub min_score: f64,
    /// Scan tuning.
    pub scan: ScanConfig,
    /// A short label recorded in the manifest (e.g. the pack name).
    pub label: String,
}

impl Default for PackOptions {
    fn default() -> Self {
        PackOptions {
            budget: 8_000,
            query: String::new(),
            min_score: 0.0,
            scan: ScanConfig::default(),
            label: "pour".into(),
        }
    }
}

/// A fully evaluated file with its final decision.
#[derive(Debug, Clone)]
pub struct Entry {
    pub candidate: Candidate,
    pub decision: Decision,
}

/// The complete result of a compile.
#[derive(Debug, Clone)]
pub struct Pack {
    pub options_budget: u64,
    pub options_query: String,
    pub options_min_score: f64,
    pub label: String,
    pub entries: Vec<Entry>,
    pub tokens_used: u64,
    pub captured_value: f64,
    pub method: String,
    /// Files rejected during scanning, kept for the audit trail.
    pub scan_rejected: Vec<scan::RejectedFile>,
}

impl Pack {
    /// Included entries in fill order.
    pub fn included(&self) -> Vec<&Entry> {
        let mut inc: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| matches!(e.decision, Decision::Included { .. }))
            .collect();
        inc.sort_by_key(|e| match e.decision {
            Decision::Included { rank } => rank,
            _ => usize::MAX,
        });
        inc
    }

    /// Number of files in each disposition bucket.
    pub fn counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut inc = 0;
        let mut sec = 0;
        let mut bud = 0;
        let mut low = 0;
        let mut scan = 0;
        for e in &self.entries {
            match e.decision {
                Decision::Included { .. } => inc += 1,
                Decision::ExcludedSecret => sec += 1,
                Decision::ExcludedBudget => bud += 1,
                Decision::ExcludedLowScore => low += 1,
                Decision::ExcludedScan { .. } => scan += 1,
            }
        }
        (inc, sec, bud, low, scan)
    }
}

/// Compile a pack from a repository root.
pub fn compile(root: &Path, opts: &PackOptions) -> io::Result<Pack> {
    let scan_result = scan::scan(root, &opts.scan)?;
    Ok(compile_from_scan(scan_result, opts))
}

/// Compile from an already-produced scan result (used by tests and comparisons).
pub fn compile_from_scan(scan_result: ScanResult, opts: &PackOptions) -> Pack {
    let terms = score::parse_query(&opts.query);

    // Build candidates with assay + grade + secret scan.
    let mut candidates: Vec<Candidate> = Vec::new();
    for f in scan_result.kept {
        let secrets = secrets::scan(&f.content);
        let import_hit = score::import_relevance(&f.rel_path, &f.content, &terms);
        let parts: ScoreParts = score::grade(&f.rel_path, &f.content, &terms, import_hit);
        let tok = tokens::estimate(&f.content);
        candidates.push(Candidate {
            rel_path: f.rel_path,
            bytes: f.bytes,
            tokens: tok,
            score: parts.total(),
            score_parts: parts,
            secrets,
