/**
 * Tests for the contextcrucible explorer, run with the built-in `node:test`
 * runner so no third-party test framework is required.
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  allocationByDirectory,
  allocationByLanguage,
  bar,
  includedFiles,
  parseManifest,
  renderReport,
  type Manifest,
} from "./explorer.js";

function sampleManifest(): Manifest {
  return {
    summary: {
      tool: "contextcrucible",
      version: "0.4.0",
      label: "test",
      query: "budget solver",
      budget_tokens: 2000,
      tokens_used: 1000,
      budget_utilization: 0.5,
      captured_value: 100.0,
      min_score: 0,
      solver_method: "dp",
      counts: {
        included: 3,
        excluded_secret: 1,
        excluded_budget: 0,
        excluded_low_score: 0,
        excluded_scan: 1,
      },
    },
    files: [
      {
        path: "src/budget.rs",
        decision: "include",
        tokens: 500,
        bytes: 1000,
        grade: 83.2,
        language: "rust",
        score_parts: { path: 16.7, query: 33.2, import: 20, symbol: 13.3 },
        fill_rank: 0,
        explanation: "included",
      },
      {
        path: "src/util.rs",
        decision: "include",
        tokens: 300,
        bytes: 600,
        grade: 20.0,
        language: "rust",
        score_parts: { path: 0, query: 20, import: 0, symbol: 0 },
        fill_rank: 1,
        explanation: "included",
      },
      {
        path: "main.py",
        decision: "include",
        tokens: 200,
        bytes: 400,
        grade: 12.0,
        language: "python",
        score_parts: { path: 0, query: 12, import: 0, symbol: 0 },
        fill_rank: 2,
        explanation: "included",
      },
      {
        path: "config/secrets.yaml",
        decision: "exclude:secret",
        tokens: 100,
        bytes: 200,
        grade: 0,
        language: "text",
        score_parts: { path: 0, query: 0, import: 0, symbol: 0 },
        secrets: [
          { rule: "aws-access-key-id", line: 8, confidence: 0.97, redacted: "AKIA…" },
        ],
        explanation: "quarantined",
      },
    ],
    scan_rejected: [
      { path: "web/bundle.min.js", decision: "exclude:scan", bytes: 174, scan_reason: "generated:suffix" },
    ],
  };
}

test("parseManifest accepts a well-formed manifest", () => {
  const json = JSON.stringify(sampleManifest());
  const m = parseManifest(json);
  assert.equal(m.summary.label, "test");
  assert.equal(m.files.length, 4);
});

test("parseManifest rejects invalid JSON", () => {
  assert.throws(() => parseManifest("{not json"), /not valid JSON/);
});

test("parseManifest rejects a manifest without files", () => {
  assert.throws(() => parseManifest('{"summary":{}}'), /files/);
});

test("includedFiles returns only includes in fill order", () => {
  const m = sampleManifest();
  const inc = includedFiles(m);
