/**
 * contextcrucible explorer — a dependency-free, static analyser for the JSON
 * manifests produced by the `crucible` CLI.
 *
 * The explorer does not run the compiler; it *reads a manifest* and renders the
 * pack allocation as a textual treemap / bar report. This keeps it fully
 * offline (no network, no bundler) while giving a quick visual read on where a
 * token budget went.
 */

/** A single file decision as recorded in a manifest. */
export interface ManifestFile {
  path: string;
  decision: string;
  tokens: number;
  bytes: number;
  grade: number;
  language: string;
  score_parts: {
    path: number;
    query: number;
    import: number;
    symbol: number;
  };
  fill_rank?: number;
  secrets?: Array<{
    rule: string;
    line: number;
    confidence: number;
    redacted: string;
  }>;
  scan_reason?: string;
  explanation: string;
}

/** The summary block of a manifest. */
export interface ManifestSummary {
  tool: string;
  version: string;
  label: string;
  query: string;
  budget_tokens: number;
  tokens_used: number;
  budget_utilization: number;
  captured_value: number;
  min_score: number;
  solver_method: string;
  counts: {
    included: number;
    excluded_secret: number;
    excluded_budget: number;
    excluded_low_score: number;
    excluded_scan: number;
  };
}

/** A parsed manifest. */
export interface Manifest {
  summary: ManifestSummary;
  files: ManifestFile[];
  scan_rejected: Array<{
    path: string;
    decision: string;
    bytes: number;
    scan_reason: string;
  }>;
}

/**
 * Parse and validate a manifest from a JSON string. Throws a descriptive
 * error if a required field is missing or mistyped.
 */
export function parseManifest(json: string): Manifest {
  let raw: unknown;
  try {
    raw = JSON.parse(json);
  } catch (err) {
    throw new Error(`manifest is not valid JSON: ${(err as Error).message}`);
  }
  if (typeof raw !== "object" || raw === null) {
    throw new Error("manifest must be a JSON object");
  }
  const obj = raw as Record<string, unknown>;
  if (typeof obj.summary !== "object" || obj.summary === null) {
    throw new Error("manifest is missing a 'summary' object");
  }
  if (!Array.isArray(obj.files)) {
    throw new Error("manifest is missing a 'files' array");
  }
  const summary = obj.summary as ManifestSummary;
  const files = obj.files as ManifestFile[];
  const scan_rejected = Array.isArray(obj.scan_rejected)
    ? (obj.scan_rejected as Manifest["scan_rejected"])
    : [];
  return { summary, files, scan_rejected };
}

/** Files that were included in the pack, ordered by fill rank. */
export function includedFiles(manifest: Manifest): ManifestFile[] {
  return manifest.files
    .filter((f) => f.decision === "include")
    .sort((a, b) => (a.fill_rank ?? 0) - (b.fill_rank ?? 0));
}

/** Aggregate token allocation grouped by top-level directory. */
export interface AllocationBucket {
  key: string;
  tokens: number;
  files: number;
  share: number;
}

/**
 * Group included files by their first path segment (top-level directory, or
 * "<root>" for files at the repository root) and compute token shares.
 */
export function allocationByDirectory(manifest: Manifest): AllocationBucket[] {
  const included = includedFiles(manifest);
  const total = included.reduce((sum, f) => sum + f.tokens, 0);
  const groups = new Map<string, { tokens: number; files: number }>();
  for (const f of included) {
    const slash = f.path.indexOf("/");
    const key = slash === -1 ? "<root>" : f.path.slice(0, slash);
    const cur = groups.get(key) ?? { tokens: 0, files: 0 };
    cur.tokens += f.tokens;
    cur.files += 1;
    groups.set(key, cur);
  }
  const buckets: AllocationBucket[] = [];
  for (const [key, v] of groups) {
    buckets.push({
      key,
      tokens: v.tokens,
      files: v.files,
      share: total === 0 ? 0 : v.tokens / total,
    });
  }
  // Deterministic ordering: largest allocation first, ties broken by name.
  buckets.sort((a, b) => b.tokens - a.tokens || a.key.localeCompare(b.key));
  return buckets;
}

