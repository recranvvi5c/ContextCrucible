# Budget arithmetic

The compiler tracks two numbers:

- `budget` - the hard ceiling from the manifest, in tokens.
- `spent` - the running total, accumulated per included file.

`src/budget.rs` owns the comparison. The invariant is `spent <= budget` at every step; a file that
would cross the ceiling is skipped with `reason: over-budget` and appears in the manifest's skipped
list. The estimate itself lives in `src/scan.rs`; the explorer mirrors its constants so a preview
never disagrees with the compiler.

If a future change needs a soft budget, add it as a second field - never relax the comparison.
