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
