#!/usr/bin/env python3
"""Authoritative RaTeX golden scorer and report generator.

The Python implementation is the only source of official golden scores.  It
always emits one case record per formula, including missing or failed renders,
so an intersection of fixture/output filenames can never silently shrink the
suite.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import platform
import re
import subprocess
import sys
import time
from collections import Counter, defaultdict, deque
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

from corpus import read_formulas

try:
    import numpy as np
    from PIL import Image, ImageFilter
except ImportError:
    print(
        "Install: python3 -m pip install -r tools/golden_compare/requirements.txt",
        file=sys.stderr,
    )
    sys.exit(1)


REPORT_VERSION = 1
METRIC_VERSION = "ratex-ink-v2"
INK_THRESHOLD = 240
NORM_HEIGHT = 120
DEFAULT_PASS_THRESHOLD = 0.30
ALLOWED_POLICY_STATUSES = {
    "unsupported",
    "parse_error",
    "layout_error",
    "render_error",
}
ERROR_STATUSES = {
    "missing_fixture",
    "missing_output",
    "parse_error",
    "layout_error",
    "render_error",
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def suite_hash_for_formulas(formulas: list[str]) -> str:
    canonical = "".join(
        f"{index:04d}\0{formula}\n"
        for index, formula in enumerate(formulas, start=1)
    )
    return sha256_text(canonical)


def command_output(args: list[str], cwd: Path | None = None) -> str | None:
    try:
        completed = subprocess.run(
            args,
            cwd=cwd,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=10,
        )
    except (FileNotFoundError, subprocess.SubprocessError):
        return None
    value = completed.stdout.strip()
    return value or None


def load_package_lock_version(lock_path: Path, package_name: str) -> str | None:
    if not lock_path.exists():
        return None
    try:
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        return lock.get("packages", {}).get(f"node_modules/{package_name}", {}).get("version")
    except (OSError, json.JSONDecodeError):
        return None


def collect_puppeteer_metadata(tool_dir: Path) -> dict[str, Any]:
    lock_path = tool_dir / "package-lock.json"
    metadata: dict[str, Any] = {
        "puppeteer_version": load_package_lock_version(lock_path, "puppeteer"),
        "chromium_revision": None,
        "chromium_version": None,
    }
    if not (tool_dir / "node_modules" / "puppeteer").exists():
        return metadata

    source = """
