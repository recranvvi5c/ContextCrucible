# The Pack &amp; Manifest Format

A `crucible compile` run produces two artefacts: a **pack** (the actual context
to feed an agent) and a **manifest** (a JSON audit trail of every decision).
This document specifies both so downstream tools — like the TypeScript explorer
in `explorer/` — can consume them reliably.

## The pack

The pack is a UTF-8 text file. Its first lines are a comment header, followed by
each included file wrapped in `BEGIN`/`END` delimiters:

```
# contextcrucible pack :: demo :: budget=1200 tokens :: used=1178 :: method=dp
# query: budget solver knapsack
# files: 3

===== BEGIN src/budget_solver.rs (546 tokens, grade 83.3, rust) =====
// ... file content ...
===== END src/budget_solver.rs =====

===== BEGIN README.md (311 tokens, grade 30.3, text) =====
# Sample Repository
...
===== END README.md =====
```

Files appear in **fill order** — the order the budget solver committed them,
highest grade first. Every included file is guaranteed to end with a newline
inside its block so concatenation never fuses two files.

## The manifest

The manifest is deterministic, pretty-printed JSON with three top-level keys.

### `summary`

| Field                | Type   | Meaning                                            |
|----------------------|--------|----------------------------------------------------|
| `tool`               | string | Always `"contextcrucible"`.                        |
| `version`            | string | Crate version that produced the pack.              |
| `label`              | string | The `--label` given at compile time.               |
| `query`              | string | The relevance query (may be empty).                |
| `budget_tokens`      | int    | The hard token budget.                             |
| `tokens_used`        | int    | Tokens actually consumed (always `<= budget`).     |
| `budget_utilization` | float  | `tokens_used / budget_tokens`, `0..=1`.            |
| `captured_value`     | float  | Sum of grades of included files.                   |
| `min_score`          | float  | The relevance floor applied.                       |
| `solver_method`      | string | `"dp"`, `"greedy"`, or `"empty"`.                  |
| `counts`             | object | Per-disposition file counts (see below).           |

`counts` has: `included`, `excluded_secret`, `excluded_budget`,
`excluded_low_score`, `excluded_scan`.

### `files`

An array, one object per **evaluated candidate** (files that survived scanning).
Each entry explains its fate:

```json
{
  "path": "src/budget_solver.rs",
  "decision": "include",
  "tokens": 546,
  "bytes": 1139,
  "grade": 83.25,
  "language": "rust",
  "score_parts": { "path": 16.67, "query": 33.25, "import": 20.0, "symbol": 13.33 },
  "fill_rank": 0,
  "explanation": "included at fill rank 0 — grade 83.2 ..."
}
```

`decision` is one of:

| Code                | Reason                                                  |
|---------------------|---------------------------------------------------------|
| `include`           | Selected by the budget solver.                          |
| `exclude:secret`    | Quarantined — a likely credential was detected.         |
| `exclude:budget`    | Lost the budget contest.                                |
| `exclude:low-score` | Fell below `min_score`.                                 |
| `exclude:scan`      | Rejected during scanning (also listed in `scan_rejected`). |

`score_parts` always sums (clamped) to `grade`. When a file carries secrets a
`secrets` array is present with `rule`, `line`, `confidence`, and a **redacted**
excerpt — the raw secret is never written to the manifest.

### `scan_rejected`

Files rejected before scoring (binary, vendor, generated, oversized). Each has
`path`, `bytes`, and a `scan_reason` such as `binary:extension`,
`generated:suffix`, or `too-large:<n>b`.

## Determinism guarantees

Given identical inputs and budget, the tool guarantees:

1. The **set** of included files is identical.
2. Their **fill order** is identical.
3. `tokens_used` and `captured_value` are identical.
4. The manifest is **byte-for-byte** identical (fixed float precision, ordered
   keys, path-sorted arrays).

This makes packs safe to check into CI and diff across code changes.

## Consuming a manifest

The explorer parses the manifest and renders allocation by directory and by
language. See `explorer/src/explorer.ts` for the typed interface
(`Manifest`, `ManifestSummary`, `ManifestFile`).
