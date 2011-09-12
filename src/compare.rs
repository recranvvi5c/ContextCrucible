//! The assay balance: weigh two pours against each other.
//!
//! Comparison is done over parsed manifest summaries (or live [`Pack`]s). It
//! reports which files entered one pack but not the other, and how the budget
//! utilisation and captured value shifted.

use crate::json::Json;
use crate::pack::{utilization, Pack};
use crate::Decision;
use std::collections::BTreeSet;

/// The difference between a baseline pack (`a`) and a candidate pack (`b`).
#[derive(Debug, Clone)]
pub struct Comparison {
    pub label_a: String,
    pub label_b: String,
    pub tokens_a: u64,
    pub tokens_b: u64,
    pub value_a: f64,
    pub value_b: f64,
    pub util_a: f64,
    pub util_b: f64,
    /// Files included in `b` but not in `a`.
    pub added: Vec<String>,
    /// Files included in `a` but not in `b`.
    pub removed: Vec<String>,
    /// Files included in both.
    pub retained: Vec<String>,
}

impl Comparison {
    /// Change in captured value, `b - a`.
    pub fn value_delta(&self) -> f64 {
        self.value_b - self.value_a
    }

    /// Change in tokens used, `b - a`.
    pub fn token_delta(&self) -> i64 {
        self.tokens_b as i64 - self.tokens_a as i64
    }

    /// Render a compact human-readable report.
    pub fn to_report(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "crucible compare :: {} → {}\n",
            self.label_a, self.label_b
        ));
        out.push_str(&format!(
            "  tokens : {} → {} ({:+})\n",
            self.tokens_a,
            self.tokens_b,
            self.token_delta()
        ));
        out.push_str(&format!(
            "  value  : {:.1} → {:.1} ({:+.1})\n",
            self.value_a,
            self.value_b,
            self.value_delta()
        ));
        out.push_str(&format!(
            "  util   : {:.1}% → {:.1}%\n",
            self.util_a * 100.0,
            self.util_b * 100.0
        ));
        out.push_str(&format!("  added ({}):\n", self.added.len()));
        for f in &self.added {
            out.push_str(&format!("    + {}\n", f));
        }
        out.push_str(&format!("  removed ({}):\n", self.removed.len()));
        for f in &self.removed {
            out.push_str(&format!("    - {}\n", f));
        }
        out.push_str(&format!("  retained: {} file(s)\n", self.retained.len()));
        out
    }

    /// Render the comparison as JSON.
    pub fn to_json(&self) -> String {
        let arr = |v: &[String]| Json::Array(v.iter().map(Json::s).collect());
        Json::Object(vec![
            ("baseline".into(), Json::s(&self.label_a)),
            ("candidate".into(), Json::s(&self.label_b)),
            ("tokens_a".into(), Json::Int(self.tokens_a as i64)),
            ("tokens_b".into(), Json::Int(self.tokens_b as i64)),
            ("token_delta".into(), Json::Int(self.token_delta())),
            ("value_a".into(), Json::Float(self.value_a)),
            ("value_b".into(), Json::Float(self.value_b)),
            ("value_delta".into(), Json::Float(self.value_delta())),
            ("util_a".into(), Json::Float(self.util_a)),
            ("util_b".into(), Json::Float(self.util_b)),
            ("added".into(), arr(&self.added)),
            ("removed".into(), arr(&self.removed)),
            ("retained".into(), arr(&self.retained)),
        ])
        .to_pretty()
    }
}

/// Included file set of a pack, sorted.
fn included_set(pack: &Pack) -> BTreeSet<String> {
    pack.entries
        .iter()
        .filter(|e| matches!(e.decision, Decision::Included { .. }))
        .map(|e| e.candidate.rel_path.clone())
        .collect()
}

/// Compare two live packs.
pub fn compare(a: &Pack, b: &Pack) -> Comparison {
    let set_a = included_set(a);
    let set_b = included_set(b);

    let added: Vec<String> = set_b.difference(&set_a).cloned().collect();
    let removed: Vec<String> = set_a.difference(&set_b).cloned().collect();
    let retained: Vec<String> = set_a.intersection(&set_b).cloned().collect();

    Comparison {
        label_a: a.label.clone(),
        label_b: b.label.clone(),
        tokens_a: a.tokens_used,
        tokens_b: b.tokens_used,
        value_a: a.captured_value,
        value_b: b.captured_value,
        util_a: utilization(a),
