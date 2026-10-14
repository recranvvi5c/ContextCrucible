<!-- The Token Foundry :: contextcrucible -->

<p align="center">
  <img src="docs/assets/crucible-hero.svg" alt="contextcrucible pours molten tokens from a crucible into a budget-sized mold" width="100%" />
</p>

<h1 align="center">contextcrucible</h1>

<p align="center">
  <strong>A budget-aware context compiler for coding agents.</strong><br/>
  Scan the ore body. Reject the slag. Assay the token mass. Grade the relevance.<br/>
  Spark-test for secrets. Pour a hard-budget context pack — and stamp a manifest that explains every gram.
</p>

<p align="center">
  <code>rust 1.74+ · stdlib only</code> ·
  <code>typescript 5 explorer</code> ·
  <code>zero runtime dependencies</code> ·
  <code>deterministic output</code>
</p>

---

## Why a foundry?

Feeding a coding agent is metallurgy, not magic. You have a mountain of raw ore
— a repository — and a mold of fixed size — the model's context window. You
cannot pour the whole mountain in. You must **choose**: melt down the highest-
grade files, skim off the vendored slag and the minified dross, and *never* let
a molten credential splash into the pour.

`contextcrucible` is that foundry. It is a single, dependency-free Rust binary
(`crucible`) plus a small TypeScript explorer that visualises the result. Every
stage is transparent and every decision is written to an auditable manifest.
There are **no embeddings, no learned models, and no network calls** — every
point of relevance is traceable to a concrete token match you can read yourself.

<p align="center">
  <img src="docs/assets/budget-smelter.svg" alt="Animated bars showing a token budget poured into the highest-grade files first" width="90%" />
</p>

---

## The seven stages of the pour

```
   ore body                                                        context pack
  ┌────────┐   scan      tokens     score      secrets    budget   ┌──────────┐
  │  repo  │ ────────▶ ────────▶ ────────▶ ────────▶ ────────▶ ──▶ │  pack +  │
  └────────┘  reject   assay      grade      quarantine  pour      │ manifest │
              slag     mass       relevance  credentials           └──────────┘
```

1. **scan** — a deterministic, symlink-safe walk that rejects vendor
   directories (`node_modules`, `target`, `vendor`, …), generated artefacts
   (`*.min.js`, lockfiles, `*.pb.go`, …), binary files (by extension *and* by
   sampling the head for NUL bytes and non-text ratio), and anything over the
   size cap.
2. **tokens** — a transparent token estimator. It counts word segments (long
   identifiers fragment every ~4 chars, matching how BPE splits
   `snake_case_names`), charges one segment per punctuation mark, and collapses
   whitespace runs. It is a *heuristic*, deterministic and honest — it does not
   pretend to be any specific vendor tokenizer.
3. **score** — four additive signals, weighted to 100: **path** (25), **query
   frequency** (35, with diminishing returns), **import graph** (20), and
   **symbol definitions** (20). The breakdown is preserved so the manifest can
   explain each grade.
4. **secrets** — rule-based spark tests for AWS keys, PEM private keys, GitHub
   and Slack tokens, JWTs, and a generic *high-entropy assignment to a secret-
   looking name*. Placeholders like `changeme` or `${VAR}` are deliberately
   ignored. Any finding at or above 0.75 confidence quarantines the whole file.
5. **budget** — the pour itself: an exact 0/1 knapsack solved by dynamic
   programming over a quantised capacity grid, with a deterministic
   value-density greedy fallback for very large inputs. The hard token bound is
   enforced after reconstruction, always.
6. **pack** — the selected files, concatenated in fill order with clear
   delimiters, ready to paste into an agent.
7. **manifest** — a byte-stable JSON audit trail: every include and exclude,
   why, and at what token cost. See [`docs/PACK.md`](docs/PACK.md).

---

## Install &amp; build

The core needs only a Rust toolchain (1.74 or newer). The explorer needs Node 18+.

```sh
# build the CLI
cargo build --release        # binary at target/release/crucible

# build the explorer
cd explorer && npm install && npm run build && cd ..
```

Or lean on the `Makefile`:

```sh
make            # build + test the crate and the explorer
make demo       # compile a pack from the fixture and render it
make compare    # build two packs and weigh them
```

---

## Quick pour

Everything below is a **real transcript** captured from the bundled
`fixtures/sample-repo`.

### 1. Look at the ore — what does the scanner keep?