/** Aggregate token allocation grouped by detected language. */
export function allocationByLanguage(manifest: Manifest): AllocationBucket[] {
  const included = includedFiles(manifest);
  const total = included.reduce((sum, f) => sum + f.tokens, 0);
  const groups = new Map<string, { tokens: number; files: number }>();
  for (const f of included) {
    const cur = groups.get(f.language) ?? { tokens: 0, files: 0 };
    cur.tokens += f.tokens;
    cur.files += 1;
    groups.set(f.language, cur);
  }
  const buckets: AllocationBucket[] = [];
  for (const [key, v] of groups) {
    buckets.push({
      key,
      tokens: v.tokens,
      files: v.files,
      share: total === 0 ? 0 : v.tokens / total,
    });
  }
  buckets.sort((a, b) => b.tokens - a.tokens || a.key.localeCompare(b.key));
  return buckets;
}

/** Render a fixed-width horizontal bar for a fractional value in [0,1]. */
export function bar(fraction: number, width = 30): string {
  const clamped = Math.max(0, Math.min(1, fraction));
  const filled = Math.round(clamped * width);
  return "█".repeat(filled) + "·".repeat(width - filled);
}

/** Right-pad a string to a given width (deterministic, no locale effects). */
function pad(s: string, width: number): string {
  return s.length >= width ? s : s + " ".repeat(width - s.length);
}

/** Left-pad a number to a given width. */
function lpad(n: number | string, width: number): string {
  const s = String(n);
  return s.length >= width ? s : " ".repeat(width - s.length) + s;
}

/**
 * Render a full textual report of a pack allocation, suitable for a terminal.
 * Pure function of the manifest — identical input yields identical output.
 */
export function renderReport(manifest: Manifest): string {
  const s = manifest.summary;
  const lines: string[] = [];
  lines.push("╔══════════════════════════════════════════════════════════════╗");
  lines.push(`║ contextcrucible pack :: ${pad(s.label, 38)}║`);
  lines.push("╚══════════════════════════════════════════════════════════════╝");
  lines.push(`  query          : ${s.query || "(none)"}`);
  lines.push(`  solver         : ${s.solver_method}`);
  lines.push(
    `  budget         : ${s.tokens_used} / ${s.budget_tokens} tokens  ` +
      `(${(s.budget_utilization * 100).toFixed(1)}% utilised)`,
  );
  lines.push(`  captured value : ${s.captured_value.toFixed(1)}`);
  lines.push(
    `  decisions      : ${s.counts.included} in · ` +
      `${s.counts.excluded_secret} secret · ` +
      `${s.counts.excluded_budget} budget · ` +
      `${s.counts.excluded_low_score} low-score · ` +
      `${s.counts.excluded_scan} scan`,
  );
  lines.push("");

  lines.push("  Allocation by directory");
  lines.push("  " + "─".repeat(60));
  for (const b of allocationByDirectory(manifest)) {
    lines.push(
      `  ${pad(b.key, 14)} ${bar(b.share)} ${lpad(b.tokens, 6)}t ` +
        `${lpad((b.share * 100).toFixed(1) + "%", 6)} (${b.files})`,
    );
  }
  lines.push("");

  lines.push("  Allocation by language");
  lines.push("  " + "─".repeat(60));
  for (const b of allocationByLanguage(manifest)) {
    lines.push(
      `  ${pad(b.key, 14)} ${bar(b.share)} ${lpad(b.tokens, 6)}t ` +
        `${lpad((b.share * 100).toFixed(1) + "%", 6)} (${b.files})`,
    );
  }
  lines.push("");

  lines.push("  Included files (fill order)");
  lines.push("  " + "─".repeat(60));
  for (const f of includedFiles(manifest)) {
    lines.push(
      `  #${lpad(f.fill_rank ?? 0, 2)} ${pad(f.path, 34)} ` +
        `${lpad(f.tokens, 6)}t  grade ${lpad(f.grade.toFixed(1), 5)}`,
    );
  }

  const secretFiles = manifest.files.filter((f) => f.decision === "exclude:secret");
  if (secretFiles.length > 0) {
    lines.push("");
    lines.push("  Quarantined (secrets)");
    lines.push("  " + "─".repeat(60));
    for (const f of secretFiles) {
      const rule = f.secrets && f.secrets[0] ? f.secrets[0].rule : "unknown";
      lines.push(`  ! ${pad(f.path, 34)} ${rule}`);
    }
  }

  lines.push("");
  return lines.join("\n");
}
