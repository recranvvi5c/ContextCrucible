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
            language: detect_language(&f.content),
            content: f.content,
        });
    }
    candidates.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    // Partition: quarantine secrets and low scores before the solver sees them.
    let mut entries: Vec<Entry> = Vec::new();
    let mut solver_items: Vec<Item> = Vec::new();

    for cand in candidates {
        if cand.is_quarantined() {
            entries.push(Entry {
                candidate: cand,
                decision: Decision::ExcludedSecret,
            });
            continue;
        }
        if cand.score < opts.min_score {
            entries.push(Entry {
                candidate: cand,
                decision: Decision::ExcludedLowScore,
            });
            continue;
        }
        solver_items.push(Item {
            id: cand.rel_path.clone(),
            tokens: cand.tokens,
            score: cand.score,
        });
        entries.push(Entry {
            candidate: cand,
            decision: Decision::ExcludedBudget,
        });
    }

    // Solve the hard-budget selection.
    let solution = budget::solve(&solver_items, opts.budget);

    // Apply the solution: promote selected entries to Included with their rank.
    for (rank, id) in solution.selected.iter().enumerate() {
        if let Some(e) = entries.iter_mut().find(|e| &e.candidate.rel_path == id) {
            e.decision = Decision::Included { rank };
        }
    }

    Pack {
        options_budget: opts.budget,
        options_query: opts.query.clone(),
        options_min_score: opts.min_score,
        label: opts.label.clone(),
        entries,
        tokens_used: solution.tokens_used,
        captured_value: solution.value,
        method: solution.method.to_string(),
        scan_rejected: scan_result.rejected,
    }
}

/// Render the pack content: a delimited concatenation of included files.
pub fn render_pack(pack: &Pack) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# contextcrucible pack :: {} :: budget={} tokens :: used={} :: method={}\n",
        pack.label, pack.options_budget, pack.tokens_used, pack.method
    ));
    if !pack.options_query.is_empty() {
        out.push_str(&format!("# query: {}\n", pack.options_query));
    }
    out.push_str(&format!("# files: {}\n\n", pack.included().len()));
    for entry in pack.included() {
        let c = &entry.candidate;
        out.push_str(&format!(
            "===== BEGIN {} ({} tokens, grade {:.1}, {}) =====\n",
            c.rel_path, c.tokens, c.score, c.language
        ));
        out.push_str(&c.content);
        if !c.content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("===== END {} =====\n\n", c.rel_path));
    }
    out
}

