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