import { PUPPETEER_REVISIONS } from "puppeteer-core/internal/revisions.js";
import packageJson from "puppeteer/package.json" with { type: "json" };
console.log(JSON.stringify({
  puppeteer_version: packageJson.version,
  chromium_revision: PUPPETEER_REVISIONS.chrome,
}));
"""
    raw = command_output(["node", "--input-type=module", "-e", source], cwd=tool_dir)
    if raw:
        try:
            metadata.update(json.loads(raw.splitlines()[-1]))
        except json.JSONDecodeError:
            pass
    return metadata


def collect_os_metadata() -> dict[str, Any]:
    value: dict[str, Any] = {
        "platform": platform.platform(),
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "runner_os": os.environ.get("RUNNER_OS"),
        "image_os": os.environ.get("ImageOS"),
        "image_version": os.environ.get("ImageVersion"),
    }
    os_release = Path("/etc/os-release")
    if os_release.exists():
        parsed: dict[str, str] = {}
        for line in os_release.read_text(encoding="utf-8").splitlines():
            if "=" in line:
                key, raw = line.split("=", 1)
                parsed[key] = raw.strip().strip('"')
        value["os_image"] = parsed.get("PRETTY_NAME") or parsed.get("NAME")
    elif platform.system() == "Darwin":
        product = command_output(["sw_vers", "-productVersion"])
        build = command_output(["sw_vers", "-buildVersion"])
        value["os_image"] = f"macOS {product} ({build})" if product else "macOS"
    else:
        value["os_image"] = platform.platform()
    return value


def font_hashes(repo_root: Path, tool_dir: Path) -> dict[str, dict[str, str]]:
    candidates = {
        "ratex": [
            repo_root / "fonts",
            repo_root / "crates" / "ratex-katex-fonts" / "fonts",
        ],
        "katex_reference": [tool_dir / "node_modules" / "katex" / "dist" / "fonts"],
    }
    result: dict[str, dict[str, str]] = {}
    for label, directories in candidates.items():
        directory = next((item for item in directories if item.is_dir()), None)
        if directory is None:
            result[label] = {}
            continue
        result[label] = {
            path.name: sha256_file(path)
            for path in sorted(directory.iterdir())
            if path.is_file() and path.suffix.lower() in {".ttf", ".woff", ".woff2"}
        }
    return result


def collect_environment(repo_root: Path, tool_dir: Path) -> dict[str, Any]:
    rustc = command_output(["rustc", "-Vv"])
    cargo = command_output(["cargo", "-V"])
    return {
        "rust_toolchain": rustc,
        "cargo_version": cargo,
        "python_version": platform.python_version(),
        "pillow_version": getattr(sys.modules.get("PIL"), "__version__", None),
        "numpy_version": np.__version__,
        "node_version": command_output(["node", "--version"]),
        "os": collect_os_metadata(),
        **collect_puppeteer_metadata(tool_dir),
        "font_file_hashes": font_hashes(repo_root, tool_dir),
    }


def load_image(path: Path) -> np.ndarray:
    with Image.open(path) as image:
        return np.array(image.convert("RGB"), dtype=np.uint8)


def get_ink_mask(img: np.ndarray) -> np.ndarray:
    """Return a boolean mask where True means a non-white (ink) pixel."""
    return np.any(img < INK_THRESHOLD, axis=2)


def crop_to_content(img: np.ndarray, margin: int = 2) -> np.ndarray:
    mask = get_ink_mask(img)
    if not np.any(mask):
        return img[:10, :10]
    rows = np.any(mask, axis=1)
    cols = np.any(mask, axis=0)
    rmin, rmax = np.where(rows)[0][[0, -1]]
    cmin, cmax = np.where(cols)[0][[0, -1]]
    rmin = max(0, int(rmin) - margin)
    rmax = min(img.shape[0] - 1, int(rmax) + margin)
    cmin = max(0, int(cmin) - margin)
    cmax = min(img.shape[1] - 1, int(cmax) + margin)
    return img[rmin : rmax + 1, cmin : cmax + 1]


def normalize_size(img: np.ndarray, target_h: int = NORM_HEIGHT) -> np.ndarray:
    h, w = img.shape[:2]
    if h == 0 or w == 0:
        return np.full((target_h, target_h, 3), 255, dtype=np.uint8)
    scale = target_h / h
    new_w = max(1, int(w * scale))
    resized = Image.fromarray(img).resize(
        (new_w, target_h), Image.Resampling.LANCZOS
    )
    return np.array(resized, dtype=np.uint8)


def _best_2d_alignment(
    ref_ink: np.ndarray,
    test_ink: np.ndarray,
    max_vshift: int,
    max_hshift: int,
) -> tuple[int, int, int, int, int, int, np.ndarray]:
    """Align test ink to reference ink and return metrics plus best dy/dx."""
    height, width = ref_ink.shape
    best_intersection = -1
    best_dy = 0
    best_dx = 0
    ref_count = int(np.sum(ref_ink))

    for dy in range(-max_vshift, max_vshift + 1):
        if dy > 0:
            r_y0, r_y1 = dy, height
            t_y0, t_y1 = 0, height - dy
        elif dy < 0:
            r_y0, r_y1 = 0, height + dy
            t_y0, t_y1 = -dy, height
        else:
            r_y0, r_y1 = 0, height
            t_y0, t_y1 = 0, height

        ref_strip = ref_ink[r_y0:r_y1, :]
        test_strip = test_ink[t_y0:t_y1, :]
        for dx in range(-max_hshift, max_hshift + 1):
            if dx > 0:
                intersection = int(
                    np.sum(ref_strip[:, dx:] & test_strip[:, : width - dx])
                )
            elif dx < 0:
                intersection = int(
                    np.sum(ref_strip[:, : width + dx] & test_strip[:, -dx:])
                )
            else:
                intersection = int(np.sum(ref_strip & test_strip))
            if intersection > best_intersection:
                best_intersection = intersection
                best_dy = dy
                best_dx = dx

    shifted = np.zeros_like(test_ink)
    target_y0 = max(0, best_dy)
    target_y1 = min(height, height + best_dy)
    source_y0 = max(0, -best_dy)
    source_y1 = source_y0 + (target_y1 - target_y0)
    target_x0 = max(0, best_dx)
    target_x1 = min(width, width + best_dx)
    source_x0 = max(0, -best_dx)
    source_x1 = source_x0 + (target_x1 - target_x0)
    shifted[target_y0:target_y1, target_x0:target_x1] = test_ink[
        source_y0:source_y1, source_x0:source_x1
    ]

    test_count = int(np.sum(shifted))
    intersection = int(np.sum(ref_ink & shifted))
    union = int(np.sum(ref_ink | shifted))
    return (
        intersection,
        union,
        ref_count,
        test_count,
        best_dy,
        best_dx,
        shifted,
    )


def _dilate_mask(mask: np.ndarray, radius: int) -> np.ndarray:
    if radius <= 0:
        return mask
    image = Image.fromarray(mask.astype(np.uint8) * 255, "L")
    return np.array(image.filter(ImageFilter.MaxFilter(radius * 2 + 1))) > 0


def compute_ink_metrics(
    ref_img: np.ndarray,
    test_img: np.ndarray,
    *,
    prooftree_tolerant: bool = False,
) -> dict[str, Any]:
    """Compute the versioned ratex-ink-v2 metric."""
    ref_crop = crop_to_content(ref_img)
    test_crop = crop_to_content(test_img)
    ref_norm = normalize_size(ref_crop)
    test_norm = normalize_size(test_crop)
    _, ref_width = ref_norm.shape[:2]
    _, test_width = test_norm.shape[:2]
    width = max(ref_width, test_width)

    def pad_width(img: np.ndarray, target_width: int) -> np.ndarray:
        height, current_width = img.shape[:2]
        if current_width >= target_width:
            return img[:, :target_width]
        padded = np.full((height, target_width, 3), 255, dtype=np.uint8)
        padded[:, :current_width] = img
        return padded

    ref_final = pad_width(ref_norm, width)
    test_final = pad_width(test_norm, width)
    ref_ink = get_ink_mask(ref_final)
    test_ink = get_ink_mask(test_final)
    ref_ink_count = int(np.sum(ref_ink))
    test_ink_count = int(np.sum(test_ink))

    # Both formulas are blank: this is a perfect match (e.g. \\allowbreak).
    # Without this special case, precision/recall are 0/0 and the combined
    # score is pinned at 0.8 even though the rendered output is identical.
    if ref_ink_count == 0 and test_ink_count == 0:
        return {
            "score": 1.0,
            "iou": 1.0,
            "precision": 1.0,
            "recall": 1.0,
            "f1": 1.0,
            "aspect_similarity": 1.0,
            "width_similarity": 1.0,
            "best_dx": 0,
            "best_dy": 0,
            "ref_size": [ref_img.shape[1], ref_img.shape[0]],
            "test_size": [test_img.shape[1], test_img.shape[0]],
            "ref_crop": [ref_crop.shape[1], ref_crop.shape[0]],
            "test_crop": [test_crop.shape[1], test_crop.shape[0]],
            "ref_ink_px": 0,
            "test_ink_px": 0,
            "tolerant_f1": 1.0 if prooftree_tolerant else None,
        }

    (
        intersection,
        union,
        ref_count,
        test_count,
        best_dy,
        best_dx,
        aligned_test_ink,
    ) = _best_2d_alignment(
        ref_ink,
        test_ink,
        NORM_HEIGHT // 8,
        max(ref_width, test_width) // 16,
    )

    iou = intersection / union if union else 1.0
    precision = intersection / test_count if test_count else 0.0
    recall = intersection / ref_count if ref_count else 0.0
    f1 = (
        2 * precision * recall / (precision + recall)
        if precision + recall
        else 0.0
    )
    ref_aspect = ref_crop.shape[1] / ref_crop.shape[0]
    test_aspect = test_crop.shape[1] / test_crop.shape[0]
    aspect_similarity = min(ref_aspect, test_aspect) / max(ref_aspect, test_aspect)
    width_similarity = min(ref_width, test_width) / max(ref_width, test_width)

    score = (
        0.4 * iou
        + 0.2 * recall
        + 0.2 * aspect_similarity
        + 0.2 * width_similarity
    )
    tolerant_f1 = None
    if prooftree_tolerant:
        tolerance_px = 20
        ref_dilated = _dilate_mask(ref_ink, tolerance_px)
        test_dilated = _dilate_mask(aligned_test_ink, tolerance_px)
        tolerant_recall = (
            float(np.sum(ref_ink & test_dilated)) / ref_count if ref_count else 0.0
        )
        tolerant_precision = (
            float(np.sum(aligned_test_ink & ref_dilated)) / test_count
            if test_count
            else 0.0
        )
        tolerant_f1 = (
            2
            * tolerant_precision
            * tolerant_recall
            / (tolerant_precision + tolerant_recall)
            if tolerant_precision + tolerant_recall
            else 0.0
        )
        score = max(
            score,
            0.5 * tolerant_f1
            + 0.25 * aspect_similarity
            + 0.25 * width_similarity,
        )

    return {
        "score": float(score),
        "iou": float(iou),
        "precision": float(precision),
        "recall": float(recall),
        "f1": float(f1),
        "aspect_similarity": float(aspect_similarity),
        "width_similarity": float(width_similarity),
        "best_dx": best_dx,
        "best_dy": best_dy,
        "ref_size": [ref_img.shape[1], ref_img.shape[0]],
        "test_size": [test_img.shape[1], test_img.shape[0]],
        "ref_crop": [ref_crop.shape[1], ref_crop.shape[0]],
        "test_crop": [test_crop.shape[1], test_crop.shape[0]],
        "ref_ink_px": ref_count,
        "test_ink_px": test_count,
        "tolerant_f1": float(tolerant_f1) if tolerant_f1 is not None else None,
    }


def save_diff_image(ref_img: np.ndarray, test_img: np.ndarray, diff_path: Path) -> None:
    ref_norm = normalize_size(crop_to_content(ref_img))
    test_norm = normalize_size(crop_to_content(test_img))
    width = max(ref_norm.shape[1], test_norm.shape[1])

    def pad_width(img: np.ndarray) -> np.ndarray:
        padded = np.full((NORM_HEIGHT, width, 3), 255, dtype=np.uint8)
        padded[:, : img.shape[1]] = img
        return padded

    ref_final = pad_width(ref_norm)
    test_final = pad_width(test_norm)
    ref_ink = get_ink_mask(ref_final)
    test_ink = get_ink_mask(test_final)
    visual = np.full_like(ref_final, 255)
    visual[ref_ink & test_ink] = [0, 0, 0]
    visual[ref_ink & ~test_ink] = [0, 200, 0]
    visual[~ref_ink & test_ink] = [200, 0, 0]
    gap = np.full((NORM_HEIGHT, 4, 3), 200, dtype=np.uint8)
    Image.fromarray(
        np.hstack([ref_final, gap, test_final, gap, visual]), "RGB"
    ).save(diff_path)


def categorize_formula(formula: str) -> list[str]:
    categories: list[str] = []

    def add(name: str, condition: bool) -> None:
        if condition and name not in categories:
            categories.append(name)

    add("chemistry", "\\ce" in formula or "\\pu" in formula)
    add("environment", "\\begin{" in formula)
    add(
        "matrix",
        any(
            token in formula
            for token in (
                "{matrix}",
                "{pmatrix}",
                "{bmatrix}",
                "{vmatrix}",
                "{Vmatrix}",
                "{array}",
                "{aligned",
                "{align",
                "{gather",
            )
        ),
    )
    add(
        "delimiter",
        bool(
            re.search(
                r"\\(?:left|right|middle|big|Big|bigg|Bigg|lvert|rvert|vert|Vert|langle|rangle)",
                formula,
            )
        ),
    )
    add("sized", bool(re.search(r"\\(?:big|Big|bigg|Bigg)[lmr]?", formula)))
    add("vertical-bar", "\\vert" in formula or "\\Vert" in formula or "|" in formula)
    add("fraction", "\\frac" in formula or "\\over" in formula or "\\above" in formula)
    add("radical", "\\sqrt" in formula)
    add("script", "^" in formula or "_" in formula)
    add(
        "accent",
        bool(
            re.search(
                r"\\(?:acute|bar|breve|check|dot|ddot|grave|hat|tilde|vec|widehat|widetilde)",
                formula,
            )
        ),
    )
    add("text", "\\text" in formula or "\\operatorname" in formula)
    add("spacing", bool(re.search(r"\\(?:quad|qquad|,|;|!|:|>|kern|hspace)", formula)))
    add("color", "\\color" in formula or "\\textcolor" in formula)
    add("font", bool(re.search(r"\\(?:mathbf|mathit|mathrm|mathsf|mathtt|Bbb|bf|rm)\b", formula)))
    add("image", "\\includegraphics" in formula)
    add("html", bool(re.search(r"\\html(?:Class|Id|Style|Data)", formula)))
    add("operator", bool(re.search(r"\\(?:sum|prod|int|lim|argmax|argmin)\b", formula)))
    add(
        "relation",
        "=" in formula
        or bool(re.search(r"\\(?:approx|equiv|leq|geq|sim|subset|supset)", formula)),
    )
    return categories or ["symbol"]


def load_index_manifest(path: Path) -> tuple[dict[int, dict[str, Any]], list[str]]:
    if not path.exists():
        return {}, []
    errors: list[str] = []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return {}, [f"cannot read manifest {path}: {exc}"]
    records: dict[int, dict[str, Any]] = {}
    for raw in data.get("cases", []):
        try:
            index = int(raw["index"])
        except (KeyError, TypeError, ValueError):
            errors.append(f"manifest {path} has a case without a valid index")
            continue
        if index in records:
            errors.append(f"manifest {path} repeats index {index:04d}")
        records[index] = raw
    return records, errors


def load_json_object(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def validate_generation_manifest(
    path: Path,
    records: dict[int, dict[str, Any]],
    formulas: list[str],
    test_cases_sha256: str,
) -> list[str]:
    label = path.name
    if not path.exists():
        return [f"missing generation manifest: {path}"]
    errors: list[str] = []
    metadata = load_json_object(path)
    expected_indices = set(range(1, len(formulas) + 1))
    actual_indices = set(records)
    missing = sorted(expected_indices - actual_indices)
    extra = sorted(actual_indices - expected_indices)
    if missing:
        errors.append(
            f"{label} is missing indices: "
            + ", ".join(f"{index:04d}" for index in missing[:20])
        )
    if extra:
        errors.append(
            f"{label} has out-of-range indices: "
            + ", ".join(f"{index:04d}" for index in extra[:20])
        )
    if metadata.get("case_count") != len(formulas):
        errors.append(
            f"{label} case_count={metadata.get('case_count')!r}, "
            f"expected {len(formulas)}"
        )
    if metadata.get("test_cases_sha256") != test_cases_sha256:
        errors.append(f"{label} test_cases_sha256 does not match the current suite")
    for index in sorted(expected_indices & actual_indices):
        manifest_formula = records[index].get("formula")
        if manifest_formula is not None and manifest_formula != formulas[index - 1]:
            errors.append(f"{label} formula mismatch at index {index:04d}")
    return errors


def load_policy(
    path: Path | None, formulas: list[str]
) -> tuple[dict[int, dict[str, Any]], list[str], str | None]:
    if path is None or not path.exists():
        return {}, [], None
    errors: list[str] = []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return {}, [f"cannot read policy {path}: {exc}"], None
    cases = data.get("cases", {})
    policy: dict[int, dict[str, Any]] = {}
    for raw_index, entry in cases.items():
        try:
            index = int(raw_index)
        except (TypeError, ValueError):
            errors.append(f"policy index is not an integer: {raw_index!r}")
            continue
        if not 1 <= index <= len(formulas):
            errors.append(f"policy index {index} is outside 1..{len(formulas)}")
            continue
        if not isinstance(entry, dict):
            errors.append(f"policy index {index} must contain an object")
            continue
        status = entry.get("status")
        if status not in ALLOWED_POLICY_STATUSES:
            errors.append(f"policy index {index} has invalid status {status!r}")
        expected_formula = entry.get("formula")
        if expected_formula is not None and expected_formula != formulas[index - 1]:
            errors.append(f"policy index {index} formula no longer matches the suite")
        if not entry.get("reason"):
            errors.append(f"policy index {index} must include a reason")
        policy[index] = entry
    return policy, errors, sha256_file(path)


def discover_pngs(
    directory: Path, formula_count: int, label: str
) -> tuple[dict[int, Path], list[str]]:
    images: dict[int, Path] = {}
    errors: list[str] = []
    if not directory.is_dir():
        return images, [f"{label} directory does not exist: {directory}"]
    for path in sorted(directory.glob("*.png")):
        if not re.fullmatch(r"\d{4,}\.png", path.name):
            errors.append(f"{label} has a non-canonical PNG name: {path.name}")
            continue
        index = int(path.stem)
        canonical = f"{index:04d}.png"
        if path.name != canonical:
            errors.append(f"{label} filename is not canonical: {path.name}")
        if not 1 <= index <= formula_count:
            errors.append(
                f"{label} index {index:04d} is outside 0001..{formula_count:04d}"
            )
            continue
        if index in images:
            errors.append(f"{label} repeats index {index:04d}")
        images[index] = path
    return images, errors


def directory_manifest_hash(
    directory: Path,
    images: dict[int, Path],
    manifest_path: Path,
) -> tuple[str, list[dict[str, str]]]:
    entries: list[dict[str, str]] = [
        {"path": path.name, "sha256": sha256_file(path)}
        for _, path in sorted(images.items())
    ]
    if manifest_path.exists():
        entries.append(
            {
                "path": manifest_path.name,
                "sha256": sha256_file(manifest_path),
            }
        )
    canonical = "".join(
        f"{entry['path']}\0{entry['sha256']}\n" for entry in entries
    )
    return sha256_text(canonical), entries


def empty_case_record(index: int, formula: str) -> dict[str, Any]:
    return {
        "index": index,
        "formula": formula,
        "status": None,
        "score": None,
        "iou": None,
        "precision": None,
        "recall": None,
        "f1": None,
        "aspect_similarity": None,
        "width_similarity": None,
        "best_dx": None,
        "best_dy": None,
        "ref_size": None,
        "test_size": None,
        "ref_crop": None,
        "test_crop": None,
        "ref_ink_px": None,
        "test_ink_px": None,
        "tolerant_f1": None,
        "passed": False,
        "categories": categorize_formula(formula),
        "reason": None,
        "issues": [],
        "policy": None,
    }


def manifest_failure(
    manifest_entry: dict[str, Any] | None,
    fallback: str,
) -> tuple[str, str | None]:
    if not manifest_entry:
        return fallback, None
    status = manifest_entry.get("status")
    if status in ERROR_STATUSES or status == "unsupported":
        return status, manifest_entry.get("reason") or manifest_entry.get("error")
    return fallback, manifest_entry.get("reason") or manifest_entry.get("error")


def compact_baseline(report: dict[str, Any]) -> dict[str, Any]:
    """Return the minimal version-controlled baseline representation."""
    return {
        "v": 1,
        "metric": report["metric_version"],
        "suite": report["suite"],
        "hash": report["source"]["suite_hash"],
        "scores": [
            round(float(case["score"]), 6)
            if case["status"] == "scored"
            else None
            for case in report["cases"]
        ],
    }


def write_compact_baseline(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            compact_baseline(report),
            ensure_ascii=False,
            separators=(",", ":"),
            allow_nan=False,
        )
        + "\n",
        encoding="utf-8",
    )


def compare_baseline(
    report: dict[str, Any],
    baseline_path: Path,
    max_case_regression: float | None,
    baseline_formulas_path: Path | None = None,
) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    try:
        baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return {}, [f"cannot read baseline report {baseline_path}: {exc}"]
    compact = isinstance(baseline.get("scores"), list)
    baseline_metric = (
        baseline.get("metric") if compact else baseline.get("metric_version")
    )
    baseline_suite = baseline.get("suite")
    baseline_suite_hash = (
        baseline.get("hash") if compact else baseline.get("source", {}).get("suite_hash")
    )

    if baseline_metric != report.get("metric_version"):
        errors.append(
            "baseline metric_version does not match current report: "
            f"{baseline_metric!r} != {report.get('metric_version')!r}"
        )
    if baseline_suite != report.get("suite"):
        errors.append(
            "baseline suite does not match current report: "
            f"{baseline_suite!r} != {report.get('suite')!r}"
        )
    current_suite_hash = report.get("source", {}).get("suite_hash")
    suite_changed = baseline_suite_hash != current_suite_hash

    if compact:
        if baseline.get("v") != 1:
            errors.append(f"unsupported compact baseline version: {baseline.get('v')!r}")
        if baseline_formulas_path is not None:
            baseline_formulas = read_formulas(baseline_formulas_path)
        elif not suite_changed:
            baseline_formulas = [case["formula"] for case in report["cases"]]
        else:
            baseline_formulas = []
            errors.append(
                "a compact baseline with a different suite hash requires "
                "--baseline-formulas"
            )
        scores = baseline["scores"]
        invalid_scores = [
            index
            for index, score in enumerate(scores, start=1)
            if score is not None
            and (
                isinstance(score, bool)
                or not isinstance(score, (int, float))
                or not math.isfinite(score)
                or not 0.0 <= score <= 1.0
            )
        ]
        if invalid_scores:
            errors.append(
                "compact baseline contains invalid scores at indices: "
                + ", ".join(f"{index:04d}" for index in invalid_scores[:20])
            )
        if len(scores) != len(baseline_formulas):
            errors.append(
                "compact baseline score count does not match its formula count: "
                f"{len(scores)} != {len(baseline_formulas)}"
            )
        if baseline_formulas and suite_hash_for_formulas(
            baseline_formulas
        ) != baseline_suite_hash:
            errors.append("compact baseline hash does not match its formula list")
        baseline_cases = [
            {
                "index": index,
                "formula": formula,
                "score": score if index not in invalid_scores else None,
            }
            for index, (formula, score) in enumerate(
                zip(baseline_formulas, scores), start=1
            )
        ]
    else:
        baseline_cases = baseline.get("cases", [])

    # Formula identity, rather than the numeric slot, is the stable comparison
    # key when cases are appended, removed, or reordered.  A queue preserves
    # duplicate formulas by matching their occurrences in report order.
    old_cases_by_formula: dict[str, deque[dict[str, Any]]] = defaultdict(deque)
    for item in baseline_cases:
        formula = item.get("formula")
        if isinstance(formula, str):
            old_cases_by_formula[formula].append(item)

    regressions: list[dict[str, Any]] = []
    added_cases: list[dict[str, Any]] = []
    compared = 0
    for current in report["cases"]:
        candidates = old_cases_by_formula.get(current["formula"])
        if not candidates:
            added_cases.append(
                {
                    "index": current["index"],
                    "formula": current["formula"],
                }
            )
            continue
        previous = candidates.popleft()
        old_score = float(previous.get("score") or 0.0)
        new_score = float(current.get("score") or 0.0)
        regression = old_score - new_score
        compared += 1
        if max_case_regression is not None and regression > max_case_regression:
            regressions.append(
                {
                    "index": current["index"],
                    "baseline_index": previous["index"],
                    "formula": current["formula"],
                    "baseline_score": old_score,
                    "current_score": new_score,
                    "regression": regression,
                }
            )
    removed_cases = [
        {
            "index": previous["index"],
            "formula": formula,
        }
        for formula, candidates in old_cases_by_formula.items()
        for previous in candidates
    ]
    if not suite_changed and (added_cases or removed_cases):
        errors.append(
            "baseline cases do not match current cases despite an identical suite_hash"
        )

    comparison = {
        "baseline_report": str(baseline_path),
        "baseline_suite_hash": baseline_suite_hash,
        "current_suite_hash": current_suite_hash,
        "suite_changed": suite_changed,
        "compared_case_count": compared,
        "added_case_count": len(added_cases),
        "removed_case_count": len(removed_cases),
        "added_cases": added_cases,
        "removed_cases": removed_cases,
        "max_case_regression": max_case_regression,
        "regression_count": len(regressions),
        "worst_regression": max(
            (item["regression"] for item in regressions), default=0.0
        ),
        "regressions": regressions,
    }
    return comparison, errors


def build_report(args: argparse.Namespace) -> tuple[dict[str, Any], list[str]]:
    started = time.monotonic()
    test_cases = Path(args.test_cases).resolve()
    fixtures_dir = Path(args.fixtures).resolve()
    output_dir = Path(args.output).resolve()
    repo_root = Path(__file__).resolve().parents[2]
    tool_dir = Path(__file__).resolve().parent
    formulas = read_formulas(test_cases)
    formula_count = len(formulas)
    test_cases_sha256 = sha256_file(test_cases)

    fixture_images, fixture_errors = discover_pngs(
        fixtures_dir, formula_count, "fixture"
    )
    output_images, output_errors = discover_pngs(
        output_dir, formula_count, "output"
    )
    reference_manifest_path = fixtures_dir / "reference-manifest.json"
    render_manifest_path = output_dir / "render-manifest.json"
    reference_manifest, reference_manifest_errors = load_index_manifest(
        reference_manifest_path
    )
    reference_metadata = load_json_object(reference_manifest_path)
    render_manifest, render_manifest_errors = load_index_manifest(
        render_manifest_path
    )
    if reference_manifest_path.exists() or args.require_manifests:
        reference_manifest_errors.extend(
            validate_generation_manifest(
                reference_manifest_path,
                reference_manifest,
                formulas,
                test_cases_sha256,
            )
        )
    if render_manifest_path.exists() or args.require_manifests:
        render_manifest_errors.extend(
            validate_generation_manifest(
                render_manifest_path,
                render_manifest,
                formulas,
                test_cases_sha256,
            )
        )

    policy_path = Path(args.policy).resolve() if args.policy else None
    policy, policy_errors, policy_sha = load_policy(policy_path, formulas)
    integrity_errors = (
        fixture_errors
        + output_errors
        + reference_manifest_errors
        + render_manifest_errors
        + policy_errors
    )

    if args.diff_dir:
        Path(args.diff_dir).mkdir(parents=True, exist_ok=True)
    prooftree_tolerant = args.prooftree_tolerant or any(
        "prooftree" in str(path).lower()
        for path in (fixtures_dir, output_dir, test_cases)
    )

    records: list[dict[str, Any]] = []
    for index, formula in enumerate(formulas, start=1):
        record = empty_case_record(index, formula)
        ref_path = fixture_images.get(index)
        test_path = output_images.get(index)
        if ref_path is None:
            record["issues"].append("missing_fixture")
        if test_path is None:
            record["issues"].append("missing_output")

        policy_entry = policy.get(index)
        if policy_entry is not None:
            record["status"] = policy_entry["status"]
            record["reason"] = policy_entry["reason"]
            record["policy"] = {
                key: value
                for key, value in policy_entry.items()
                if key != "formula"
            }
        elif ref_path is None:
            status, reason = manifest_failure(
                reference_manifest.get(index), "missing_fixture"
            )
            record["status"] = status
            record["reason"] = reason or f"missing {index:04d}.png in fixture directory"
        elif test_path is None:
            status, reason = manifest_failure(
                render_manifest.get(index), "missing_output"
            )
            record["status"] = status
            record["reason"] = reason or f"missing {index:04d}.png in output directory"
        else:
            try:
                ref_img = load_image(ref_path)
                test_img = load_image(test_path)
                metrics = compute_ink_metrics(
                    ref_img,
                    test_img,
                    prooftree_tolerant=prooftree_tolerant,
                )
                record.update(metrics)
                record["status"] = "scored"
                record["passed"] = metrics["score"] >= args.threshold
                if args.diff_dir:
                    in_requested_range = args.diff_from is not None and (
                        index >= args.diff_from
                        and (args.diff_to is None or index <= args.diff_to)
                    )
                    if in_requested_range or (
                        args.diff_from is None and not record["passed"]
                    ):
                        save_diff_image(
                            ref_img,
                            test_img,
                            Path(args.diff_dir) / f"{index:04d}_diff.png",
                        )
            except Exception as exc:  # Pillow/NumPy errors become explicit case records.
                record["status"] = "render_error"
                record["reason"] = str(exc)
        records.append(record)

    status_counts = Counter(record["status"] for record in records)
    scored = [record for record in records if record["status"] == "scored"]
    scores = [float(record["score"]) for record in scored]
    policy_excluded_count = sum(1 for record in records if record["policy"] is not None)
    eligible_count = formula_count - policy_excluded_count
    scored_count = len(scored)
    coverage = scored_count / eligible_count if eligible_count else 1.0
    raw_coverage = scored_count / formula_count if formula_count else 1.0
    rendered_mean = float(np.mean(scores)) if scores else 0.0
    median = float(np.median(scores)) if scores else 0.0
    coverage_adjusted_mean = (
        float(sum(scores)) / formula_count if formula_count else 0.0
    )
    passed_count = sum(1 for record in scored if record["passed"])
    pass_rate = passed_count / scored_count if scored_count else 0.0

    # A generated status manifest accounts for a failed slot even when no PNG
    # exists. This is how formula/output/report counts remain equal without
    # disguising an unsupported or failed render as a scored image.
    fixture_slots = set(fixture_images) | set(reference_manifest)
    output_slots = set(output_images) | set(render_manifest)
    fixture_count = sum(
        1 for index in fixture_slots if 1 <= index <= formula_count
    )
    output_count = sum(
        1 for index in output_slots if 1 <= index <= formula_count
    )
    counts_equal = (
        formula_count == fixture_count == output_count == len(records)
    )
    if not counts_equal:
        integrity_errors.append(
            "count mismatch: "
            f"formula_count={formula_count}, fixture_count={fixture_count}, "
            f"output_count={output_count}, report_case_count={len(records)}"
        )

    fixture_manifest_sha, fixture_entries = directory_manifest_hash(
        fixtures_dir, fixture_images, reference_manifest_path
    )
    output_manifest_sha, output_entries = directory_manifest_hash(
        output_dir, output_images, render_manifest_path
    )
    suite_hash = suite_hash_for_formulas(formulas)
    commit_sha = (
        os.environ.get("GITHUB_SHA")
        or command_output(["git", "rev-parse", "HEAD"], cwd=repo_root)
        or "unknown"
    )
    dirty = bool(command_output(["git", "status", "--porcelain"], cwd=repo_root))
    katex_version = load_package_lock_version(tool_dir / "package-lock.json", "katex")

    environment = (
        collect_environment(repo_root, tool_dir)
        if not args.skip_environment
        else {}
    )
    for key in ("puppeteer_version", "chromium_revision", "chromium_version"):
        if reference_metadata.get(key):
            environment[key] = reference_metadata[key]

    report: dict[str, Any] = {
        "report_version": REPORT_VERSION,
        "metric_version": METRIC_VERSION,
        "suite": args.suite,
        "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "generation_duration_seconds": time.monotonic() - started,
        "source": {
            "commit_sha": commit_sha,
            "working_tree_dirty": dirty,
            "test_cases": os.path.relpath(test_cases, repo_root),
            "test_cases_sha256": test_cases_sha256,
            "suite_hash": suite_hash,
            "policy": (
                os.path.relpath(policy_path, repo_root) if policy_path else None
            ),
            "policy_sha256": policy_sha,
        },
        "reference": {
            "implementation": "KaTeX",
            "katex_version": reference_metadata.get("katex_version")
            or katex_version,
            "dpr": reference_metadata.get("dpr", args.reference_dpr),
        },
        "renderer": {
            "implementation": "RaTeX",
            "dpr": args.output_dpr,
        },
        "environment": environment,
        "manifests": {
            "fixture_manifest_sha256": fixture_manifest_sha,
            "output_manifest_sha256": output_manifest_sha,
            "fixture_files": fixture_entries,
            "output_files": output_entries,
        },
        "integrity": {
            "complete": counts_equal and not integrity_errors,
            "counts_equal": counts_equal,
            "continuous_indices": len(records) == formula_count
            and all(record["index"] == i for i, record in enumerate(records, 1)),
            "errors": integrity_errors,
        },
        "summary": {
            "formula_count": formula_count,
            "fixture_count": fixture_count,
            "output_count": output_count,
            "report_case_count": len(records),
            "fixture_png_count": len(fixture_images),
            "output_png_count": len(output_images),
            "scored_count": scored_count,
            "policy_excluded_count": policy_excluded_count,
            "eligible_count": eligible_count,
            "coverage": coverage,
            "raw_coverage": raw_coverage,
            "rendered_mean": rendered_mean,
            "coverage_adjusted_mean": coverage_adjusted_mean,
            "median": median,
            "minimum": min(scores) if scores else 0.0,
            "maximum": max(scores) if scores else 0.0,
            "pass_threshold": args.threshold,
            "passed_count": passed_count,
            "pass_rate": pass_rate,
            "status_counts": dict(sorted(status_counts.items())),
        },
        "cases": records,
    }

    if args.baseline_report:
        comparison, baseline_errors = compare_baseline(
            report,
            Path(args.baseline_report).resolve(),
            args.max_case_regression,
            (
                Path(args.baseline_formulas).resolve()
                if args.baseline_formulas
                else None
            ),
        )
        report["baseline_comparison"] = comparison
        integrity_errors.extend(baseline_errors)
        report["integrity"]["errors"] = integrity_errors
        report["integrity"]["complete"] = counts_equal and not integrity_errors
    return report, integrity_errors


def write_json_report(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def write_csv_report(path: Path, records: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fields = [
        "index",
        "formula",
        "status",
        "score",
        "iou",
        "precision",
        "recall",
        "f1",
        "aspect_similarity",
        "width_similarity",
        "best_dx",
        "best_dy",
        "ref_crop",
        "test_crop",
        "categories",
        "passed",
        "reason",
    ]
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        for record in records:
            row = {key: record.get(key) for key in fields}
            row["ref_crop"] = json.dumps(row["ref_crop"], separators=(",", ":"))
            row["test_crop"] = json.dumps(row["test_crop"], separators=(",", ":"))
            row["categories"] = ";".join(record["categories"])
            writer.writerow(row)


def print_summary(report: dict[str, Any]) -> None:
    summary = report["summary"]
    print("\n" + "=" * 72)
    print(
        f"Golden Test ({report['metric_version']}): "
        f"{summary['scored_count']}/{summary['eligible_count']} eligible cases scored "
        f"({summary['coverage']:.2%} coverage)"
    )
    print(
        f"Mean: rendered={summary['rendered_mean']:.6f}  "
        f"coverage-adjusted={summary['coverage_adjusted_mean']:.6f}  "
        f"median={summary['median']:.6f}"
    )
    print(
        f"Counts: formulas={summary['formula_count']} "
        f"fixtures={summary['fixture_count']} outputs={summary['output_count']} "
        f"report={summary['report_case_count']}"
    )
    print("Statuses: " + json.dumps(summary["status_counts"], sort_keys=True))
    print("=" * 72)
    if report["integrity"]["errors"]:
        print("\nIntegrity errors:")
        for error in report["integrity"]["errors"]:
            print(f"  - {error}")
    failures = [
        record
        for record in report["cases"]
        if record["status"] != "scored" or not record["passed"]
    ]
    if failures:
        print(f"\nNon-scored or below-threshold cases ({len(failures)}):")
        for record in failures[:20]:
            score = (
                f"{record['score']:.3f}" if record["score"] is not None else "n/a"
            )
            print(
                f"  {record['index']:04d}: status={record['status']} "
                f"score={score} | {record['formula'][:60]}"
            )
        if len(failures) > 20:
            print(f"  ... and {len(failures) - 20} more")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Authoritative RaTeX golden comparison and report generator"
    )
    parser.add_argument("--ce", "--mhchem", action="store_true", dest="ce")
    parser.add_argument("--fixtures")
    parser.add_argument("--output")
    parser.add_argument("--test-cases")
    parser.add_argument("--suite")
    parser.add_argument("--policy")
    parser.add_argument("--threshold", type=float, default=DEFAULT_PASS_THRESHOLD)
    parser.add_argument("--diff-dir")
    parser.add_argument("--diff-from", type=int)
    parser.add_argument("--diff-to", type=int)
    parser.add_argument("--json-out")
    parser.add_argument("--csv-out")
    parser.add_argument("--baseline-out")
    parser.add_argument("--fail-on-missing", action="store_true")
    parser.add_argument("--min-coverage", type=float)
    parser.add_argument(
        "--min-mean",
        type=float,
        help="Minimum coverage-adjusted mean (unscored cases contribute zero)",
    )
    parser.add_argument("--baseline-report")
    parser.add_argument("--baseline-formulas")
    parser.add_argument("--max-case-regression", type=float)
    parser.add_argument(
        "--require-manifests",
        action="store_true",
        help="Fail if generated reference or render manifests are missing",
    )
    parser.add_argument("--reference-dpr", type=float, default=2.0)
    parser.add_argument("--output-dpr", type=float, default=1.0)
    parser.add_argument("--prooftree-tolerant", action="store_true")
    parser.add_argument(
        "--skip-environment",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    args = parser.parse_args(argv)

    repo_root = Path(__file__).resolve().parents[2]
    golden = repo_root / "tests" / "golden"
    if args.ce:
        args.fixtures = args.fixtures or str(golden / "fixtures_ce")
        args.output = args.output or str(golden / "output_ce")
        args.test_cases = args.test_cases or str(golden / "test_case_ce.txt")
        args.suite = args.suite or "mhchem"
        args.output_dpr = 2.0
    else:
        args.fixtures = args.fixtures or str(golden / "fixtures")
        args.output = args.output or str(golden / "output")
        args.test_cases = args.test_cases or str(golden / "test_cases.txt")
        args.suite = args.suite or "main"
        default_policy = golden / "policy.json"
        if args.policy is None and default_policy.exists():
            args.policy = str(default_policy)

    for name in ("threshold", "min_coverage", "min_mean", "max_case_regression"):
        value = getattr(args, name)
        if value is not None and not 0.0 <= value <= 1.0:
            parser.error(f"--{name.replace('_', '-')} must be between 0 and 1")
    if args.max_case_regression is not None and not args.baseline_report:
        parser.error("--max-case-regression requires --baseline-report")
    if args.baseline_formulas is not None and not args.baseline_report:
        parser.error("--baseline-formulas requires --baseline-report")
    if args.diff_to is not None and args.diff_from is None:
        parser.error("--diff-to requires --diff-from")
    if args.diff_from is not None and not args.diff_dir:
        parser.error("--diff-from requires --diff-dir")
    return args


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report, integrity_errors = build_report(args)
    if args.json_out:
        write_json_report(Path(args.json_out), report)
    if args.csv_out:
        write_csv_report(Path(args.csv_out), report["cases"])
    if args.baseline_out:
        write_compact_baseline(Path(args.baseline_out), report)
    print_summary(report)

    failures: list[str] = []
    summary = report["summary"]
    if args.fail_on_missing:
        unapproved = [
            record
            for record in report["cases"]
            if record["status"] in ERROR_STATUSES and record["policy"] is None
        ]
        if unapproved:
            failures.append(
                f"{len(unapproved)} missing/error case(s) lack an explicit policy"
            )
        if integrity_errors:
            failures.append(f"{len(integrity_errors)} integrity error(s)")
    if args.min_coverage is not None and summary["coverage"] < args.min_coverage:
        failures.append(
            f"coverage {summary['coverage']:.6f} < {args.min_coverage:.6f}"
        )
    if (
        args.min_mean is not None
        and summary["coverage_adjusted_mean"] < args.min_mean
    ):
        failures.append(
            "coverage-adjusted mean "
            f"{summary['coverage_adjusted_mean']:.6f} < {args.min_mean:.6f}"
        )
    comparison = report.get("baseline_comparison")
    if comparison and comparison.get("regression_count", 0):
        failures.append(
            f"{comparison['regression_count']} case regression(s) exceed "
            f"{comparison['max_case_regression']:.6f}"
        )
    if failures:
        print("\nGate failures:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
