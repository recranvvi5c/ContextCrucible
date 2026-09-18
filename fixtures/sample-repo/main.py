#!/usr/bin/env python3
"""Entrypoint that drives a budget allocation for the sample repository."""

from typing import Dict


def allocate(total: int, weights: Dict[str, float]) -> Dict[str, int]:
    """Distribute a total budget across weighted tasks."""
    denom = sum(weights.values())
    if denom <= 0:
        return {}
    return {name: int(total * (w / denom)) for name, w in weights.items()}


def main() -> None:
    plan = allocate(1000, {"scan": 2.0, "score": 3.0, "budget": 5.0})
    for task, share in sorted(plan.items()):
        print(f"{task}: {share} tokens")


if __name__ == "__main__":
    main()
