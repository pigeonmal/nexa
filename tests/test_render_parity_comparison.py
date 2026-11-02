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


class AppOwnedEdgeComparisonTests(unittest.TestCase):
    def make_edge_images(
        self,
        directory: Path,
        mutate_edge: str | None = None,
    ) -> tuple[Path, Path]:
        width, height = 200, 1000
        # White background for both
        aot = Image.new("RGB", (width, height), (255, 255, 255))
        dev = Image.new("RGB", (width, height), (255, 255, 255))

        # Top header at y=20..60 (inside the 7% top band which is y=0..70)
        ImageDraw.Draw(aot).rectangle((10, 20, 190, 60), fill=(50, 50, 200))
        dev_header_color = (200, 50, 50) if mutate_edge == "header" else (50, 50, 200)
        ImageDraw.Draw(dev).rectangle((10, 20, 190, 60), fill=dev_header_color)

        # Bottom bar at y=950..990 (inside the 7% bottom band which is y=930..1000)
        ImageDraw.Draw(aot).rectangle((10, 950, 190, 990), fill=(20, 150, 50))
        dev_footer_color = (220, 20, 20) if mutate_edge == "footer" else (20, 150, 50)
        ImageDraw.Draw(dev).rectangle((10, 950, 190, 990), fill=dev_footer_color)

        aot_path = directory / "aot.png"
        dev_path = directory / "dev.png"
        aot.save(aot_path)
        dev.save(dev_path)
        return aot_path, dev_path

    def test_bottom_bar_visual_mismatch_detected_despite_generic_mask(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            scratch_path = Path(scratch)
            aot_path, dev_path = self.make_edge_images(scratch_path, mutate_edge="footer")

            # Without accessibility, the generic 7% edge mask would ignore the footer
            mae, changed = COMPARISON.compare_screenshots(aot_path, dev_path)
            self.assertEqual((mae, changed), (0.0, 0.0))

            # With accessibility specifying the bottom bar node, the app-owned edge check FAILS!
            aot_a11y = [
                android_node("AppBottomBar", 950, 40)
            ]
            with self.assertRaisesRegex(AssertionError, "app-owned edge mismatch in footer 'AppBottomBar'"):
                COMPARISON.compare_screenshots(
                    aot_path, dev_path, aot_accessibility=aot_a11y
                )

    def test_header_visual_mismatch_detected_despite_generic_mask(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            scratch_path = Path(scratch)
            aot_path, dev_path = self.make_edge_images(scratch_path, mutate_edge="header")

            # With accessibility specifying the header node, the app-owned edge check FAILS!
            aot_a11y = [
                android_node("AppHeader", 20, 40)
            ]
            with self.assertRaisesRegex(AssertionError, "app-owned edge mismatch in header 'AppHeader'"):
                COMPARISON.compare_screenshots(
                    aot_path, dev_path, aot_accessibility=aot_a11y
                )

    def test_unmasked_diff_artifact_generation_on_failure(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            scratch_path = Path(scratch)
            aot_path, dev_path = self.make_edge_images(scratch_path, mutate_edge="footer")
            diff_dir = scratch_path / "artifacts"
            aot_a11y = [android_node("AppBottomBar", 950, 40)]

            try:
                COMPARISON.compare_screenshots(
                    aot_path, dev_path, aot_accessibility=aot_a11y, output_diff_dir=diff_dir, phase="footer_test"
                )
            except AssertionError:
                pass

            self.assertTrue((diff_dir / "footer_test_unmasked_diff.png").exists())
            self.assertTrue((diff_dir / "footer_test_masked_diff.png").exists())
            self.assertTrue((diff_dir / "footer_test_diff_manifest.json").exists())


class AccessibilityTreeMutationTests(unittest.TestCase):
    def test_traversal_order_mutation_detected(self) -> None:
        aot = [
            android_node("buttonA", 100),
            android_node("buttonB", 200),
        ]
        dev_swapped = [
            android_node("buttonB", 200),
            android_node("buttonA", 100),
        ]
        discrepancies = COMPARISON.verify_accessibility_tree(aot, dev_swapped, check_traversal_order=True)
        self.assertTrue(len(discrepancies) > 0)
        self.assertIn("traversal order mismatch", discrepancies[0])

    def test_hit_target_or_enabled_mutation_detected(self) -> None:
        aot = [android_node("submit", 100)]
        dev_disabled = [android_node("submit", 100)]
        dev_disabled[0]["enabled"] = False  # type: ignore[index]

        discrepancies = COMPARISON.verify_accessibility_tree(aot, dev_disabled)
        self.assertTrue(len(discrepancies) > 0)
        self.assertIn("property 'enabled' differs", discrepancies[0])

        dev_unhittable = [android_node("submit", 100)]
        dev_unhittable[0]["hittable"] = False  # type: ignore[index]
        discrepancies = COMPARISON.verify_accessibility_tree(aot, dev_unhittable)
        self.assertTrue(len(discrepancies) > 0)
        self.assertIn("property 'hittable' differs", discrepancies[0])

    def test_header_4dp_offset_mutation_detected(self) -> None:
        aot = [android_node("header", 20, 40)]
        dev_shifted = [android_node("header", 24, 40)]  # 4px/dp offset
        discrepancies = COMPARISON.verify_accessibility_tree(aot, dev_shifted)
        self.assertTrue(len(discrepancies) > 0)
        self.assertIn("frame geometry differs", discrepancies[0])


class DynamicOverlayBoundsTests(unittest.TestCase):
    def test_dynamic_overlay_bounds_derived_from_accessibility(self) -> None:
        a11y = [
            {
                "role": "dialog",
                "label": "CustomDialog",
                "identifier": "dialog_container",
                "frame": {"left": 40, "top": 120, "right": 360, "bottom": 720},
            }
        ]
        surface, interior = COMPARISON.derive_overlay_bounds(a11y, "dialog", 400, 800)
        self.assertEqual(surface, (40, 120, 360, 720))
        # Interior is inset by ~5%
        self.assertTrue(interior[0] > surface[0])
        self.assertTrue(interior[1] > surface[1])
        self.assertTrue(interior[2] < surface[2])
        self.assertTrue(interior[3] < surface[3])

    def test_dynamic_overlay_bounds_falls_back_when_no_modal_nodes(self) -> None:
        a11y = [
            android_node("RegularButton", 300)
        ]
        surface, interior = COMPARISON.derive_overlay_bounds(a11y, "dialog", 400, 800)
        # Should match static OVERLAY_BOUNDS["dialog"]
        expected_surface = (
            round(COMPARISON.OVERLAY_BOUNDS["dialog"][0] * 400),
            round(COMPARISON.OVERLAY_BOUNDS["dialog"][1] * 800),
            round(COMPARISON.OVERLAY_BOUNDS["dialog"][2] * 400),
            round(COMPARISON.OVERLAY_BOUNDS["dialog"][3] * 800),
        )
        self.assertEqual(surface, expected_surface)


if __name__ == "__main__":
    unittest.main()
