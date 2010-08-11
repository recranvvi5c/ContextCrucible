//! A tiny, dependency-free JSON writer.
//!
//! The project is stdlib-only, so we cannot pull in `serde`. This module offers
//! just enough to emit stable, pretty-printed JSON for manifests and pack
//! metadata. Keys are written in insertion order, giving deterministic output.

use std::fmt::Write as _;

/// A JSON value tree that can be rendered deterministically.
#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Bool(bool),
    /// Integer number (rendered without a decimal point).
    Int(i64),
    /// Floating number (rendered with fixed precision for stability).
    Float(f64),
    Str(String),
    Array(Vec<Json>),
    /// Object preserving insertion order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Convenience constructor for a string value.
    pub fn s<T: Into<String>>(v: T) -> Json {
        Json::Str(v.into())
    }

    /// Render to a pretty-printed string with two-space indentation.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out.push('\n');
        out
    }

