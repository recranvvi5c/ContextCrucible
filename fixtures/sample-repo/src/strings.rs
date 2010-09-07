// An unrelated utility fixture. Low relevance to a "budget solver" query, so
// contextcrucible should deprioritise it under a tight budget.

/// Convert a byte count into a human-readable size string.
pub fn human_size(bytes: u64) -> String {
