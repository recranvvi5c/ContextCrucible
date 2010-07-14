//! contextcrucible — a budget-aware context compiler for coding agents.
//!
//! The library is built entirely on the Rust standard library. It is organised
//! as a small pipeline of pure-ish stages that mirror an industrial foundry:
//!
//! 1. [`scan`]   — walk the ore body (repository) and reject slag
//!    (generated / vendor / binary files).
//! 2. [`tokens`] — assay each nugget for its token mass.
//! 3. [`score`]  — grade relevance from path, query, import and symbol signals.
//! 4. [`secrets`]— spark-test for likely credentials and quarantine them.
//! 5. [`budget`] — cast the final pour under a hard token budget.
//! 6. [`pack`]   — stamp the context pack and its explanatory manifest.
//! 7. [`compare`]— weigh two pours against each other.
//!
//! Everything is deterministic: given the same inputs and the same budget the
//! selection, ordering and manifest are byte-for-byte reproducible.

pub mod budget;
pub mod compare;
pub mod json;
pub mod pack;
pub mod scan;
pub mod score;
pub mod secrets;
pub mod tokens;

/// Library version, surfaced by the CLI `--version` flag and embedded in packs.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A single candidate file discovered during a scan, carried through the
/// pipeline and progressively enriched with assay / grade / secret data.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Repository-relative path using forward slashes.
    pub rel_path: String,
    /// Raw byte length on disk.
    pub bytes: u64,
    /// The file's textual content (only present for retained text files).
    pub content: String,
    /// Estimated token mass from [`tokens::estimate`].
    pub tokens: u64,
    /// Relevance grade in the range `0.0..=100.0` from [`score`].
    pub score: f64,
