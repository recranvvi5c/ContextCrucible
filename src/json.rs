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

    fn write_pretty(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{}", n);
            }
            Json::Float(f) => {
                if f.is_finite() {
                    // Fixed precision keeps output byte-stable across platforms.
                    let _ = write!(out, "{:.4}", f);
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => write_escaped(out, s),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    push_indent(out, indent + 1);
                    item.write_pretty(out, indent + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                push_indent(out, indent);
                out.push(']');
            }
            Json::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (i, (k, v)) in entries.iter().enumerate() {
                    push_indent(out, indent + 1);
                    write_escaped(out, k);
                    out.push_str(": ");
                    v.write_pretty(out, indent + 1);
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                push_indent(out, indent);
                out.push('}');
            }
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_object_in_order() {
        let j = Json::Object(vec![("b".into(), Json::Int(1)), ("a".into(), Json::Int(2))]);
        let s = j.to_pretty();
        let b_pos = s.find("\"b\"").unwrap();
        let a_pos = s.find("\"a\"").unwrap();
        assert!(b_pos < a_pos, "insertion order must be preserved");
    }

    #[test]
    fn escapes_control_chars() {
        let j = Json::s("line\nbreak\t\"q\"");
        let s = j.to_pretty();
        assert!(s.contains("\\n"));
        assert!(s.contains("\\t"));
        assert!(s.contains("\\\""));
    }

    #[test]
    fn floats_are_fixed_precision() {
        let j = Json::Float(0.5);
        assert_eq!(j.to_pretty().trim(), "0.5000");
    }

    #[test]
    fn empty_containers() {
        assert_eq!(Json::Array(vec![]).to_pretty().trim(), "[]");
        assert_eq!(Json::Object(vec![]).to_pretty().trim(), "{}");
    }
}
