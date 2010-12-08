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
