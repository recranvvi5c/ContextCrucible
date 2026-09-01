//! Spark test: detect likely credentials so they never reach an LLM context.
//!
//! Detection is deliberately conservative and rule-based — there are no network
//! calls and no learned models. Each rule reports a confidence in `0.0..=1.0`.
//! Any finding at or above [`QUARANTINE_CONFIDENCE`] excludes the whole file.
//!
//! The rules combine well-known token shapes (AWS access keys, private-key PEM
//! headers, JWTs, Slack / GitHub tokens) with a generic "assignment of a long
//! high-entropy value to a secret-looking name" heuristic.

/// Files with a finding at or above this confidence are excluded outright.
pub const QUARANTINE_CONFIDENCE: f64 = 0.75;

/// A single secret detection within a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// Human-readable rule name (e.g. `aws-access-key-id`).
    pub rule: String,
    /// 1-based line number where the match began.
    pub line: usize,
    /// Detection confidence in `0.0..=1.0`.
    pub confidence: f64,
    /// A redacted excerpt safe to show in a manifest.
    pub redacted: String,
}

/// Scan file content and return all findings, ordered by line.
pub fn scan(content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        detect_line(line, line_no, &mut findings);
    }
    findings.sort_by(|a, b| a.line.cmp(&b.line).then(a.rule.cmp(&b.rule)));
    findings
}

fn detect_line(line: &str, line_no: usize, out: &mut Vec<Finding>) {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") && !trimmed.contains('=') {
        // Skip obvious pure comments without assignments; keep commented secrets.
    }

    // 1. Private key PEM headers.
    if trimmed.contains("-----BEGIN") && trimmed.contains("PRIVATE KEY-----") {
        out.push(Finding {
            rule: "private-key-pem".into(),
            line: line_no,
            confidence: 0.99,
            redacted: "-----BEGIN … PRIVATE KEY-----".into(),
        });
        return;
    }

    // 2. AWS access key id: AKIA/ASIA + 16 uppercase alnum.
    if let Some(m) = find_aws_key(trimmed) {
        out.push(Finding {
            rule: "aws-access-key-id".into(),
            line: line_no,
            confidence: 0.97,
            redacted: redact(&m),
        });
        return;
    }

    // 3. GitHub personal access token: ghp_ / gho_ / ghs_ + 36 alnum.
    if let Some(m) = find_prefixed(trimmed, &["ghp_", "gho_", "ghs_", "ghu_", "ghr_"], 36) {
        out.push(Finding {
            rule: "github-token".into(),
            line: line_no,
            confidence: 0.95,
            redacted: redact(&m),
        });
        return;
    }

    // 4. Slack token: xox[baprs]-...
    if let Some(m) = find_slack(trimmed) {
        out.push(Finding {
            rule: "slack-token".into(),
            line: line_no,
            confidence: 0.93,
            redacted: redact(&m),
        });
        return;
    }

    // 5. JWT: three base64url segments separated by dots, first starts eyJ.
    if let Some(m) = find_jwt(trimmed) {
        out.push(Finding {
            rule: "jwt".into(),
            line: line_no,
            confidence: 0.85,
            redacted: redact(&m),
        });
        return;
    }

    // 6. Generic secret assignment heuristic.
    if let Some((name, value)) = split_assignment(trimmed) {
        if looks_secret_name(&name) && high_entropy_value(&value) {
            out.push(Finding {
                rule: "generic-secret-assignment".into(),
                line: line_no,
                confidence: 0.80,
                redacted: format!("{} = {}", name.trim(), redact(&value)),
            });
        }
    }
}

