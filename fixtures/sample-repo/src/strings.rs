// An unrelated utility fixture. Low relevance to a "budget solver" query, so
// contextcrucible should deprioritise it under a tight budget.

/// Convert a byte count into a human-readable size string.
pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
