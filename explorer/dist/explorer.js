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