fn find_aws_key(s: &str) -> Option<String> {
    for prefix in ["AKIA", "ASIA"] {
        if let Some(pos) = s.find(prefix) {
            let rest: String = s[pos..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if rest.len() == 20
                && rest[4..]
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            {
                return Some(rest);
            }
        }
    }
    None
}

fn find_prefixed(s: &str, prefixes: &[&str], token_len: usize) -> Option<String> {
    for p in prefixes {
        if let Some(pos) = s.find(p) {
            let tail: String = s[pos + p.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if tail.len() >= token_len {
                return Some(format!("{}{}", p, tail));
            }
        }
    }
    None
}

fn find_slack(s: &str) -> Option<String> {
    for p in ["xoxb-", "xoxa-", "xoxp-", "xoxr-", "xoxs-"] {
        if let Some(pos) = s.find(p) {
            let tail: String = s[pos..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if tail.len() >= 20 {
                return Some(tail);
            }
        }
    }
    None
}

fn find_jwt(s: &str) -> Option<String> {
    for token in s.split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '=') {
        if token.starts_with("eyJ") {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() == 3
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(is_b64url))
                && token.len() >= 40
            {
                return Some(token.to_string());
            }
        }
    }
    None
}

fn is_b64url(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Split `name = value` / `name: value` assignments (JSON, YAML, env, code).
fn split_assignment(s: &str) -> Option<(String, String)> {
    let sep = s.find('=').or_else(|| s.find(':'))?;
    let name = s[..sep]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string();
    let value_raw = s[sep + 1..].trim();
    // Strip surrounding quotes and trailing commas / semicolons.
    let value = value_raw
        .trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ';' || c == ' ')
        .to_string();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    Some((name, value))
}

fn looks_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    const NEEDLES: &[&str] = &[
        "secret",
        "password",
        "passwd",
        "token",
        "api_key",
        "apikey",
        "api-key",
        "access_key",
        "private_key",
        "client_secret",
        "auth",
        "credential",
    ];
    NEEDLES.iter().any(|n| lower.contains(n))
}

/// A value is "high entropy" if it is long enough and mixes character classes,
/// rejecting obvious placeholders like `changeme` or `your-token-here`.
pub fn high_entropy_value(v: &str) -> bool {
    if v.len() < 16 {
        return false;
    }
    let lower = v.to_ascii_lowercase();
    const PLACEHOLDERS: &[&str] = &[
        "changeme",
        "your",
        "example",
        "placeholder",
        "todo",
        "xxxx",
        "dummy",
        "redacted",
        "<",
        "${",
        "{{",
    ];
    if PLACEHOLDERS.iter().any(|p| lower.contains(p)) {
        return false;
    }
    let has_lower = v.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = v.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = v.chars().any(|c| c.is_ascii_digit());
    let classes = [has_lower, has_upper, has_digit]
        .iter()
        .filter(|b| **b)
        .count();
    let alnum = v.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    classes >= 2 && alnum >= 12
}

/// Redact a matched secret, keeping only a short prefix.
pub fn redact(secret: &str) -> String {
    let keep = secret.chars().take(4).collect::<String>();
    format!("{}…[redacted:{} chars]", keep, secret.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aws_key() {
        let f = scan("aws_key = AKIAIOSFODNN7EXAMPLE\n");
        assert!(f.iter().any(|x| x.rule == "aws-access-key-id"));
    }

    #[test]
    fn detects_private_key_header() {
        let f = scan("-----BEGIN RSA PRIVATE KEY-----\n");
        assert_eq!(f[0].rule, "private-key-pem");
        assert!(f[0].confidence >= QUARANTINE_CONFIDENCE);
    }

    #[test]
    fn detects_github_token() {
        let tok = format!("token: ghp_{}", "a".repeat(36));
        let f = scan(&tok);
        assert!(f.iter().any(|x| x.rule == "github-token"));
    }

    #[test]
    fn detects_jwt() {
        let jwt = "auth=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N";
        let f = scan(jwt);
        assert!(f.iter().any(|x| x.rule == "jwt"));
    }

    #[test]
    fn generic_assignment_flagged() {
        let f = scan("api_secret = \"9f8a7b6c5d4e3f2a1b0c9d8e\"\n");
        assert!(f.iter().any(|x| x.rule == "generic-secret-assignment"));
    }

    #[test]
    fn placeholder_not_flagged() {
        let f = scan("api_secret = \"your-secret-here-changeme\"\n");
        assert!(f.is_empty(), "placeholders must not be flagged: {:?}", f);
    }

    #[test]
    fn normal_code_is_clean() {
        let f = scan("let total = items.iter().map(|i| i.mass).sum();\n");
        assert!(f.is_empty());
    }

    #[test]
    fn redaction_hides_body() {
        let r = redact("AKIAIOSFODNN7EXAMPLE");
        assert!(r.starts_with("AKIA"));
        assert!(!r.contains("IOSFODNN"));
    }

    #[test]
    fn low_entropy_short_value_ignored() {
        assert!(!high_entropy_value("short"));
        assert!(!high_entropy_value("aaaaaaaaaaaaaaaa")); // one class only
    }
}
