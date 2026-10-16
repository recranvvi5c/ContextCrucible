# Examples

Worked examples produced from `fixtures/sample-repo`. Regenerate them with
`make demo` and `make compare`, or with the commands below.

## Files

| File                   | Produced by                                             |
|------------------------|---------------------------------------------------------|
| `demo-pack.txt`        | `crucible compile --budget 1200`                        |
| `demo-manifest.json`   | the manifest for the demo pack                          |
| `tight-pack.txt`       | `crucible compile --budget 600`                         |
| `tight-manifest.json`  | the manifest for the tight pack                         |

## Reproduce

Compile the demo pack (from the project root):

```sh
cargo run -- compile \
  --path fixtures/sample-repo \
  --budget 1200 \
  --query "budget solver knapsack" \
  --label demo \
  --out examples/demo-pack.txt \
  --manifest examples/demo-manifest.json
```

Visualise it with the explorer:

```sh
cd explorer && npm install && npm run build && cd ..
node explorer/dist/cli.js examples/demo-manifest.json
```

Compile a tighter pack and compare:

```sh
cargo run -- compile --path fixtures/sample-repo --budget 600 \
  --query "budget solver knapsack" --label tight \
  --out examples/tight-pack.txt --manifest examples/tight-manifest.json

cargo run -- compare --a examples/tight-manifest.json --b examples/demo-manifest.json
```

## What to look for

- `src/budget_solver.rs` grades highest (all four signals fire) and is poured
  first at fill rank 0.
- `config/secrets.yaml` is **quarantined** — it holds a fake AWS key.
- `web/bundle.min.js` never becomes a candidate: it is rejected during scanning
  as `generated:suffix`.
- Raising the budget from 600 to 1200 *adds* `README.md` and `main.py` without
  ever removing a file that still fits.
