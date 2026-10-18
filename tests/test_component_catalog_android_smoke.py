"""Pixel-level assertions for platform chrome checked in the Android catalog."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw


SCRIPT = Path(__file__).with_name("component-catalog-android-smoke.py")
SPEC = importlib.util.spec_from_file_location("component_catalog_android_smoke", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {SCRIPT}")
SMOKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SMOKE)


class AndroidStatusBarContrastTests(unittest.TestCase):
    def test_rejects_white_status_content_on_a_light_surface(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            screenshot = Path(scratch) / "light-status.png"
            Image.new("RGB", (100, 100), (242, 242, 247)).save(screenshot)

            with self.assertRaisesRegex(RuntimeError, "no dark status-bar foreground"):
                SMOKE.assert_dark_status_bar_foreground(screenshot)

    def test_accepts_dark_status_content_on_a_light_surface(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            screenshot = Path(scratch) / "dark-status.png"
            image = Image.new("RGB", (100, 100), (242, 242, 247))
            ImageDraw.Draw(image).rectangle((2, 1, 12, 3), fill=(24, 24, 24))
            image.save(screenshot)

            SMOKE.assert_dark_status_bar_foreground(screenshot)


class AndroidSwitchVisualTests(unittest.TestCase):
    def test_counts_only_pixels_that_change_inside_switch_bounds(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            before = Path(scratch) / "before.png"
            after = Path(scratch) / "after.png"
            Image.new("RGB", (32, 24), (255, 255, 255)).save(before)
            current = Image.new("RGB", (32, 24), (255, 255, 255))
            ImageDraw.Draw(current).rectangle((10, 8, 15, 15), fill=(0, 122, 255))
            current.save(after)

            self.assertEqual(SMOKE.changed_pixel_count(before, after, (0, 0, 24, 20)), 48)
            self.assertEqual(SMOKE.changed_pixel_count(before, after, (0, 0, 8, 20)), 0)


if __name__ == "__main__":
    unittest.main()
