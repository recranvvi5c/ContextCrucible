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
export declare function parseManifest(json: string): Manifest;
/** Files that were included in the pack, ordered by fill rank. */
export declare function includedFiles(manifest: Manifest): ManifestFile[];
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
export declare function allocationByDirectory(manifest: Manifest): AllocationBucket[];
/** Aggregate token allocation grouped by detected language. */
export declare function allocationByLanguage(manifest: Manifest): AllocationBucket[];
/** Render a fixed-width horizontal bar for a fractional value in [0,1]. */
export declare function bar(fraction: number, width?: number): string;
/**
 * Render a full textual report of a pack allocation, suitable for a terminal.
 * Pure function of the manifest — identical input yields identical output.
 */
export declare function renderReport(manifest: Manifest): string;
