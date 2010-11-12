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
