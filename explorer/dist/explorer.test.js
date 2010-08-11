/**
 * Tests for the contextcrucible explorer, run with the built-in `node:test`
 * runner so no third-party test framework is required.
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { allocationByDirectory, allocationByLanguage, bar, includedFiles, parseManifest, renderReport, } from "./explorer.js";
function sampleManifest() {
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
