#!/usr/bin/env python3
"""
Compare RaTeX rendered PNGs against KaTeX reference PNGs.

Uses ink-coverage-based comparison instead of raw pixel diff:
1. Extract non-white (ink) pixels from both images
2. Compute IoU (Intersection over Union) of ink regions
3. Compare ink pixel color similarity in overlapping areas

Usage:
    python3 compare_golden.py [--fixtures DIR] [--output DIR] [--threshold FLOAT]
    python3 compare_golden.py --ce   # mhchem: fixtures_ce vs output_ce, test_case_ce.txt
"""
import argparse
import os
import sys
from pathlib import Path

try:
    from PIL import Image
    import numpy as np
except ImportError:
    print("Install: pip install Pillow numpy", file=sys.stderr)
    sys.exit(1)

INK_THRESHOLD = 240  # pixel channel value below this is considered "ink"
NORM_HEIGHT = 120     # normalize all crops to this height for comparison


def load_image(path: str) -> np.ndarray:
    img = Image.open(path).convert("RGB")
    return np.array(img, dtype=np.uint8)


def get_ink_mask(img: np.ndarray) -> np.ndarray:
    """Return boolean mask where True = non-white (ink) pixel."""
    return np.any(img < INK_THRESHOLD, axis=2)


def crop_to_content(img: np.ndarray, margin: int = 2) -> np.ndarray:
    """Crop image to bounding box of non-white content."""
    mask = get_ink_mask(img)
    if not np.any(mask):
        return img[:10, :10]  # fallback: tiny empty
    rows = np.any(mask, axis=1)
    cols = np.any(mask, axis=0)
    rmin, rmax = np.where(rows)[0][[0, -1]]
    cmin, cmax = np.where(cols)[0][[0, -1]]
    rmin = max(0, rmin - margin)
    rmax = min(img.shape[0] - 1, rmax + margin)
    cmin = max(0, cmin - margin)
    cmax = min(img.shape[1] - 1, cmax + margin)
    return img[rmin:rmax + 1, cmin:cmax + 1]


def normalize_size(img: np.ndarray, target_h: int = NORM_HEIGHT) -> np.ndarray:
    """Resize image to target height, preserving aspect ratio."""
    h, w = img.shape[:2]
    if h == 0 or w == 0:
        return np.full((target_h, target_h, 3), 255, dtype=np.uint8)
    scale = target_h / h
    new_w = max(1, int(w * scale))
    pil_img = Image.fromarray(img)
    resized = pil_img.resize((new_w, target_h), Image.Resampling.LANCZOS)
    return np.array(resized, dtype=np.uint8)


def compute_ink_metrics(ref_img: np.ndarray, test_img: np.ndarray) -> dict:
    """Compare two images: crop to content, normalize size, then compare ink overlap."""
    # Step 1: crop to content bounding box
    ref_crop = crop_to_content(ref_img)
    test_crop = crop_to_content(test_img)

    # Step 2: normalize to same height
    ref_norm = normalize_size(ref_crop)
    test_norm = normalize_size(test_crop)

    # Step 3: pad to same width for pixel-level comparison
    rh, rw = ref_norm.shape[:2]
    th, tw = test_norm.shape[:2]
    w = max(rw, tw)

    def pad_w(img, target_w):
        h, cur_w = img.shape[:2]
        if cur_w >= target_w:
            return img[:, :target_w]
        padded = np.full((h, target_w, 3), 255, dtype=np.uint8)
        padded[:, :cur_w] = img
        return padded

    ref_final = pad_w(ref_norm, w)
    test_final = pad_w(test_norm, w)

    # Step 4: compute ink-based metrics
    ref_ink = get_ink_mask(ref_final)
    test_ink = get_ink_mask(test_final)

    ref_count = int(np.sum(ref_ink))
    test_count = int(np.sum(test_ink))
    intersection = int(np.sum(ref_ink & test_ink))
    union = int(np.sum(ref_ink | test_ink))

    iou = intersection / union if union > 0 else 1.0

    precision = intersection / test_count if test_count > 0 else 0.0
    recall = intersection / ref_count if ref_count > 0 else 0.0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0

    # Aspect ratio similarity (of cropped content)
    ref_aspect = ref_crop.shape[1] / ref_crop.shape[0] if ref_crop.shape[0] > 0 else 1.0
    test_aspect = test_crop.shape[1] / test_crop.shape[0] if test_crop.shape[0] > 0 else 1.0
    aspect_sim = min(ref_aspect, test_aspect) / max(ref_aspect, test_aspect) if max(ref_aspect, test_aspect) > 0 else 0.0

    # Width ratio after normalization to same height
    width_sim = min(rw, tw) / max(rw, tw) if max(rw, tw) > 0 else 0.0

    # Combined score
    score = 0.4 * iou + 0.2 * recall + 0.2 * aspect_sim + 0.2 * width_sim

    return {
        "iou": iou,
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "aspect_sim": aspect_sim,
        "width_sim": width_sim,
        "score": score,
        "ref_ink_px": ref_count,
        "test_ink_px": test_count,
        "size_ref": (ref_img.shape[1], ref_img.shape[0]),
        "size_test": (test_img.shape[1], test_img.shape[0]),
        "crop_ref": (ref_crop.shape[1], ref_crop.shape[0]),
        "crop_test": (test_crop.shape[1], test_crop.shape[0]),
    }