/// Render the explanatory manifest as pretty JSON.
pub fn render_manifest(pack: &Pack) -> String {
    let (inc, sec, bud, low, scan_ex) = pack.counts();

    let summary = Json::Object(vec![
        ("tool".into(), Json::s("contextcrucible")),
        ("version".into(), Json::s(VERSION)),
        ("label".into(), Json::s(&pack.label)),
        ("query".into(), Json::s(&pack.options_query)),
        (
            "budget_tokens".into(),
            Json::Int(pack.options_budget as i64),
        ),
        ("tokens_used".into(), Json::Int(pack.tokens_used as i64)),
        ("budget_utilization".into(), Json::Float(utilization(pack))),
        ("captured_value".into(), Json::Float(pack.captured_value)),
        ("min_score".into(), Json::Float(pack.options_min_score)),
        ("solver_method".into(), Json::s(&pack.method)),
        (
            "counts".into(),
            Json::Object(vec![
                ("included".into(), Json::Int(inc as i64)),
                ("excluded_secret".into(), Json::Int(sec as i64)),
                ("excluded_budget".into(), Json::Int(bud as i64)),
                ("excluded_low_score".into(), Json::Int(low as i64)),
                (
                    "excluded_scan".into(),
                    Json::Int((scan_ex + pack.scan_rejected.len()) as i64),
                ),
            ]),
        ),
    ]);

    // File-level decisions (evaluated candidates).
    let mut file_entries: Vec<Json> = Vec::new();
    let included = pack.included();
    for entry in &pack.entries {
        let c = &entry.candidate;
        let rank = included
            .iter()
            .position(|e| e.candidate.rel_path == c.rel_path);
        let mut obj = vec![
            ("path".into(), Json::s(&c.rel_path)),
            ("decision".into(), Json::s(entry.decision.code())),
            ("tokens".into(), Json::Int(c.tokens as i64)),
            ("bytes".into(), Json::Int(c.bytes as i64)),
            ("grade".into(), Json::Float(c.score)),
            ("language".into(), Json::s(&c.language)),
            (
                "score_parts".into(),
                Json::Object(vec![
                    ("path".into(), Json::Float(c.score_parts.path)),
                    ("query".into(), Json::Float(c.score_parts.query)),
                    ("import".into(), Json::Float(c.score_parts.import)),
                    ("symbol".into(), Json::Float(c.score_parts.symbol)),
                ]),
            ),
        ];
        if let Some(r) = rank {
            obj.push(("fill_rank".into(), Json::Int(r as i64)));
        }
        if !c.secrets.is_empty() {
            let findings: Vec<Json> = c
                .secrets
                .iter()
                .map(|f| {
                    Json::Object(vec![
                        ("rule".into(), Json::s(&f.rule)),
                        ("line".into(), Json::Int(f.line as i64)),
                        ("confidence".into(), Json::Float(f.confidence)),
                        ("redacted".into(), Json::s(&f.redacted)),
                    ])
                })
                .collect();
            obj.push(("secrets".into(), Json::Array(findings)));
        }
        if let Decision::ExcludedScan { reason } = &entry.decision {
            obj.push(("scan_reason".into(), Json::s(reason)));
        }
        obj.push(("explanation".into(), Json::s(explain(entry, rank))));
        file_entries.push(Json::Object(obj));
    }

    // Scan-rejected files (never became candidates).
    let mut rejected_entries: Vec<Json> = Vec::new();
    for r in &pack.scan_rejected {
        rejected_entries.push(Json::Object(vec![
            ("path".into(), Json::s(&r.rel_path)),
            ("decision".into(), Json::s("exclude:scan")),
            ("bytes".into(), Json::Int(r.bytes as i64)),
            ("scan_reason".into(), Json::s(&r.reason)),
        ]));
    }

    let root = Json::Object(vec![
        ("summary".into(), summary),
        ("files".into(), Json::Array(file_entries)),
        ("scan_rejected".into(), Json::Array(rejected_entries)),
    ]);
    root.to_pretty()
}

/// Budget utilisation in `0.0..=1.0`.
pub fn utilization(pack: &Pack) -> f64 {
    if pack.options_budget == 0 {
        0.0
    } else {
        pack.tokens_used as f64 / pack.options_budget as f64
    }
}

fn explain(entry: &Entry, rank: Option<usize>) -> String {
    let c = &entry.candidate;
    match &entry.decision {
        Decision::Included { .. } => format!(
            "included at fill rank {} — grade {:.1} (path {:.1}/query {:.1}/import {:.1}/symbol {:.1}) for {} tokens",
            rank.unwrap_or(0),
            c.score,
            c.score_parts.path,
            c.score_parts.query,
            c.score_parts.import,
            c.score_parts.symbol,
            c.tokens
        ),
        Decision::ExcludedSecret => format!(
            "quarantined — {} secret finding(s), highest confidence {:.2}",
            c.secrets.len(),
            c.secrets.iter().map(|f| f.confidence).fold(0.0, f64::max)
        ),
        Decision::ExcludedBudget => format!(
            "not poured — grade {:.1} at {} tokens lost the budget contest",
            c.score, c.tokens
        ),
        Decision::ExcludedLowScore => format!(
            "below floor — grade {:.1} < min_score",
            c.score
        ),
        Decision::ExcludedScan { reason } => format!("rejected during scan — {}", reason),
    }
}

/// Best-effort language label from content shape / shebang.
pub fn detect_language(content: &str) -> String {
    let head = content.trim_start();
    if head.starts_with("#!") {
        if head.contains("python") {
            return "python".into();
        }
        if head.contains("node") {
            return "javascript".into();
        }
        if head.contains("bash") || head.contains("/sh") {
            return "shell".into();
        }
    }
    if head.contains("fn main(") || head.contains("pub fn ") || head.contains("impl ") {
        return "rust".into();
    }
    if head.contains("interface ") || head.contains(": string") || head.contains("export const") {
        return "typescript".into();
    }
    if head.contains("def ") && head.contains(':') {
        return "python".into();