```text
$ crucible scan --path fixtures/sample-repo
kept 5 file(s):
  +      670b  README.md
  +      354b  config/secrets.yaml
  +      620b  main.py
  +     1139b  src/budget_solver.rs
  +      913b  src/strings.rs
rejected 1 file(s):
  - generated:suffix       web/bundle.min.js
```

The minified bundle is skimmed off as slag before it ever costs a token.

### 2. Pour a pack under a 1200-token budget

```text
$ crucible compile --path fixtures/sample-repo --budget 1200 \
      --query "budget solver knapsack" --label demo \
      --out examples/demo-pack.txt --manifest examples/demo-manifest.json
crucible: pack written to examples/demo-pack.txt
crucible: manifest written to examples/demo-manifest.json
crucible: poured 3 file(s), 1178 tokens / 1200 budget (98.2% util) via dp;
          excluded secret=1 budget=1 low-score=0 scan=1
```

98.2% of the mold filled, by an *exact* DP, with the fake-credential file
quarantined and the minified bundle never in contention.

### 3. Visualise the pour with the explorer

```text
$ node explorer/dist/cli.js examples/demo-manifest.json
╔══════════════════════════════════════════════════════════════╗
║ contextcrucible pack :: demo                                  ║
╚══════════════════════════════════════════════════════════════╝
  query          : budget solver knapsack
  solver         : dp
  budget         : 1178 / 1200 tokens  (98.2% utilised)
  captured value : 125.3
  decisions      : 3 in · 1 secret · 1 budget · 0 low-score · 1 scan

  Allocation by directory
  ────────────────────────────────────────────────────────────
  <root>         ████████████████··············    632t  53.7% (2)
  src            ██████████████················    546t  46.3% (1)

  Allocation by language
  ────────────────────────────────────────────────────────────
  rust           ██████████████················    546t  46.3% (1)
  python         ████████······················    321t  27.2% (1)
  text           ████████······················    311t  26.4% (1)

  Included files (fill order)
  ────────────────────────────────────────────────────────────
  # 0 src/budget_solver.rs                  546t  grade  83.3
  # 1 README.md                             311t  grade  30.3
  # 2 main.py                               321t  grade  11.7

  Quarantined (secrets)
  ────────────────────────────────────────────────────────────
  ! config/secrets.yaml                aws-access-key-id
```

`src/budget_solver.rs` grades **83.3** — its path, body, imports, and symbol
names all resonate with the query "budget solver knapsack", so it is poured
first at fill rank 0.

### 4. Weigh two pours against each other

```text
$ crucible compile --path fixtures/sample-repo --budget 600 \
      --query "budget solver knapsack" --label tight \
      --manifest examples/tight-manifest.json --out examples/tight-pack.txt
crucible: poured 1 file(s), 546 tokens / 600 budget (91.0% util) via dp;
          excluded secret=1 budget=3 low-score=0 scan=1

$ crucible compare --a examples/tight-manifest.json --b examples/demo-manifest.json
crucible compare :: tight → demo
  tokens : 546 → 1178 (+632)
  value  : 83.2 → 125.2 (+42.0)
  util   : 91.0% → 98.2%
  added (2):
    + README.md
    + main.py
  removed (0):
  retained: 1 file(s)
```

Loosening the budget from 600 to 1200 tokens *adds* two files and captures 42
more grade-points — and crucially **removes nothing** that still fit. That
monotonicity is asserted in the integration tests.

---

## A slice of the manifest

Every grade is explained. Here is the entry for the winning file, verbatim:

```json
{
  "path": "src/budget_solver.rs",
  "decision": "include",
  "tokens": 546,
  "bytes": 1139,
  "grade": 83.25,
  "language": "rust",
  "score_parts": {
    "path": 16.6667,
    "query": 33.2500,
    "import": 20.0000,
    "symbol": 13.3333
  },
  "fill_rank": 0,
  "explanation": "included at fill rank 0 — grade 83.2 (path 16.7/query 33.2/import 20.0/symbol 13.3) for 546 tokens"
}
```

And the quarantined file — note the secret is **redacted**, never echoed:

```json
{
  "path": "config/secrets.yaml",
  "decision": "exclude:secret",
  "secrets": [
    { "rule": "aws-access-key-id", "line": 8, "confidence": 0.97, "redacted": "AKIA…[redacted:20 chars]" }
  ],
  "explanation": "quarantined — 1 secret finding(s), highest confidence 0.97"
}
```

---

## Command reference

```text
crucible compile   Pour a context pack from a repository
crucible scan      Report kept / rejected files from a scan
crucible compare   Weigh two manifests against each other
crucible help      Show usage
```

### `compile` flags

