//! Ore extraction: walk a repository and reject slag.
//!
//! A file is rejected (never enters the pipeline) if it is:
//!   * inside a well-known vendor / dependency / build directory,
//!   * matched by a generated-artifact name or extension,
//!   * binary (contains a NUL byte in its leading sample, or is > the text
//!     ratio threshold of non-text bytes),
//!   * larger than `max_bytes`.
//!
//! Everything is walked with a manual, deterministic depth-first traversal that
//! sorts each directory's entries so results never depend on filesystem order.

use crate::Decision;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Directories whose entire subtree is skipped.
pub const VENDOR_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    "out",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".gradle",
    ".idea",
    ".vscode",
    "coverage",
    ".next",
    ".nuxt",
];

/// File extensions treated as binary outright (fast path before sampling).
pub const BINARY_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tiff", "pdf", "zip", "gz", "tar", "bz2",
    "xz", "7z", "rar", "jar", "war", "class", "exe", "dll", "so", "dylib", "o", "a", "obj", "bin",
    "wasm", "woff", "woff2", "ttf", "otf", "eot", "mp3", "mp4", "avi", "mov", "mkv", "flac", "wav",
    "ogg", "sqlite", "db", "pyc", "pyo", "lock",
];

/// File names or suffixes that mark generated artefacts.
pub const GENERATED_SUFFIXES: &[&str] = &[
    ".min.js",
    ".min.css",
    ".map",
    ".generated.ts",
    ".g.dart",
    ".pb.go",
    "_pb2.py",
];

/// Exact generated file names.
pub const GENERATED_NAMES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "poetry.lock",
    "composer.lock",
    "Gemfile.lock",
];

/// Tunable scan limits.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Reject files strictly larger than this many bytes.
    pub max_bytes: u64,
    /// Number of leading bytes sampled for binary detection.
    pub sample_bytes: usize,
    /// If the fraction of non-text bytes in the sample exceeds this, treat as
    /// binary. Range `0.0..=1.0`.
    pub binary_ratio: f64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        ScanConfig {
            max_bytes: 512 * 1024,
            sample_bytes: 8192,
            binary_ratio: 0.30,
        }
    }
}

/// A file that survived the scan, ready for assay.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub rel_path: String,
    pub bytes: u64,
    pub content: String,
}

/// A file rejected during scanning, retained for the manifest's audit trail.
#[derive(Debug, Clone)]
pub struct RejectedFile {
    pub rel_path: String,
    pub bytes: u64,
    pub reason: String,
}

/// Combined scan result.
#[derive(Debug, Default)]
pub struct ScanResult {
    pub kept: Vec<ScannedFile>,
    pub rejected: Vec<RejectedFile>,
}

/// Walk `root` and classify every regular file. Deterministic ordering.
pub fn scan(root: &Path, cfg: &ScanConfig) -> io::Result<ScanResult> {
    let mut result = ScanResult::default();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries: Vec<PathBuf> = match fs::read_dir(&dir) {
            Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).collect(),
            Err(_) => continue,
        };
        // Sort for deterministic traversal (directories and files interleaved
        // by path string; we push dirs onto the stack so we reverse-sort to
        // keep overall lexicographic emission order stable).
        entries.sort();
        // Process files first (in order), collect dirs to recurse afterwards.
        let mut subdirs: Vec<PathBuf> = Vec::new();
        for path in entries {
            let file_type = match fs::symlink_metadata(&path) {
                Ok(m) => m.file_type(),
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                // Never follow symlinks: avoids cycles and escaping the root.
                continue;
            }
            if file_type.is_dir() {
                let name = file_name_of(&path);
                if VENDOR_DIRS.contains(&name.as_str()) {
                    continue;
                }
                subdirs.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            classify_file(root, &path, cfg, &mut result)?;
        }
        // Push subdirs in reverse so the smallest path is popped first.
        subdirs.sort();
        for d in subdirs.into_iter().rev() {
            stack.push(d);
        }
    }

    result.kept.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    result.rejected.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(result)
}

fn classify_file(
    root: &Path,
    path: &Path,
    cfg: &ScanConfig,
    result: &mut ScanResult,
) -> io::Result<()> {
    let rel = rel_path(root, path);
    let bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let name = file_name_of(path);
    let lower = name.to_ascii_lowercase();

    if GENERATED_NAMES
        .iter()
        .any(|g| g.eq_ignore_ascii_case(&name))
    {
        result.rejected.push(RejectedFile {
            rel_path: rel,
            bytes,
            reason: "generated:name".into(),
        });
        return Ok(());
    }
    if GENERATED_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
        result.rejected.push(RejectedFile {
            rel_path: rel,
            bytes,
            reason: "generated:suffix".into(),
        });
        return Ok(());
    }
    if let Some(ext) = extension_of(&lower) {
        if BINARY_EXTS.contains(&ext.as_str()) {
            result.rejected.push(RejectedFile {
                rel_path: rel,
                bytes,
                reason: "binary:extension".into(),
            });
            return Ok(());
        }
    }
    if bytes > cfg.max_bytes {
        result.rejected.push(RejectedFile {
            rel_path: rel,
            bytes,
            reason: format!("too-large:{}b", bytes),
        });
        return Ok(());
    }

    // Sample the head for binary detection before committing to a full read.
    let sample = read_sample(path, cfg.sample_bytes)?;
    if is_binary_sample(&sample, cfg.binary_ratio) {
        result.rejected.push(RejectedFile {
            rel_path: rel,
            bytes,
            reason: "binary:content".into(),
        });
        return Ok(());
