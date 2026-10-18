"""Regression tests for AOT/DevRuntime screenshot state normalization."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw


SCRIPT = Path(__file__).with_name("compare-render-parity.py")
SPEC = importlib.util.spec_from_file_location("render_parity_comparison", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {SCRIPT}")
COMPARISON = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPARISON)


def ios_node(label: str, y: int, height: int = 40) -> dict[str, object]:
    return {
        "role": "button",
        "label": label,
        "identifier": label,
        "selected": False,
        "enabled": True,
        "frame": {"x": 20, "y": y, "width": 180, "height": height},
    }


def android_node(label: str, y: int, height: int = 40) -> dict[str, object]:
    return {
        "role": "android.widget.Button",
        "label": label,
        "identifier": label,
        "selected": False,
        "enabled": True,
        "value": label,
        "checkable": False,
        "checked": False,
        "hittable": True,
        "frame": {"left": 20, "top": y, "right": 200, "bottom": y + height},
    }


class UniformScrollOffsetTests(unittest.TestCase):
    def test_ios_scroll_offset_keeps_logical_geometry_exact(self) -> None:
        aot = [ios_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        dev = [ios_node(f"row-{index}", 298 + index * 60) for index in range(4)]

        self.assertEqual(
            COMPARISON.uniform_scroll_offset(aot, dev, "style-input", 2400),
            (2, "pt"),
        )

    def test_android_uniform_scroll_is_normalized_in_pixels(self) -> None:
        aot = [android_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        dev = [android_node(f"row-{index}", 268 + index * 60) for index in range(4)]
        aot.append(android_node("toolbar", 88))
        dev.append(android_node("toolbar", 88))

        self.assertEqual(
            COMPARISON.uniform_scroll_offset(aot, dev, "style-button", 1568),
            (32, "px"),
        )

    def test_android_clipped_item_at_screen_edge_can_match_after_alignment(self) -> None:
        aot = [android_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        dev = [android_node(f"row-{index}", 280 + index * 60) for index in range(4)]
        aot.append(android_node("asset", 1540, 28))
        dev.append(android_node("asset", 1520, 48))

        self.assertEqual(
            COMPARISON.uniform_scroll_offset(aot, dev, "style-images", 1568),
            (20, "px"),
        )

    def test_android_rejects_nonuniform_scroll_or_middle_geometry_changes(self) -> None:
        aot = [android_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        nonuniform = [android_node(f"row-{index}", 268 + index * 60) for index in range(4)]
        nonuniform[2]["frame"]["top"] += 8  # type: ignore[index]
        nonuniform[2]["frame"]["bottom"] += 8  # type: ignore[index]
        self.assertIsNone(
            COMPARISON.uniform_scroll_offset(aot, nonuniform, "style-button", 1568)
        )

        changed_height = [android_node(f"row-{index}", 280 + index * 60) for index in range(4)]
        changed_height[2] = android_node("row-2", 400, 56)
        self.assertIsNone(
            COMPARISON.uniform_scroll_offset(aot, changed_height, "style-button", 1568)
        )

    def test_android_ignores_only_items_clipped_by_the_capture_edge(self) -> None:
        aot = [android_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        dev = [android_node(f"row-{index}", 332 + index * 60) for index in range(4)]
        dev.append(android_node("partially-visible-heading", 204, 28))
        self.assertEqual(
            COMPARISON.uniform_scroll_offset(aot, dev, "controls-scroll-5", 1568),
            (-32, "px"),
        )

        dev[-1] = android_node("missing-middle-row", 500, 28)
        self.assertIsNone(
            COMPARISON.uniform_scroll_offset(aot, dev, "controls-scroll-5", 1568)
        )

    def test_android_normalizes_children_clipped_by_the_app_scroll_viewport(self) -> None:
        aot = [android_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        dev = [android_node(f"row-{index}", 298 + index * 60) for index in range(4)]
        # These children touch the fixed title and bottom navigation bounds.
        # Compose reports only their visible frame, so a two-pixel scroll can
        # make their visible heights differ even with identical content.
        aot.extend((android_node("top-clipped", 204, 18), android_node("bottom-clipped", 1372, 36)))
        dev.extend((android_node("top-clipped", 204, 20), android_node("bottom-clipped", 1370, 38)))

        self.assertEqual(
            COMPARISON.uniform_scroll_offset(aot, dev, "controls-scroll-3", 1600),
            (2, "px"),
        )

    def test_android_allows_aot_child_clipped_after_scroll_alignment(self) -> None:
        aot = [android_node(f"row-{index}", 300 + index * 60) for index in range(4)]
        dev = [android_node(f"row-{index}", 311 + index * 60) for index in range(4)]
        # The AOT child is just below the app viewport; after applying the
        # -11px AOT-minus-Dev offset its DevRuntime counterpart is clipped too.
        aot.append(android_node("bottom-clipped", 1401, 7))

        self.assertEqual(
            COMPARISON.uniform_scroll_offset(aot, dev, "controls-scroll-3", 1600),
            (-11, "px"),
        )

        aot[-1] = android_node("missing-middle-row", 700, 28)
        self.assertIsNone(
            COMPARISON.uniform_scroll_offset(aot, dev, "controls-scroll-3", 1600)
        )


class OverlayScreenshotTests(unittest.TestCase):
    def make_overlay_pair(self, directory: Path) -> tuple[Path, Path]:
        width, height = 120, 220
        aot = Image.new("RGB", (width, height))
        dev = Image.new("RGB", (width, height))
        for y in range(height):
            for x in range(width):
                aot.putpixel((x, y), (y, y, y))
                dev.putpixel((x, y), (min(255, y + 4),) * 3)

        # Keep the modal fixed to the viewport while its background is scrolled.
        bounds = COMPARISON.OVERLAY_BOUNDS["dialog"]
        rect = (
            round(bounds[0] * width),
            round(bounds[1] * height),
            round(bounds[2] * width) - 1,
            round(bounds[3] * height) - 1,
        )
        ImageDraw.Draw(aot).rectangle(rect, fill=(48, 48, 48))
        ImageDraw.Draw(dev).rectangle(rect, fill=(48, 48, 48))

        aot_path = directory / "aot.png"
        dev_path = directory / "dev.png"
        aot.save(aot_path)
        dev.save(dev_path)
        return aot_path, dev_path

    def test_overlay_comparison_aligns_only_scrolled_background(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            aot, dev = self.make_overlay_pair(Path(scratch))
            overlay_mae, overlay_changed, background_mae, background_changed = (
                COMPARISON.compare_overlay_screenshots(aot, dev, "dialog", 4)
            )

        self.assertEqual((overlay_mae, overlay_changed), (0, 0))
        self.assertEqual((background_mae, background_changed), (0, 0))

    def test_overlay_comparison_rejects_a_modal_surface_regression(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            aot, dev = self.make_overlay_pair(Path(scratch))
            with Image.open(dev) as image:
                changed = image.convert("RGB")
            bounds = COMPARISON.OVERLAY_BOUNDS["dialog"]
            rect = (
                round(bounds[0] * changed.width),
                round(bounds[1] * changed.height),
                round(bounds[2] * changed.width) - 1,
                round(bounds[3] * changed.height) - 1,
            )
            ImageDraw.Draw(changed).rectangle(rect, fill=(180, 30, 30))
            changed.save(dev)

            with self.assertRaisesRegex(AssertionError, "modal surface mismatch"):
                COMPARISON.compare_overlay_screenshots(aot, dev, "dialog", 4)

    def test_overlay_comparison_rejects_a_dimmed_background_regression(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            aot, dev = self.make_overlay_pair(Path(scratch))
            with Image.open(dev) as image:
                changed = image.convert("RGB")
            ImageDraw.Draw(changed).rectangle((5, 45, 15, 55), fill=(240, 20, 20))
            changed.save(dev)

            with self.assertRaisesRegex(AssertionError, "dimmed background mismatch"):
                COMPARISON.compare_overlay_screenshots(aot, dev, "dialog", 4)


if __name__ == "__main__":
    unittest.main()