def save_diff_image(ref_img, test_img, diff_path):
    """Save side-by-side: ref | test | diff. All normalized to same height."""
    ref_crop = crop_to_content(ref_img)
    test_crop = crop_to_content(test_img)
    ref_norm = normalize_size(ref_crop)
    test_norm = normalize_size(test_crop)

    # Pad to same width
    w = max(ref_norm.shape[1], test_norm.shape[1])
    def pad_w(img, target_w):
        h, cur_w = img.shape[:2]
        padded = np.full((h, target_w, 3), 255, dtype=np.uint8)
        padded[:, :min(cur_w, target_w)] = img[:, :min(cur_w, target_w)]
        return padded

    ref_final = pad_w(ref_norm, w)
    test_final = pad_w(test_norm, w)

    # Diff visualization
    ref_ink = get_ink_mask(ref_final)
    test_ink = get_ink_mask(test_final)
    vis = np.full_like(ref_final, 255)
    vis[ref_ink & test_ink] = [0, 0, 0]
    vis[ref_ink & ~test_ink] = [0, 200, 0]
    vis[~ref_ink & test_ink] = [200, 0, 0]

    # Side-by-side: ref | gap | test | gap | diff
    gap = np.full((NORM_HEIGHT, 4, 3), 200, dtype=np.uint8)
    combined = np.hstack([ref_final, gap, test_final, gap, vis])
    Image.fromarray(combined, "RGB").save(diff_path)


def main():
    parser = argparse.ArgumentParser(description="Golden test comparison (ink-based)")
    parser.add_argument(
        "--ce",
        "--mhchem",
        action="store_true",
        dest="ce",
        help="mhchem suite: fixtures_ce vs output_ce (implies --test-cases test_case_ce.txt)",
    )
    parser.add_argument("--fixtures", default=None)
    parser.add_argument("--output", default=None)
    parser.add_argument("--threshold", type=float, default=0.30, help="Combined score threshold to pass")
    parser.add_argument("--diff-dir", default=None)
    parser.add_argument("--test-cases", default=None)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    golden = repo_root / "tests" / "golden"

    if args.ce:
        args.fixtures = str(golden / "fixtures_ce")
        args.output = str(golden / "output_ce")
        args.test_cases = str(golden / "test_case_ce.txt")
    else:
        if args.fixtures is None:
            args.fixtures = str(golden / "fixtures")
        if args.output is None:
            args.output = str(golden / "output")
        if args.test_cases is None:
            args.test_cases = str(golden / "test_cases.txt")

    test_lines = []
    if os.path.exists(args.test_cases):
        with open(args.test_cases) as f:
            test_lines = [l.strip() for l in f if l.strip() and not l.strip().startswith('#')]

    if args.diff_dir:
        os.makedirs(args.diff_dir, exist_ok=True)

    fixtures = sorted(Path(args.fixtures).glob("*.png"))
    outputs = sorted(Path(args.output).glob("*.png"))

    fixture_map = {p.stem: p for p in fixtures}
    output_map = {p.stem: p for p in outputs}
    common = sorted(set(fixture_map) & set(output_map))

    if not common:
        print("No matching PNGs found!")
        sys.exit(1)

    passed = 0
    failed = 0
    scores = []
    results = []

    for name in common:
        idx = int(name) - 1
        formula = test_lines[idx] if idx < len(test_lines) else name

        ref_img = load_image(str(fixture_map[name]))
        test_img = load_image(str(output_map[name]))
        stats = compute_ink_metrics(ref_img, test_img)

        is_pass = stats["score"] >= args.threshold
        if is_pass:
            passed += 1
        else:
            failed += 1

        scores.append(stats["score"])
        results.append((name, formula, stats, is_pass))

        if args.diff_dir and not is_pass:
            save_diff_image(ref_img, test_img, os.path.join(args.diff_dir, f"{name}_diff.png"))

    total = passed + failed
    pass_rate = passed / total * 100 if total > 0 else 0
    avg_score = np.mean(scores) if scores else 0
    median_score = np.median(scores) if scores else 0

    print(f"\n{'='*65}")
    print(f"Golden Test (ink-based): {passed}/{total} passed ({pass_rate:.1f}%)")
    print(f"Score: avg={avg_score:.3f}  median={median_score:.3f}  min={min(scores):.3f}  max={max(scores):.3f}")
    print(f"{'='*65}")

    failures = [(n, f, s) for n, f, s, p in results if not p]
    if failures:
        print(f"\nFailed ({len(failures)}):")
        for name, formula, stats in failures[:20]:
            print(f"  {name}: score={stats['score']:.3f} iou={stats['iou']:.3f} "
                  f"recall={stats['recall']:.3f} aspect={stats['aspect_sim']:.2f} | {formula[:50]}")
        if len(failures) > 20:
            print(f"  ... and {len(failures) - 20} more")

    low_score = [(n, f, s) for n, f, s, _ in results if s["score"] < 0.5]
    if low_score:
        print(f"\nScore < 0.5 ({len(low_score)}):")
        for name, formula, stats in low_score[:20]:
            print(f"  {name}: score={stats['score']:.3f} iou={stats['iou']:.3f} "
                  f"recall={stats['recall']:.3f} aspect={stats['aspect_sim']:.2f} | {formula[:50]}")
        if len(low_score) > 20:
            print(f"  ... and {len(low_score) - 20} more")

    # Score distribution
    bins = [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.01]
    hist, _ = np.histogram(scores, bins=bins)
    print(f"\nScore distribution:")
    for i in range(len(hist)):
        bar = "█" * hist[i]
        print(f"  {bins[i]:.1f}-{bins[i+1]:.1f}: {hist[i]:3d} {bar}")

    print(f"\nThreshold: {args.threshold}")
    return 0 if pass_rate >= 90.0 else 1


if __name__ == "__main__":
    sys.exit(main())
