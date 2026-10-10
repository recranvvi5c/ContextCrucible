//! Ore grading: score each file's relevance to the task at hand.
//!
//! Four transparent signals are combined into a 0–100 grade. None of them use
//! learned embeddings — every point is traceable to a concrete token match, so
//! the manifest can explain exactly *why* a file scored the way it did.
//!
//! | Signal   | Weight | What it rewards                                        |
//! |----------|--------|--------------------------------------------------------|
//! | path     | 25     | query terms appearing in the file path                 |
//! | query    | 35     | query terms appearing in the file body (freq-capped)   |
//! | import   | 20     | files that import / are imported by query-named modules|
//! | symbol   | 20     | definitions (fn/class/struct/def) matching query terms |

/// Per-signal contribution, retained for the manifest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScoreParts {
    pub path: f64,
    pub query: f64,
    pub import: f64,
    pub symbol: f64,
}

impl ScoreParts {
    /// Total grade, clamped to `0.0..=100.0`.
    pub fn total(&self) -> f64 {
        (self.path + self.query + self.import + self.symbol).clamp(0.0, 100.0)
    }
}

/// Signal weights (must sum to 100).
const W_PATH: f64 = 25.0;
const W_QUERY: f64 = 35.0;
const W_IMPORT: f64 = 20.0;
const W_SYMBOL: f64 = 20.0;

/// Normalise a free-text query into lowercase alphanumeric terms (length >= 2).
pub fn parse_query(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_ascii_lowercase())
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

/// Grade a single file.
///
/// * `rel_path` — repo-relative path.
/// * `content`  — file body.
/// * `terms`    — normalised query terms from [`parse_query`].
/// * `import_hit` — whether import analysis linked this file to the query.
pub fn grade(rel_path: &str, content: &str, terms: &[String], import_hit: bool) -> ScoreParts {
    if terms.is_empty() {
        // With no query, grade purely on structural centrality: shallow,
        // source-like files score a flat baseline so the budget still fills.
        let baseline = baseline_score(rel_path);
        return ScoreParts {
            path: baseline,
            query: 0.0,
            import: 0.0,
            symbol: 0.0,
        };
    }

    let path_lower = rel_path.to_ascii_lowercase();
    let body_lower = content.to_ascii_lowercase();

    // --- path signal ---
    let path_hits = terms
        .iter()
        .filter(|t| path_lower.contains(t.as_str()))
        .count();
    let path = if terms.is_empty() {
        0.0
    } else {
        W_PATH * (path_hits as f64 / terms.len() as f64)
    };

    // --- query signal (frequency, capped so long files can't dominate) ---
    let mut query_frac = 0.0;
    for t in terms {
        let occurrences = count_occurrences(&body_lower, t);
        // Diminishing returns: 0 -> 0, 1 -> 0.6, >=3 -> 1.0
        let term_score = match occurrences {
            0 => 0.0,
            1 => 0.6,
            2 => 0.85,
            _ => 1.0,
        };
        query_frac += term_score;
    }
    let query = W_QUERY * (query_frac / terms.len() as f64);

    // --- import signal ---
    let import = if import_hit { W_IMPORT } else { 0.0 };

    // --- symbol signal ---
    let symbols = extract_symbols(content);
    let symbol_hits = terms
        .iter()
        .filter(|t| symbols.iter().any(|s| s.contains(t.as_str())))
        .count();
    let symbol = if terms.is_empty() {
        0.0
    } else {
        W_SYMBOL * (symbol_hits as f64 / terms.len() as f64)
    };

    ScoreParts {
        path,
        query,
        import,
        symbol,
    }
}

/// Baseline structural score used when no query is supplied: rewards shallow
/// paths and recognised source extensions.
fn baseline_score(rel_path: &str) -> f64 {
    let depth = rel_path.matches('/').count();
    let depth_score = W_PATH * (1.0 / (1.0 + depth as f64));
    let ext_bonus = if is_source_like(rel_path) { 6.0 } else { 0.0 };
    (depth_score + ext_bonus).min(W_PATH)
}

fn is_source_like(rel_path: &str) -> bool {
    const EXTS: &[&str] = &[
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".java", ".rb", ".c", ".h", ".cpp",
        ".hpp", ".cs", ".swift", ".kt", ".php",
    ];
    let lower = rel_path.to_ascii_lowercase();
    EXTS.iter().any(|e| lower.ends_with(e))
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        count += 1;
        start += pos + needle.len();
    }
    count
}