| Flag           | Default    | Meaning                                      |
|----------------|------------|----------------------------------------------|
| `--path`       | `.`        | Repository root to scan.                     |
| `--budget`     | `8000`     | Hard token budget — never exceeded.          |
| `--query`      | *(none)*   | Free-text relevance query.                   |
| `--min-score`  | `0`        | Drop files graded below this floor.          |
| `--max-bytes`  | `524288`   | Skip files larger than this.                 |
| `--label`      | `pour`     | Label recorded in the manifest.              |
| `--out`        | *stdout*   | Where to write the pack content.             |
| `--manifest`   | *(none)*   | Where to write the JSON manifest.            |

With no query, `compile` grades on structural centrality (shallow, source-like
files first) so the budget still fills sensibly.

---

## How the budget solver decides

The pour is a classic 0/1 knapsack: maximise captured relevance subject to a
hard token bound. `contextcrucible` solves it **exactly** with dynamic
programming whenever the problem fits a bounded grid (items × quantised
budget buckets ≤ 4M cells), and falls back to a deterministic value-density
greedy only for very large repositories. Both paths:

- break ties by `(−score, tokens, path)` so runs are reproducible;
- drop any item that alone exceeds the budget;
- re-enforce the *true* token bound after reconstruction, trimming
  lowest-value items if quantisation rounding nudged the pour over the line.

The classic knapsack trap — greedily grabbing one big high-value item and
missing a better pair — is covered by a unit test
(`dp_beats_naive_greedy_on_classic_case`): with a budget of 100 the solver
chooses two items summing to value 101 over a single item of value 100.

---

## The token assay, honestly

Token counting here is a **heuristic**, and the code says so. It does not ship a
vendor merge table and it does not claim to reproduce any specific tokenizer.
What it gives you is *stable, explainable, monotone* estimates that are more
than good enough to rank files and fill a budget — and identical on every run,
every platform. If you need exact counts for a specific model, feed the emitted
pack to that model's own tokenizer; the pack is plain text designed for exactly
that hand-off.

---

## Project layout

```
contextcrucible/
├── Cargo.toml               # crate manifest (bin: crucible, lib: contextcrucible)
├── Makefile                 # build/test/demo/compare orchestration
├── src/
│   ├── lib.rs               # pipeline types: Candidate, Decision
│   ├── main.rs              # the `crucible` CLI (hand-rolled arg parsing)
│   ├── scan.rs              # ore extraction / slag rejection
│   ├── tokens.rs            # token assay
│   ├── score.rs             # four-signal relevance grading
│   ├── secrets.rs           # credential spark tests + redaction
│   ├── budget.rs            # exact DP knapsack + greedy fallback
│   ├── pack.rs              # pipeline orchestration + manifest rendering
│   ├── compare.rs           # weigh two packs
│   └── json.rs              # dependency-free JSON writer
├── tests/pipeline.rs        # end-to-end tests over the fixture repo
├── fixtures/sample-repo/    # a mixed repo: relevant code, secrets, generated
├── examples/                # reproducible packs + manifests
├── explorer/                # TypeScript static explorer (node:test suite)
│   └── src/{explorer,cli,explorer.test}.ts
├── docs/
│   ├── PACK.md              # pack + manifest format spec
│   └── assets/*.svg         # two animated, fully local diagrams
├── LICENSE · CHANGELOG.md
└── .github/workflows/ci.yml # Rust matrix + explorer + determinism smoke test
```

---

## Testing

```sh
cargo test                       # 65 Rust unit + integration tests
cd explorer && npm test          # 9 explorer tests via node:test
```

The Rust suite covers token monotonicity, binary detection, every secret rule,
each scoring signal, knapsack optimality and hard-bound enforcement, manifest
shape, and full-pipeline determinism against the fixture. CI additionally runs a
**determinism smoke test**: it compiles the same pack twice and `diff`s both the
pack and the manifest to guarantee byte-for-byte reproducibility.

---

## Design principles

- **Explainable over clever.** Every grade decomposes into signals you can read.
- **Deterministic over fast.** Same input, same pour, same bytes — always.
- **Safe by default.** A suspected secret excludes the file; nothing molten
  splashes into the pour, and nothing secret is ever written to the manifest.
- **Honest about limits.** Heuristic token counts and rule-based relevance, with
  no dressed-up claims of semantic understanding.
- **Dependency-free.** The core is pure Rust stdlib; the explorer's only dev
  dependency is TypeScript itself.

---

## License

[MIT](LICENSE) — melt it down and recast it however you like.
