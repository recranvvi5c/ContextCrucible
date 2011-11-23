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
