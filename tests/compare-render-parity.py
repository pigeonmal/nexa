#!/usr/bin/env python3
"""Compare same-platform AOT and DevRuntime accessibility and screenshot captures."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

try:
    from PIL import Image, ImageChops, ImageStat
except ImportError as error:
    raise SystemExit("Install screenshot comparison dependencies with `python3 -m pip install -r tests/requirements-render-parity.txt`.") from error

SCROLL_PHASES = {
    "controls-scroll-3",
    "controls-scroll-5",
    "list-parameters-next",
    "lists-scroll-1",
    "lists-scroll-3",
    "style-button",
    "style-button-interaction",
    "style-images",
    "style-input",
    "style-pressable",
    "style-typography",
}


def canonical_accessibility(path: Path) -> list[object]:
    data = json.loads(path.read_text())
    if not isinstance(data, list):
        raise ValueError(f"{path} must contain an accessibility list")

    def stable_key(node: object) -> tuple[object, ...]:
        if not isinstance(node, dict):
            return (str(node),)
        frame = node.get("frame")
        frame = frame if isinstance(frame, dict) else {}
        return (
            node.get("role", ""),
            node.get("label", ""),
            node.get("identifier", ""),
            node.get("selected", False),
            node.get("enabled", False),
            frame.get("x", frame.get("left", 0)),
            frame.get("width", frame.get("right", 0)),
            frame.get("height", frame.get("bottom", 0)),
            frame.get("y", frame.get("top", 0)),
        )

    return sorted(data, key=stable_key)


def uniform_scroll_offset(aot: list[object], dev: list[object], phase: str) -> float | None:
    """Accept a small, uniform iOS scroll-position delta on scroll checkpoints."""
    if phase not in SCROLL_PHASES or len(aot) != len(dev):
        return None

    deltas = []
    for left, right in zip(aot, dev):
        if not isinstance(left, dict) or not isinstance(right, dict):
            return None
        left = dict(left)
        right = dict(right)
        left_frame = left.pop("frame", None)
        right_frame = right.pop("frame", None)
        if left != right or not isinstance(left_frame, dict) or not isinstance(right_frame, dict):
            return None
        if set(left_frame) != {"x", "y", "width", "height"} or left_frame.keys() != right_frame.keys():
            return None
        if any(left_frame[key] != right_frame[key] for key in ("x", "width", "height")):
            return None
        deltas.append(round(left_frame["y"] - right_frame["y"], 1))

    if not deltas:
        return None
    moving = [delta for delta in deltas if abs(delta) > 0.1]
    if len(moving) < 3 or len(moving) * 2 < len(deltas):
        return None
    offset = sorted(moving)[len(moving) // 2]
    if abs(offset) > 4.0 or any(abs(delta - offset) > 0.1 for delta in moving):
        return None
    return offset


def pixels_per_point(accessibility: list[object], screenshot_width: int) -> float:
    frames = [node.get("frame") for node in accessibility if isinstance(node, dict)]
    frames = [frame for frame in frames if isinstance(frame, dict) and "x" in frame and "width" in frame]
    if not frames:
        return 1.0
    logical_width = max(frame["x"] + frame["width"] for frame in frames) + min(frame["x"] for frame in frames)
    return screenshot_width / logical_width if logical_width > 0 else 1.0


def android_accessibility(xml_path: Path, json_path: Path) -> None:
    import xml.etree.ElementTree as element_tree

    root = element_tree.parse(xml_path).getroot()
    entries = []
    for node in root.iter("node"):
        attrs = node.attrib
        label = attrs.get("text") or attrs.get("content-desc") or ""
        clickable = attrs.get("clickable") == "true"
        checkable = attrs.get("checkable") == "true"
        if not label and not clickable and not checkable:
            continue
        values = [int(value) for value in re.findall(r"-?\d+", attrs.get("bounds", ""))]
        if len(values) != 4:
            continue
        entries.append(
            {
                "role": attrs.get("class", ""),
                "label": label,
                "identifier": attrs.get("resource-id", ""),
                "value": attrs.get("text", ""),
                "enabled": attrs.get("enabled") == "true",
                "hittable": clickable,
                "selected": attrs.get("selected") == "true",
                "checkable": checkable,
                "checked": attrs.get("checked") == "true",
                "frame": {"left": values[0], "top": values[1], "right": values[2], "bottom": values[3]},
            }
        )
    json_path.write_text(json.dumps(entries, sort_keys=True, indent=2) + "\n")


def compare_screenshots(aot: Path, dev: Path, vertical_offset_points: float = 0.0, scale: float = 1.0) -> tuple[float, float]:
    with Image.open(aot) as source:
        image_a = source.convert("RGB")
    with Image.open(dev) as source:
        image_d = source.convert("RGB")
    if image_a.size != image_d.size:
        raise ValueError(f"screenshot dimensions differ: AOT={image_a.size}, DevRuntime={image_d.size}")

    height = image_a.height
    # Mask the OS-owned status and navigation areas before comparing pixels.
    # When a scroll checkpoint settles a few points apart, align the content
    # before comparing it. Widen the masked edge band to avoid comparing the
    # shifted content at crop boundaries; app-owned controls remain checked by
    # the accessibility snapshot and by unshifted captures.
    mask_fraction = 0.12 if vertical_offset_points else 0.07
    first_row = round(height * mask_fraction)
    last_row = height - first_row
    if vertical_offset_points:
        image_d = ImageChops.offset(image_d, 0, round(vertical_offset_points * scale))
    difference = ImageChops.difference(
        image_a.crop((0, first_row, image_a.width, last_row)),
        image_d.crop((0, first_row, image_d.width, last_row)),
    )
    mean_absolute_error = sum(ImageStat.Stat(difference).mean) / 3
    red, green, blue = difference.split()
    maximum = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    changed = maximum.point(lambda value: 255 if value > 8 else 0)
    changed_pixel_percent = changed.histogram()[255] * 100 / (image_a.width * (last_row - first_row))
    if mean_absolute_error > 1.0 or changed_pixel_percent > 0.5:
        raise AssertionError(
            f"screenshot mismatch: MAE={mean_absolute_error:.3f}, "
            f"pixels over 8 levels={changed_pixel_percent:.3f}%"
        )
    return mean_absolute_error, changed_pixel_percent


def main() -> int:
    if len(sys.argv) == 4 and sys.argv[1] == "--android-xml":
        android_accessibility(Path(sys.argv[2]), Path(sys.argv[3]))
        return 0
    if len(sys.argv) != 3:
        print("usage: compare-render-parity.py <aot-captures> <dev-captures> | --android-xml <tree.xml> <out.json>", file=sys.stderr)
        return 2
    aot, dev = map(Path, sys.argv[1:])
    failures = 0
    aot_phases = {path.stem for path in aot.glob("*.json")}
    dev_phases = {path.stem for path in dev.glob("*.json")}
    if not aot_phases:
        print(f"no AOT accessibility captures found in {aot}", file=sys.stderr)
        return 2
    if aot_phases != dev_phases:
        print(
            f"capture phases differ: AOT-only={sorted(aot_phases - dev_phases)}, "
            f"Dev-only={sorted(dev_phases - aot_phases)}",
            file=sys.stderr,
        )
        return 1
    for phase in sorted(aot_phases):
        aot_accessibility = canonical_accessibility(aot / f"{phase}.json")
        dev_accessibility = canonical_accessibility(dev / f"{phase}.json")
        vertical_offset = 0.0
        if aot_accessibility != dev_accessibility:
            vertical_offset = uniform_scroll_offset(aot_accessibility, dev_accessibility, phase)
            if vertical_offset is not None:
                print(f"{phase}: accessibility matches with {vertical_offset:+.1f}pt content scroll offset")
            else:
                failures += 1
                print(f"{phase}: accessibility roles, labels, actions, or frames differ", file=sys.stderr)
                for index, (left, right) in enumerate(zip(aot_accessibility, dev_accessibility)):
                    if left != right:
                        print(f"  entry {index}: AOT={left!r}; DevRuntime={right!r}", file=sys.stderr)
                        if index >= 9:
                            break
                if len(aot_accessibility) != len(dev_accessibility):
                    print(f"  node count: AOT={len(aot_accessibility)}, DevRuntime={len(dev_accessibility)}", file=sys.stderr)
        else:
            print(f"{phase}: accessibility roles, labels, actions, and frames match")
        try:
            scale = pixels_per_point(aot_accessibility, Image.open(aot / f"{phase}.png").width)
            mae, changed = compare_screenshots(aot / f"{phase}.png", dev / f"{phase}.png", vertical_offset, scale)
            print(f"{phase}: screenshot matches (MAE={mae:.3f}, changed pixels={changed:.3f}%)")
        except (AssertionError, OSError, ValueError) as error:
            failures += 1
            print(f"{phase}: {error}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
