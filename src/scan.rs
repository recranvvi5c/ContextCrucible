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
