//! `crucible` — the command-line face of the token foundry.
//!
//! Subcommands:
//!   compile   scan a repo and pour a context pack + manifest
//!   scan      list what the scanner keeps and rejects
//!   compare   weigh two manifests against each other
//!   help / --version
//!
//! The CLI is argument-parsed by hand (stdlib only). All output is
//! deterministic so it can be diffed in CI.

use contextcrucible::compare::{self, Comparison};
use contextcrucible::pack::{self, PackOptions};
use contextcrucible::scan::ScanConfig;
use contextcrucible::VERSION;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("crucible: error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");
    match cmd {
        "--version" | "-V" => {
            println!("crucible {}", VERSION);
            Ok(ExitCode::SUCCESS)
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(ExitCode::SUCCESS)
        }
        "compile" => cmd_compile(&args[1..]),
        "scan" => cmd_scan(&args[1..]),
        "compare" => cmd_compare(&args[1..]),
        other => Err(format!(
            "unknown subcommand `{}` (try `crucible help`)",
            other
        )),
    }
}

fn print_help() {
    println!(
        r#"crucible {ver} — budget-aware context compiler (the token foundry)

USAGE:
    crucible <command> [options]

COMMANDS:
    compile   Pour a context pack from a repository
    scan      Report kept / rejected files from a scan
    compare   Weigh two manifests against each other
    help      Show this message

COMPILE OPTIONS:
    --path <dir>         Repository root to scan            (default: .)
    --budget <tokens>    Hard token budget                  (default: 8000)
    --query <text>       Relevance query (quote it)         (default: none)
    --min-score <f>      Drop files graded below this floor (default: 0)
    --max-bytes <n>      Skip files larger than n bytes     (default: 524288)
    --label <name>       Label recorded in the manifest     (default: pour)
    --out <file>         Write pack content here            (default: stdout)
    --manifest <file>    Write JSON manifest here           (optional)

SCAN OPTIONS:
    --path <dir>         Repository root to scan            (default: .)
    --max-bytes <n>      Size cap                           (default: 524288)

COMPARE OPTIONS:
    --a <manifest.json>  Baseline manifest
    --b <manifest.json>  Candidate manifest
    --json               Emit JSON instead of a text report

EXAMPLES:
    crucible compile --path . --budget 4000 --query "budget solver" \
        --out pack.txt --manifest manifest.json
    crucible scan --path .
    crucible compare --a base.json --b candidate.json
"#,
        ver = VERSION
    );
}

// ---------------------------------------------------------------------------
// compile
// ---------------------------------------------------------------------------

fn cmd_compile(args: &[String]) -> Result<ExitCode, String> {
    let opt = Options::parse(args)?;
    let root = opt.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!("path is not a directory: {}", root.display()));
    }

    let mut scan = ScanConfig::default();
    if let Some(mb) = opt.max_bytes {
        scan.max_bytes = mb;
    }

    let options = PackOptions {
        budget: opt.budget.unwrap_or(8_000),
        query: opt.query.clone().unwrap_or_default(),
        min_score: opt.min_score.unwrap_or(0.0),
        scan,
        label: opt.label.clone().unwrap_or_else(|| "pour".into()),
    };

    let pack = pack::compile(&root, &options).map_err(|e| format!("scan failed: {}", e))?;

    let pack_text = pack::render_pack(&pack);
    match &opt.out {
        Some(p) => {
            fs::write(p, &pack_text).map_err(|e| format!("writing pack: {}", e))?;
            eprintln!("crucible: pack written to {}", p.display());
        }
        None => print!("{}", pack_text),
    }

    if let Some(mp) = &opt.manifest {
        let manifest = pack::render_manifest(&pack);
        fs::write(mp, &manifest).map_err(|e| format!("writing manifest: {}", e))?;
        eprintln!("crucible: manifest written to {}", mp.display());
    }

    let (inc, sec, bud, low, scan_ex) = pack.counts();
    eprintln!(
        "crucible: poured {} file(s), {} tokens / {} budget ({:.1}% util) via {}; \
         excluded secret={} budget={} low-score={} scan={}",
        inc,
        pack.tokens_used,
        pack.options_budget,
        pack::utilization(&pack) * 100.0,
        pack.method,
        sec,
        bud,
        low,
        scan_ex + pack.scan_rejected.len(),
    );
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// scan
// ---------------------------------------------------------------------------

fn cmd_scan(args: &[String]) -> Result<ExitCode, String> {
    let opt = Options::parse(args)?;
    let root = opt.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!("path is not a directory: {}", root.display()));
    }
    let mut cfg = ScanConfig::default();
    if let Some(mb) = opt.max_bytes {
        cfg.max_bytes = mb;
    }
    let result = contextcrucible::scan::scan(&root, &cfg).map_err(|e| e.to_string())?;
    println!("kept {} file(s):", result.kept.len());
    for f in &result.kept {
        println!("  + {:>8}b  {}", f.bytes, f.rel_path);
    }
    println!("rejected {} file(s):", result.rejected.len());
    for r in &result.rejected {
        println!("  - {:<22} {}", r.reason, r.rel_path);
    }
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// compare
// ---------------------------------------------------------------------------

fn cmd_compare(args: &[String]) -> Result<ExitCode, String> {
    let opt = Options::parse(args)?;
    let a = opt
        .a
        .as_ref()
        .ok_or("compare requires --a <manifest.json>")?;
    let b = opt
        .b
        .as_ref()
        .ok_or("compare requires --b <manifest.json>")?;

    let man_a = ManifestView::load(a)?;
    let man_b = ManifestView::load(b)?;

    let cmp: Comparison = compare::compare_sets(&man_a.into_stats(), &man_b.into_stats());

    if opt.json {
        println!("{}", cmp.to_json());
    } else {
        print!("{}", cmp.to_report());
    }
    Ok(ExitCode::SUCCESS)
}

