#!/usr/bin/env node
/**
 * crucible-explorer — CLI wrapper around the static explorer.
 *
 * Usage:
 *   crucible-explorer <manifest.json>
 *   crucible-explorer --json <manifest.json>   # emit machine-readable buckets
 */

import { readFileSync } from "node:fs";
import {
  allocationByDirectory,
  allocationByLanguage,
  parseManifest,
  renderReport,
} from "./explorer.js";

function main(argv: string[]): number {
  const args = argv.slice(2);
  if (args.length === 0 || args[0] === "-h" || args[0] === "--help") {
    process.stdout.write(
      "crucible-explorer — visualise a contextcrucible manifest\n\n" +
        "USAGE:\n" +
        "  crucible-explorer <manifest.json>\n" +
        "  crucible-explorer --json <manifest.json>\n",
    );
    return args.length === 0 ? 1 : 0;
  }

  let asJson = false;
  let path: string | undefined;
  for (const a of args) {
    if (a === "--json") {
      asJson = true;
    } else {
      path = a;
    }
  }
  if (!path) {
    process.stderr.write("crucible-explorer: missing manifest path\n");
    return 1;
  }

  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (err) {
    process.stderr.write(
      `crucible-explorer: cannot read ${path}: ${(err as Error).message}\n`,
    );
    return 1;
  }

  let manifest;
  try {
    manifest = parseManifest(text);
  } catch (err) {
    process.stderr.write(`crucible-explorer: ${(err as Error).message}\n`);
    return 1;
  }

  if (asJson) {
    const payload = {
      label: manifest.summary.label,
      byDirectory: allocationByDirectory(manifest),
      byLanguage: allocationByLanguage(manifest),
    };
    process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
  } else {
    process.stdout.write(renderReport(manifest) + "\n");
  }
  return 0;
}

process.exit(main(process.argv));
