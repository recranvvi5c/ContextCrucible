# Sample Repository

A tiny fixture repository consumed by contextcrucible's tests and examples.

## Layout

- `src/budget_solver.rs` — a budget allocation + knapsack solver (high relevance
  to a "budget solver" query).
- `src/strings.rs` — unrelated string helpers (low relevance).
- `config/secrets.yaml` — contains a fake credential to exercise quarantine.
- `web/bundle.min.js` — a generated artefact that the scanner rejects.
- `main.py` — a Python entrypoint that imports the budget solver conceptually.

The point of the fixture is to give the compiler a mix of relevant code,
irrelevant code, secrets, and generated files so decisions are observable.