/// Extract lowercase definition names from common languages via lightweight
/// keyword scanning. This is not a full parser — it recognises the leading
/// token after `fn`/`def`/`class`/`struct`/`function`/`interface`/`type`.
pub fn extract_symbols(content: &str) -> Vec<String> {
    const KEYWORDS: &[&str] = &[
        "fn",
        "def",
        "class",
        "struct",
        "function",
        "interface",
        "type",
        "enum",
        "trait",
        "impl",
        "const",
        "let",
        "var",
    ];
    let mut names = Vec::new();
    for line in content.lines() {
        let mut words = line.split_whitespace();
        while let Some(w) = words.next() {
            let clean = w.trim_start_matches(['(', '{']);
            if KEYWORDS.contains(&clean) {
                if let Some(name) = words.next() {
                    let ident: String = name
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>()
                        .to_ascii_lowercase();
                    if !ident.is_empty() {
                        names.push(ident);
                    }
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Extract module-ish names referenced by import/use/require/include lines.
/// Used to build the import adjacency for the import signal.
pub fn extract_imports(content: &str) -> Vec<String> {
    let mut mods = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        let lower = t.to_ascii_lowercase();
        let payload = if lower.starts_with("use ") {
            Some(&t[4..])
        } else if lower.starts_with("import ") {
            Some(&t[7..])
        } else if lower.starts_with("from ") {
            Some(&t[5..])
        } else if let Some(idx) = t.find("require(") {
            Some(&t[idx + 8..])
        } else if lower.starts_with("#include") {
            Some(&t[8..])
        } else {
            None
        };
        if let Some(p) = payload {
            for frag in
                p.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '/'))
            {
                let last = frag.rsplit(['.', '/']).next().unwrap_or(frag);
                if last.len() >= 2 && last.chars().next().is_some_and(|c| c.is_alphabetic()) {
                    mods.push(last.to_ascii_lowercase());
                }
            }
        }
    }
    mods.sort();
    mods.dedup();
    mods
}

/// Determine whether a file participates in the import graph relevant to the
/// query: either it imports a query-named module, or its own module stem
/// matches a query term (so importers of it are relevant too).
pub fn import_relevance(rel_path: &str, content: &str, terms: &[String]) -> bool {
    if terms.is_empty() {
        return false;
    }
    let stem = module_stem(rel_path);
    if terms.iter().any(|t| stem.contains(t.as_str())) {
        return true;
    }
    let imports = extract_imports(content);
    imports
        .iter()
        .any(|m| terms.iter().any(|t| m.contains(t.as_str())))
}

fn module_stem(rel_path: &str) -> String {
    let file = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let stem = file.split('.').next().unwrap_or(file);
    stem.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parses_and_dedups() {
        let q = parse_query("Budget budget SOLVER a");
        assert_eq!(q, vec!["budget".to_string(), "solver".to_string()]);
    }

    #[test]
    fn path_signal_rewards_matches() {
        let terms = parse_query("budget");
        let s = grade("src/budget.rs", "fn nothing() {}", &terms, false);
        assert!(s.path > 0.0);
    }

    #[test]
    fn query_signal_diminishes() {
        let terms = parse_query("token");
        let few = grade("a.rs", "token", &terms, false).query;
        let many = grade("a.rs", "token token token token", &terms, false).query;
        assert!(many > few);
        assert!(many <= 35.0);
    }

    #[test]
    fn symbol_signal_detects_definitions() {
        let terms = parse_query("smelt");
        let s = grade("a.rs", "fn smelt_ore() {}", &terms, false);
        assert!(s.symbol > 0.0);
    }

    #[test]
    fn extract_symbols_handles_multiple_langs() {
        let syms = extract_symbols("fn alpha() {}\nclass Beta:\ndef gamma():");
        assert!(syms.contains(&"alpha".to_string()));
        assert!(syms.contains(&"beta".to_string()));
        assert!(syms.contains(&"gamma".to_string()));
    }

    #[test]
    fn extract_imports_handles_langs() {
        let imports = extract_imports("use crate::budget;\nimport os\nfrom foundry import x");
        assert!(imports.contains(&"budget".to_string()));
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"foundry".to_string()));
    }

    #[test]
    fn import_relevance_via_stem() {
        let terms = parse_query("budget");
        assert!(import_relevance("src/budget.rs", "", &terms));
    }

    #[test]
    fn no_query_uses_baseline() {
        let s = grade("main.rs", "fn main() {}", &[], false);
        assert!(s.total() > 0.0);
        assert!(s.query == 0.0);
    }

    #[test]
    fn total_is_clamped() {
        let p = ScoreParts {
            path: 40.0,
            query: 40.0,
            import: 40.0,
            symbol: 40.0,
        };
        assert_eq!(p.total(), 100.0);
    }
}
