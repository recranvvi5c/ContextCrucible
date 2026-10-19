# Contributing

1. Open an issue before anything larger than a bug fix, so the shape is agreed first.
2. One behaviour per pull request, with the test that pins it.
3. Run the project's checks before opening the PR.

## Checks

- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --release`