/// The subset of a manifest the compare command needs.
struct ManifestView {
    label: String,
    included: BTreeSet<String>,
    tokens_used: u64,
    captured_value: f64,
    budget: u64,
}

impl ManifestView {
    fn load(path: &Path) -> Result<ManifestView, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("reading manifest {}: {}", path.display(), e))?;
        Ok(ManifestView {
            label: extract_string(&text, "label").unwrap_or_else(|| path.display().to_string()),
            tokens_used: extract_number(&text, "tokens_used").unwrap_or(0.0) as u64,
            captured_value: extract_number(&text, "captured_value").unwrap_or(0.0),
            budget: extract_number(&text, "budget_tokens").unwrap_or(0.0) as u64,
            included: extract_included(&text),
        })
    }

    fn into_stats(self) -> compare::ManifestStats {
        compare::ManifestStats {
            label: self.label,
            included: self.included,
            tokens_used: self.tokens_used,
            value: self.captured_value,
            budget: self.budget,
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal manifest field extraction (stdlib only, tolerant of our own format)
// ---------------------------------------------------------------------------

fn extract_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let body = &rest[1..];
    let end = body.find('"')?;
    Some(body[..end].to_string())
}

fn extract_number(json: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(rest.len());
    rest[..end].parse::<f64>().ok()
}

/// Collect the `path` of every file entry whose `decision` is `include`.
fn extract_included(json: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    // Walk each object beginning with a "path" key inside the "files" array.
    let mut cursor = 0;
    while let Some(rel) = json[cursor..].find("\"path\":") {
        let abs = cursor + rel;
        // path value
        let after = abs + "\"path\":".len();
        let seg = &json[after..];
        let seg_trim = seg.trim_start();
        if !seg_trim.starts_with('"') {
            cursor = after;
            continue;
        }
        let body = &seg_trim[1..];
        let path = match body.find('"') {
            Some(end) => body[..end].to_string(),
            None => {
                cursor = after;
                continue;
            }
        };
        // Look ahead for the decision within a bounded window of this entry.
        let window_end = (abs + 400).min(json.len());
        let window = &json[abs..window_end];
        if window.contains("\"decision\": \"include\"") {
            set.insert(path);
        }
        cursor = after;
    }
    set
}

// ---------------------------------------------------------------------------
// Hand-rolled option parsing
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Options {
    path: Option<PathBuf>,
    budget: Option<u64>,
    query: Option<String>,
    min_score: Option<f64>,
    max_bytes: Option<u64>,
    label: Option<String>,
    out: Option<PathBuf>,
    manifest: Option<PathBuf>,
    a: Option<PathBuf>,
    b: Option<PathBuf>,
    json: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Options, String> {
        let mut o = Options::default();
        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            let mut take = || {
                i += 1;
                args.get(i)
                    .cloned()
                    .ok_or_else(|| format!("missing value for {}", arg))
            };
            match arg {
                "--path" => o.path = Some(PathBuf::from(take()?)),
                "--budget" => o.budget = Some(parse_u64(&take()?, "--budget")?),
                "--query" => o.query = Some(take()?),
                "--min-score" => o.min_score = Some(parse_f64(&take()?, "--min-score")?),
                "--max-bytes" => o.max_bytes = Some(parse_u64(&take()?, "--max-bytes")?),
                "--label" => o.label = Some(take()?),
                "--out" => o.out = Some(PathBuf::from(take()?)),
                "--manifest" => o.manifest = Some(PathBuf::from(take()?)),
                "--a" => o.a = Some(PathBuf::from(take()?)),
                "--b" => o.b = Some(PathBuf::from(take()?)),
                "--json" => o.json = true,
                other => return Err(format!("unknown option `{}`", other)),
            }
            i += 1;
        }
        Ok(o)
    }
}

fn parse_u64(s: &str, flag: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .map_err(|_| format!("{} expects a non-negative integer, got `{}`", flag, s))
}

fn parse_f64(s: &str, flag: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|_| format!("{} expects a number, got `{}`", flag, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compile_options() {
        let args = vec![
            "--path".into(),
            "x".into(),
            "--budget".into(),
            "1234".into(),
            "--query".into(),
            "budget solver".into(),
        ];
        let o = Options::parse(&args).unwrap();
        assert_eq!(o.budget, Some(1234));
        assert_eq!(o.query.as_deref(), Some("budget solver"));
    }

    #[test]
    fn rejects_unknown_option() {
        let args = vec!["--nope".into()];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn extract_string_reads_label() {
        let json = "{\n  \"label\": \"alpha\"\n}";
        assert_eq!(extract_string(json, "label").as_deref(), Some("alpha"));
    }

    #[test]
    fn extract_number_reads_tokens() {
        let json = "{\n  \"tokens_used\": 4210\n}";
        assert_eq!(extract_number(json, "tokens_used"), Some(4210.0));
    }

    #[test]
    fn extract_included_finds_only_includes() {
        let json = r#"{
  "files": [
    { "path": "a.rs", "decision": "include", "tokens": 3 },
    { "path": "b.rs", "decision": "exclude:budget", "tokens": 3 }
  ]
}"#;
        let set = extract_included(json);
        assert!(set.contains("a.rs"));
        assert!(!set.contains("b.rs"));
    }
}
