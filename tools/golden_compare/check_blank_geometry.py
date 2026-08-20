#!/usr/bin/env python3
"""Detect blank-output geometry regressions between a base and current RaTeX run."""

from __future__ import annotations

import argparse
from collections import defaultdict, deque
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

INK_THRESHOLD = 240


def read_formulas(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith(("%", "#"))
    ]


def image_geometry_if_blank(path: Path) -> tuple[int, int] | None:
    with Image.open(path) as image:
        rgb = np.array(image.convert("RGB"), dtype=np.uint8)
    if np.any(rgb < INK_THRESHOLD):
        return None
    height, width = rgb.shape[:2]
    return width, height


def size_regression(baseline: tuple[int, int], current: tuple[int, int]) -> float:
    def axis_regression(old: int, new: int) -> float:
        if old == new:
            return 0.0
        larger = max(old, new)
        if larger == 0:
            return 0.0
        return 1.0 - min(old, new) / larger

    return max(
        axis_regression(baseline[0], current[0]),
        axis_regression(baseline[1], current[1]),
    )


def compare_blank_geometry(
    baseline_cases: Path,
    baseline_output: Path,
    current_cases: Path,
    current_output: Path,
    max_size_regression: float,
) -> list[dict[str, Any]]:
    baseline_formulas = read_formulas(baseline_cases)
    current_formulas = read_formulas(current_cases)

    baseline_by_formula: dict[str, deque[tuple[int, Path]]] = defaultdict(deque)
    for index, formula in enumerate(baseline_formulas, start=1):
        baseline_by_formula[formula].append(
            (index, baseline_output / f"{index:04d}.png")
        )

    regressions: list[dict[str, Any]] = []
    for current_index, formula in enumerate(current_formulas, start=1):
        candidates = baseline_by_formula.get(formula)
        if not candidates:
            continue
        baseline_index, baseline_path = candidates.popleft()
        current_path = current_output / f"{current_index:04d}.png"
        if not baseline_path.exists() or not current_path.exists():
            continue

        baseline_size = image_geometry_if_blank(baseline_path)
        current_size = image_geometry_if_blank(current_path)
        if baseline_size is None or current_size is None:
            continue

        regression = size_regression(baseline_size, current_size)
        if regression > max_size_regression:
            regressions.append(
                {
                    "index": current_index,
                    "baseline_index": baseline_index,
                    "formula": formula,
                    "baseline_size": list(baseline_size),
                    "current_size": list(current_size),
                    "regression": regression,
                }
            )

    return regressions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-cases", type=Path, required=True)
    parser.add_argument("--baseline-output", type=Path, required=True)
    parser.add_argument("--current-cases", type=Path, required=True)
    parser.add_argument("--current-output", type=Path, required=True)
    parser.add_argument("--max-size-regression", type=float, default=0.05)
    args = parser.parse_args()

    regressions = compare_blank_geometry(
        args.baseline_cases,
        args.baseline_output,
        args.current_cases,
        args.current_output,
        args.max_size_regression,
    )
    if regressions:
        for item in regressions:
            print(
                "::error::blank-output geometry regression "
                f"at current {item['index']:04d} / base {item['baseline_index']:04d}: "
                f"{item['baseline_size']} -> {item['current_size']} "
                f"({item['regression']:.3f}) for {item['formula']!r}"
            )
        return 1

    print("Blank-output geometry gate passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
