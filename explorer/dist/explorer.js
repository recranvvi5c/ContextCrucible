/**
 * contextcrucible explorer — a dependency-free, static analyser for the JSON
 * manifests produced by the `crucible` CLI.
 *
 * The explorer does not run the compiler; it *reads a manifest* and renders the
 * pack allocation as a textual treemap / bar report. This keeps it fully
 * offline (no network, no bundler) while giving a quick visual read on where a
 * token budget went.
 */
/**
 * Parse and validate a manifest from a JSON string. Throws a descriptive
 * error if a required field is missing or mistyped.
 */
export function parseManifest(json) {
    let raw;
    try {
        raw = JSON.parse(json);
    }
    catch (err) {
        throw new Error(`manifest is not valid JSON: ${err.message}`);
    }
    if (typeof raw !== "object" || raw === null) {
        throw new Error("manifest must be a JSON object");
    }
    const obj = raw;
    if (typeof obj.summary !== "object" || obj.summary === null) {
        throw new Error("manifest is missing a 'summary' object");
    }
    if (!Array.isArray(obj.files)) {
        throw new Error("manifest is missing a 'files' array");
    }
    const summary = obj.summary;
    const files = obj.files;
    const scan_rejected = Array.isArray(obj.scan_rejected)
        ? obj.scan_rejected
        : [];
    return { summary, files, scan_rejected };
}
/** Files that were included in the pack, ordered by fill rank. */
export function includedFiles(manifest) {
    return manifest.files
        .filter((f) => f.decision === "include")
        .sort((a, b) => (a.fill_rank ?? 0) - (b.fill_rank ?? 0));
}
/**
 * Group included files by their first path segment (top-level directory, or
 * "<root>" for files at the repository root) and compute token shares.
 */
export function allocationByDirectory(manifest) {
    const included = includedFiles(manifest);
    const total = included.reduce((sum, f) => sum + f.tokens, 0);
    const groups = new Map();
    for (const f of included) {
        const slash = f.path.indexOf("/");
        const key = slash === -1 ? "<root>" : f.path.slice(0, slash);
        const cur = groups.get(key) ?? { tokens: 0, files: 0 };
        cur.tokens += f.tokens;
        cur.files += 1;
        groups.set(key, cur);
    }
    const buckets = [];
    for (const [key, v] of groups) {
        buckets.push({
            key,
            tokens: v.tokens,
            files: v.files,
