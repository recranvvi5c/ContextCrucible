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
