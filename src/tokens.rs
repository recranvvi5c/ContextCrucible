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
            return;
        }
        // Roughly four characters per sub-token, minimum one.
        *tokens += (run_len.div_ceil(4)).max(1) as u64;
    };

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if in_space {
                in_space = false;
            }
            run_len += 1;
        } else if ch.is_whitespace() {
            flush_word(run_len, &mut tokens);
            run_len = 0;
            if !in_space {
                // A whitespace run is cheap but not free.
                tokens += 1;
                in_space = true;
            }
        } else {
            // Punctuation / symbol: flush any pending word then count the symbol.
            flush_word(run_len, &mut tokens);
            run_len = 0;
            in_space = false;
            tokens += 1;
        }
    }
