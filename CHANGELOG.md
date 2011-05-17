# Changelog

All notable changes to contextcrucible are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-08-31

### Added
- `crucible compare` subcommand: weighs two manifests, reporting added /
  removed / retained files plus token and value deltas, in text or `--json`.
- TypeScript static explorer (`explorer/`) that renders pack allocation by
  directory and language from a manifest, with a `node:test` suite.
- Two animated, fully local SVGs under `docs/assets/`.
- `docs/PACK.md` specifying the pack and manifest formats.

### Changed
- Budget solver now selects between an exact quantised DP and a deterministic
  value-density greedy fallback based on the problem size, and always enforces
  the hard token bound after reconstruction.
- Manifest gained per-signal `score_parts` and a human-readable `explanation`
  for every candidate.

### Fixed
- Ceil-rounding in the DP could momentarily overshoot the budget during
