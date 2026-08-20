from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw

TOOL_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOL_DIR))

import check_blank_geometry as blank_geometry  # noqa: E402


def write_blank(path: Path, size: tuple[int, int]) -> None:
    Image.new("RGB", size, "white").save(path)


def write_ink(path: Path, size: tuple[int, int]) -> None:
    image = Image.new("RGB", size, "white")
    draw = ImageDraw.Draw(image)
    draw.rectangle((1, 1, 3, 3), fill="black")
    image.save(path)


class BlankGeometryTests(unittest.TestCase):
    def test_blank_size_regression_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = root / "base"
            current = root / "current"
            base.mkdir()
            current.mkdir()
            (root / "base.txt").write_text("\\allowbreak\n", encoding="utf-8")
            (root / "current.txt").write_text("\\allowbreak\n", encoding="utf-8")
            write_blank(base / "0001.png", (20, 20))
            write_blank(current / "0001.png", (30, 20))

            regressions = blank_geometry.compare_blank_geometry(
                root / "base.txt",
                base,
                root / "current.txt",
                current,
                0.05,
            )
            self.assertEqual(len(regressions), 1)
            self.assertEqual(regressions[0]["baseline_size"], [20, 20])
            self.assertEqual(regressions[0]["current_size"], [30, 20])

    def test_identical_blank_geometry_passes(self) -> None:
        self.assertEqual(blank_geometry.size_regression((20, 20), (20, 20)), 0.0)

    def test_nonblank_case_is_left_to_ink_metric(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = root / "base"
            current = root / "current"
            base.mkdir()
            current.mkdir()
            (root / "base.txt").write_text("x\n", encoding="utf-8")
            (root / "current.txt").write_text("x\n", encoding="utf-8")
            write_ink(base / "0001.png", (20, 20))
            write_ink(current / "0001.png", (40, 20))

            regressions = blank_geometry.compare_blank_geometry(
                root / "base.txt",
                base,
                root / "current.txt",
                current,
                0.05,
            )
            self.assertEqual(regressions, [])


if __name__ == "__main__":
    unittest.main()
