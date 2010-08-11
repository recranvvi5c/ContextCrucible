//! Token assay.
//!
//! We do **not** claim to reproduce any particular tokenizer's vocabulary — that
//! would require shipping a model's merge table. Instead we use a transparent,
//! deterministic heuristic calibrated against the rough behaviour of byte-pair
//! encoders on source code: tokens tend to break on word boundaries, runs of
//! whitespace collapse, and punctuation is comparatively dense.
//!
//! The estimator is a pure function of the input bytes, so two runs over the
//! same file always agree.

/// Estimate the token mass of a UTF-8 string.
///
/// The heuristic counts *segments*: maximal runs of alphanumeric characters are
/// one segment each but long identifiers are split every four characters (BPE
/// tends to fragment `snake_case_identifiers`), and each punctuation or symbol
/// character counts as its own segment. Whitespace runs collapse to a single
/// low-cost segment. The result is clamped to at least 1 for non-empty input.
pub fn estimate(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let mut tokens: u64 = 0;
    let mut run_len: u32 = 0;
    let mut in_space = false;

    let flush_word = |run_len: u32, tokens: &mut u64| {
        if run_len == 0 {
