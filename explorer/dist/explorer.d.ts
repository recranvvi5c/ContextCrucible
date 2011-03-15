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
