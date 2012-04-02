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
