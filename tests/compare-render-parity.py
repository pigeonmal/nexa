#!/usr/bin/env python3
"""Compare same-platform AOT and DevRuntime accessibility and screenshot captures."""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

try:
    from PIL import Image, ImageChops, ImageDraw, ImageStat
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
    "pressable-long",
    "bottom-sheet-underlay",
    "dialog-underlay",
    "confirmation-underlay",
}

OVERLAY_UNDERLAY_PHASES = {
    "bottom-sheet": "bottom-sheet-underlay",
    "dialog": "dialog-underlay",
    "confirmation": "confirmation-underlay",
}

OVERLAY_BOUNDS = {
    # Outer bounds include rounded corners and shadows so background masking
    # never mistakes content behind an overlay for part of its surface.
    "bottom-sheet": (0.0, 0.475, 1.0, 0.93),
    "dialog": (0.055, 0.375, 0.945, 0.65),
    "confirmation": (0.055, 0.408, 0.945, 0.622),
}

OVERLAY_INTERIOR_BOUNDS = {
    # The Android dialog's outer bounds include rounded corners and the
    # dimmed content behind them. Compare opaque dialog content here; the
    # dimmed content is compared against the aligned underlay separately.
    "bottom-sheet": (0.04, 0.53, 0.96, 0.91),
    "dialog": (0.167, 0.4125, 0.833, 0.6125),
    "confirmation": (0.055, 0.408, 0.945, 0.622),
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


def load_raw_accessibility(path: Path) -> list[object]:
    data = json.loads(path.read_text())
    if not isinstance(data, list):
        raise ValueError(f"{path} must contain an accessibility list")
    return data


def verify_accessibility_tree(
    aot: list[object],
    dev: list[object],
    check_traversal_order: bool = True,
) -> list[str]:
    """Verify accessibility tree nodes, including traversal order, roles, actions, hit targets and geometry."""
    discrepancies: list[str] = []
    if len(aot) != len(dev):
        discrepancies.append(f"node count differs: AOT={len(aot)}, DevRuntime={len(dev)}")
        return discrepancies

    for index, (left, right) in enumerate(zip(aot, dev)):
        if not isinstance(left, dict) or not isinstance(right, dict):
            if left != right:
                discrepancies.append(f"node {index} differs: AOT={left!r}, DevRuntime={right!r}")
            continue

        label_a = str(left.get("label", ""))
        label_d = str(right.get("label", ""))
        role_a = str(left.get("role", ""))
        role_d = str(right.get("role", ""))
        if check_traversal_order and (label_a != label_d or role_a != role_d):
            discrepancies.append(
                f"node {index} traversal order mismatch: AOT=({role_a}, {label_a}), DevRuntime=({role_d}, {label_d})"
            )
            continue

        # Check interaction properties and hit targets
        for prop in ("enabled", "hittable", "selected", "checkable", "checked"):
            val_a = left.get(prop)
            val_d = right.get(prop)
            if val_a is not None and val_d is not None and val_a != val_d:
                discrepancies.append(
                    f"node {index} ('{label_a}') property '{prop}' differs: AOT={val_a}, DevRuntime={val_d}"
                )

        # Check geometry
        frame_a = left.get("frame")
        frame_d = right.get("frame")
        if isinstance(frame_a, dict) and isinstance(frame_d, dict):
            geom_a = _frame_geometry(frame_a)
            geom_d = _frame_geometry(frame_d)
            if geom_a and geom_d:
                x_a, y_a, w_a, h_a, unit_a = geom_a
                x_d, y_d, w_d, h_d, unit_d = geom_d
                if (x_a, y_a, w_a, h_a) != (x_d, y_d, w_d, h_d):
                    discrepancies.append(
                        f"node {index} ('{label_a}') frame geometry differs: "
                        f"AOT=({x_a}, {y_a}, {w_a}, {h_a} {unit_a}), DevRuntime=({x_d}, {y_d}, {w_d}, {h_d} {unit_d})"
                    )
    return discrepancies


def extract_app_owned_edge_regions(
    accessibility: list[object],
    image_width: int,
    image_height: int,
    mask_fraction: float = 0.07,
    scale: float = 1.0,
) -> list[dict[str, object]]:
    """Find bounding boxes of app-owned elements that fall inside the top or bottom edge mask bands."""
    top_edge = round(image_height * mask_fraction)
    bottom_edge = image_height - top_edge
    app_regions: list[dict[str, object]] = []

    for node in accessibility:
        if not isinstance(node, dict):
            continue
        frame = node.get("frame")
        if not isinstance(frame, dict):
            continue
        geom = _frame_geometry(frame)
        if geom is None:
            continue
        x, y, w, h, unit = geom
        node_scale = 1.0 if unit == "px" else scale
        left = max(0, round(x * node_scale))
        top = max(0, round(y * node_scale))
        right = min(image_width, round((x + w) * node_scale))
        bottom = min(image_height, round((y + h) * node_scale))
        if right <= left or bottom <= top:
            continue

        intersects_top = top < top_edge and bottom > 0
        intersects_bottom = bottom > bottom_edge and top < image_height

        if intersects_top or intersects_bottom:
            label = str(node.get("label") or node.get("identifier") or node.get("role") or "app_edge_element")
            edge_type = "header" if intersects_top else "footer"
            app_regions.append({
                "label": label,
                "role": str(node.get("role", "")),
                "edge": edge_type,
                "rect": (left, top, right, bottom),
                "node": node,
            })
    return app_regions


def derive_overlay_bounds(
    accessibility: list[object] | None,
    phase: str,
    image_width: int,
    image_height: int,
    scale: float = 1.0,
) -> tuple[tuple[int, int, int, int], tuple[int, int, int, int]]:
    """Derive outer and interior pixel bounds for an overlay from accessibility or fallback to constants."""
    if accessibility:
        modal_frames: list[tuple[float, float, float, float, str]] = []
        for node in accessibility:
            if not isinstance(node, dict):
                continue
            role = str(node.get("role", "")).lower()
            ident = str(node.get("identifier", "")).lower()
            label = str(node.get("label", "")).lower()
            if any(term in role or term in ident or term in label for term in ("dialog", "sheet", "alert", "modal", "confirmation")):
                frame = node.get("frame")
                if isinstance(frame, dict):
                    geom = _frame_geometry(frame)
                    if geom:
                        modal_frames.append(geom)

        if modal_frames:
            units = {geom[4] for geom in modal_frames}
            unit = next(iter(units))
            s = 1.0 if unit == "px" else scale
            min_x = min(round(geom[0] * s) for geom in modal_frames)
            min_y = min(round(geom[1] * s) for geom in modal_frames)
            max_x = max(round((geom[0] + geom[2]) * s) for geom in modal_frames)
            max_y = max(round((geom[1] + geom[3]) * s) for geom in modal_frames)

            min_x = max(0, min_x)
            min_y = max(0, min_y)
            max_x = min(image_width, max_x)
            max_y = min(image_height, max_y)
            if max_x > min_x and max_y > min_y:
                inset_x = max(1, round((max_x - min_x) * 0.05))
                inset_y = max(1, round((max_y - min_y) * 0.05))
                interior = (min_x + inset_x, min_y + inset_y, max_x - inset_x, max_y - inset_y)
                return (min_x, min_y, max_x, max_y), interior

    bounds = OVERLAY_BOUNDS.get(phase, (0.055, 0.375, 0.945, 0.65))
    interior_bounds = OVERLAY_INTERIOR_BOUNDS.get(phase, bounds)
    surface = (
        round(bounds[0] * image_width),
        round(bounds[1] * image_height),
        round(bounds[2] * image_width),
        round(bounds[3] * image_height),
    )
    interior = (
        round(interior_bounds[0] * image_width),
        round(interior_bounds[1] * image_height),
        round(interior_bounds[2] * image_width),
        round(interior_bounds[3] * image_height),
    )
    return surface, interior


def generate_diff_artifacts(
    image_a: Image.Image,
    image_d: Image.Image,
    output_dir: Path,
    phase: str,
    excluded_regions: list[dict[str, object]] | None = None,
    app_owned_regions: list[dict[str, object]] | None = None,
    mae: float = 0.0,
    changed_percent: float = 0.0,
    status: str = "pass",
) -> Path:
    """Save unmasked diff, masked diff, and diff manifest artifacts for triage."""
    output_dir.mkdir(parents=True, exist_ok=True)
    unmasked_diff = ImageChops.difference(image_a, image_d)
    unmasked_path = output_dir / f"{phase}_unmasked_diff.png"
    unmasked_diff.save(unmasked_path)

    masked_diff = unmasked_diff.copy()
    draw = ImageDraw.Draw(masked_diff)
    if excluded_regions:
        for region in excluded_regions:
            rect = region.get("rect")
            if rect and len(rect) == 4:
                draw.rectangle(rect, outline=(128, 128, 128), fill=(40, 40, 40))
    masked_path = output_dir / f"{phase}_masked_diff.png"
    masked_diff.save(masked_path)

    manifest_data = {
        "phase": phase,
        "status": status,
        "mae": round(mae, 4),
        "changed_pixels_percent": round(changed_percent, 4),
        "dimensions": {"width": image_a.width, "height": image_a.height},
        "excluded_regions": excluded_regions or [],
        "app_owned_edge_regions": [
            {"label": r.get("label"), "edge": r.get("edge"), "rect": r.get("rect")}
            for r in (app_owned_regions or [])
        ],
    }
    manifest_path = output_dir / f"{phase}_diff_manifest.json"
    manifest_path.write_text(json.dumps(manifest_data, indent=2) + "\n")
    return manifest_path


def _frame_geometry(frame: dict[str, object]) -> tuple[float, float, float, float, str] | None:
    """Return x, y, width, height, and coordinate unit for a platform frame."""
    if set(frame) == {"x", "y", "width", "height"}:
        return (frame["x"], frame["y"], frame["width"], frame["height"], "pt")
    if set(frame) == {"left", "top", "right", "bottom"}:
        return (
            frame["left"],
            frame["top"],
            frame["right"] - frame["left"],
            frame["bottom"] - frame["top"],
            "px",
        )
    return None


def uniform_scroll_offset(
    aot: list[object], dev: list[object], phase: str, image_height: int
) -> tuple[float, str] | None:
    """Accept a uniform content scroll delta without hiding geometry changes."""
    if phase not in SCROLL_PHASES:
        return None

    def group_by_semantics(nodes: list[object]) -> dict[str, list[tuple[dict[str, object], tuple[float, float, float, float, str]]]] | None:
        groups: dict[str, list[tuple[dict[str, object], tuple[float, float, float, float, str]]]] = {}
        for node in nodes:
            if not isinstance(node, dict):
                return None
            frame = node.get("frame")
            if not isinstance(frame, dict):
                return None
            geometry = _frame_geometry(frame)
            if geometry is None:
                return None
            semantic = {key: value for key, value in node.items() if key != "frame"}
            signature = json.dumps(semantic, sort_keys=True, separators=(",", ":"))
            groups.setdefault(signature, []).append((semantic, geometry))
        for entries in groups.values():
            entries.sort(key=lambda entry: (entry[1][1], entry[1][0]))
        return groups

    aot_groups = group_by_semantics(aot)
    dev_groups = group_by_semantics(dev)
    if aot_groups is None or dev_groups is None:
        return None

    units = {
        geometry[4]
        for groups in (aot_groups, dev_groups)
        for entries in groups.values()
        for _, geometry in entries
    }
    if len(units) != 1:
        return None
    unit = next(iter(units))
    signatures = aot_groups.keys() | dev_groups.keys()
    paired: list[
        tuple[
            tuple[float, float, float, float, str],
            tuple[float, float, float, float, str],
        ]
    ] = []
    unmatched: list[tuple[str, tuple[float, float, float, float, str]]] = []
    for signature in signatures:
        left_entries = aot_groups.get(signature, [])
        right_entries = dev_groups.get(signature, [])
        shared_count = min(len(left_entries), len(right_entries))
        paired.extend(
            (left[1], right[1])
            for left, right in zip(left_entries[:shared_count], right_entries[:shared_count])
        )
        unmatched.extend(("AOT", entry[1]) for entry in left_entries[shared_count:])
        unmatched.extend(("DevRuntime", entry[1]) for entry in right_entries[shared_count:])

    if len(paired) < 3:
        return None
    if unit == "pt" and unmatched:
        return None

    deltas: list[float] = []
    for left_frame, right_frame in paired:
        left_x, left_y, left_width, _, left_unit = left_frame
        right_x, right_y, right_width, _, right_unit = right_frame
        if left_unit != right_unit or left_x != right_x or left_width != right_width:
            return None
        deltas.append(round(left_y - right_y, 1))

    # Scroll content is clipped by the app's own title and bottom-bar regions,
    # not only by the physical screenshot edges. A small alignment delta can
    # therefore change the reported accessibility height of a child at either
    # scroll viewport boundary even when its layout is unchanged.
    edge_top = round(image_height * 0.14)
    edge_bottom = round(image_height * 0.875)
    offset_candidates = paired
    if unit == "px":
        # A clipped edge child can report a partial height or a different
        # scroll delta from the rows below it. Infer the shared offset from
        # fully visible content and validate edge children separately below.
        offset_candidates = [
            (left_frame, right_frame)
            for left_frame, right_frame in paired
            if not (
                left_frame[1] < edge_top
                or right_frame[1] < edge_top
                or left_frame[1] + left_frame[3] >= edge_bottom
                or right_frame[1] + right_frame[3] >= edge_bottom
            )
        ]
    candidate_deltas = [
        round(left_frame[1] - right_frame[1], 1)
        for left_frame, right_frame in offset_candidates
    ]
    moving = [delta for delta in candidate_deltas if abs(delta) > 0.1]
    if len(moving) < 3 or len(moving) * 2 < len(candidate_deltas):
        return None
    offset = sorted(moving)[len(moving) // 2]
    maximum_offset = 4.0 if unit == "pt" else 64.0
    if abs(offset) > maximum_offset or any(abs(delta - offset) > 0.1 for delta in moving):
        return None

    for left_frame, right_frame in paired:
        left_x, left_y, left_width, left_height, _ = left_frame
        right_x, right_y, right_width, right_height, _ = right_frame
        delta = round(left_y - right_y, 1)
        if left_x != right_x or left_width != right_width:
            return None
        if abs(delta) <= 0.1:
            if (left_y, left_height) != (right_y, right_height):
                if unit != "px" or not (
                    left_y < edge_top
                    or right_y < edge_top
                    or left_y + left_height >= edge_bottom
                    or right_y + right_height >= edge_bottom
                ):
                    return None
            continue
        if abs(delta - offset) > 0.1:
            if unit != "px" or not (
                left_y < edge_top
                or right_y < edge_top
                or left_y + left_height >= edge_bottom
                or right_y + right_height >= edge_bottom
            ):
                return None
        if unit == "px":
            # Android exposes clipped bounds for children at the viewport
            # edges. Apply the candidate scroll to the Dev bounds, clip them
            # to the device screen, then require the remaining geometry to
            # equal the AOT tree exactly.
            aligned_top = max(0, round(right_y + offset))
            aligned_bottom = min(image_height, round(right_y + right_height + offset))
            visible_top = max(0, round(left_y))
            visible_bottom = min(image_height, round(left_y + left_height))
            if (aligned_top, aligned_bottom) != (visible_top, visible_bottom):
                if not (
                    visible_top < edge_top
                    or aligned_top < edge_top
                    or visible_bottom > edge_bottom
                    or aligned_bottom > edge_bottom
                ):
                    return None
        elif left_height != right_height:
            return None
    for side, (_, y, _, height, _) in unmatched:
        # `offset` is AOT.y - DevRuntime.y. Convert an unmatched item to the
        # other capture's coordinates with the inverse sign for AOT items.
        aligned_y = y - offset if side == "AOT" else y + offset
        if not (
            y < edge_top
            or y + height > edge_bottom
            or aligned_y < edge_top
            or aligned_y + height > edge_bottom
        ):
            return None

    return offset, unit


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


def compare_screenshots(
    aot: Path,
    dev: Path,
    vertical_offset_points: float = 0.0,
    scale: float = 1.0,
    aot_accessibility: list[object] | None = None,
    dev_accessibility: list[object] | None = None,
    output_diff_dir: Path | None = None,
    phase: str = "screenshot",
) -> tuple[float, float]:
    with Image.open(aot) as source:
        image_a = source.convert("RGB")
    with Image.open(dev) as source:
        image_d = source.convert("RGB")
    if image_a.size != image_d.size:
        raise ValueError(f"screenshot dimensions differ: AOT={image_a.size}, DevRuntime={image_d.size}")

    height = image_a.height
    width = image_a.width
    # Mask the OS-owned status and navigation areas before comparing pixels.
    # When a scroll checkpoint settles a few points apart, align the content
    # before comparing it. Widen the masked edge band to avoid comparing the
    # shifted content at crop boundaries; app-owned controls remain checked by
    # the accessibility snapshot and by unshifted captures.
    mask_fraction = (
        0.17 if vertical_offset_points and scale <= 1.0 else 0.12 if vertical_offset_points else 0.07
    )
    first_row = round(height * mask_fraction)
    last_row = height - first_row
    if vertical_offset_points:
        image_d = ImageChops.offset(image_d, 0, round(vertical_offset_points * scale))

    excluded_regions = [
        {"reason": "status_bar", "rect": [0, 0, width, first_row]},
        {"reason": "navigation_bar", "rect": [0, last_row, width, height]},
    ]

    difference = ImageChops.difference(
        image_a.crop((0, first_row, image_a.width, last_row)),
        image_d.crop((0, first_row, image_d.width, last_row)),
    )
    mean_absolute_error = sum(ImageStat.Stat(difference).mean) / 3
    red, green, blue = difference.split()
    maximum = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    changed = maximum.point(lambda value: 255 if value > 8 else 0)
    changed_pixel_percent = changed.histogram()[255] * 100 / (image_a.width * (last_row - first_row))

    # App-owned edge region check:
    # Ensure app-owned toolbars, headers, bottom bars in the masked edge bands
    # are checked directly rather than hidden by the generic edge mask.
    app_regions: list[dict[str, object]] = []
    if aot_accessibility:
        app_regions = extract_app_owned_edge_regions(
            aot_accessibility, width, height, mask_fraction=mask_fraction, scale=scale
        )
        for region in app_regions:
            rect = region["rect"]
            edge_crop_a = image_a.crop(rect)
            edge_crop_d = image_d.crop(rect)
            edge_diff = ImageChops.difference(edge_crop_a, edge_crop_d)
            edge_mae = sum(ImageStat.Stat(edge_diff).mean) / 3
            edge_r, edge_g, edge_b = edge_diff.split()
            edge_max = ImageChops.lighter(ImageChops.lighter(edge_r, edge_g), edge_b)
            edge_changed_img = edge_max.point(lambda v: 255 if v > 8 else 0)
            edge_pixels = (rect[2] - rect[0]) * (rect[3] - rect[1])
            edge_changed_percent = (
                edge_changed_img.histogram()[255] * 100 / edge_pixels if edge_pixels > 0 else 0.0
            )

            if edge_mae > 1.0 or edge_changed_percent > 0.5:
                if output_diff_dir or os.environ.get("NEXA_SAVE_DIFF_ARTIFACTS") == "1":
                    target_dir = output_diff_dir or Path(aot).parent / "diff_artifacts"
                    generate_diff_artifacts(
                        image_a, image_d, target_dir, phase,
                        excluded_regions=excluded_regions,
                        app_owned_regions=app_regions,
                        mae=edge_mae,
                        changed_percent=edge_changed_percent,
                        status="fail",
                    )
                raise AssertionError(
                    f"app-owned edge mismatch in {region['edge']} '{region['label']}': "
                    f"MAE={edge_mae:.3f}, pixels over 8 levels={edge_changed_percent:.3f}%"
                )

    if mean_absolute_error > 1.0 or changed_pixel_percent > 0.5:
        if output_diff_dir or os.environ.get("NEXA_SAVE_DIFF_ARTIFACTS") == "1":
            target_dir = output_diff_dir or Path(aot).parent / "diff_artifacts"
            generate_diff_artifacts(
                image_a, image_d, target_dir, phase,
                excluded_regions=excluded_regions,
                app_owned_regions=app_regions,
                mae=mean_absolute_error,
                changed_percent=changed_pixel_percent,
                status="fail",
            )
        raise AssertionError(
            f"screenshot mismatch: MAE={mean_absolute_error:.3f}, "
            f"pixels over 8 levels={changed_pixel_percent:.3f}%"
        )

    if output_diff_dir or os.environ.get("NEXA_SAVE_DIFF_ARTIFACTS") == "1":
        target_dir = output_diff_dir or Path(aot).parent / "diff_artifacts"
        generate_diff_artifacts(
            image_a, image_d, target_dir, phase,
            excluded_regions=excluded_regions,
            app_owned_regions=app_regions,
            mae=mean_absolute_error,
            changed_percent=changed_pixel_percent,
            status="pass",
        )

    return mean_absolute_error, changed_pixel_percent


def compare_overlay_screenshots(
    aot: Path,
    dev: Path,
    phase: str,
    background_offset_px: float,
    accessibility: list[object] | None = None,
) -> tuple[float, float, float, float]:
    """Compare a modal surface and its dimmed background separately.

    The underlay captures provide the actual scroll delta. Align only the
    background pixels; the modal itself stays fixed to the viewport and is
    compared in its original position.
    """
    with Image.open(aot) as source:
        image_a = source.convert("RGB")
    with Image.open(dev) as source:
        image_d = source.convert("RGB")
    if image_a.size != image_d.size:
        raise ValueError(f"screenshot dimensions differ: AOT={image_a.size}, DevRuntime={image_d.size}")

    width, height = image_a.size
    (surface_left, surface_top, surface_right, surface_bottom), (left, top, right, bottom) = (
        derive_overlay_bounds(accessibility, phase, width, height)
    )
    overlay_a = image_a.crop((left, top, right, bottom))
    overlay_d = image_d.crop((left, top, right, bottom))

    offset_px = round(background_offset_px)
    shifted_d = ImageChops.offset(image_d, 0, offset_px)
    first_row = round(height * 0.17)
    last_row = height - first_row
    background_a = image_a.crop((0, first_row, width, last_row))
    background_d = shifted_d.crop((0, first_row, width, last_row))
    background_diff = ImageChops.difference(background_a, background_d)
    mask = Image.new("L", (width, last_row - first_row), 255)
    draw = ImageDraw.Draw(mask)
    rects = (
        (surface_left, surface_top, surface_right - 1, surface_bottom - 1),
        (surface_left, surface_top + offset_px, surface_right - 1, surface_bottom - 1 + offset_px),
    )
    for x1, y1, x2, y2 in rects:
        y1 = max(y1, first_row) - first_row
        y2 = min(y2, last_row - 1) - first_row
        if y1 <= y2:
            draw.rectangle((x1, y1, x2, y2), fill=0)

    overlay_mae = sum(ImageStat.Stat(ImageChops.difference(overlay_a, overlay_d)).mean) / 3
    overlay_channels = ImageChops.difference(overlay_a, overlay_d).split()
    overlay_max = ImageChops.lighter(ImageChops.lighter(overlay_channels[0], overlay_channels[1]), overlay_channels[2])
    overlay_changed = overlay_max.point(lambda value: 255 if value > 8 else 0)
    overlay_percent = overlay_changed.histogram()[255] * 100 / (overlay_a.width * overlay_a.height)

    background_mae = sum(ImageStat.Stat(background_diff, mask).mean) / 3
    red, green, blue = background_diff.split()
    background_max = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    background_changed = background_max.point(lambda value: 255 if value > 8 else 0)
    included_pixels = mask.histogram()[255]
    background_percent = background_changed.histogram(mask)[255] * 100 / included_pixels
    if overlay_mae > 1.0 or overlay_percent > 0.5:
        raise AssertionError(
            f"modal surface mismatch: MAE={overlay_mae:.3f}, "
            f"pixels over 8 levels={overlay_percent:.3f}%"
        )
    if background_mae > 1.0 or background_percent > 0.5:
        raise AssertionError(
            f"dimmed background mismatch: MAE={background_mae:.3f}, "
            f"pixels over 8 levels={background_percent:.3f}%"
        )
    return overlay_mae, overlay_percent, background_mae, background_percent

    background_mae = sum(ImageStat.Stat(background_diff, mask).mean) / 3
    red, green, blue = background_diff.split()
    background_max = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    background_changed = background_max.point(lambda value: 255 if value > 8 else 0)
    included_pixels = mask.histogram()[255]
    background_percent = background_changed.histogram(mask)[255] * 100 / included_pixels
    if overlay_mae > 1.0 or overlay_percent > 0.5:
        raise AssertionError(
            f"modal surface mismatch: MAE={overlay_mae:.3f}, "
            f"pixels over 8 levels={overlay_percent:.3f}%"
        )
    if background_mae > 1.0 or background_percent > 0.5:
        raise AssertionError(
            f"dimmed background mismatch: MAE={background_mae:.3f}, "
            f"pixels over 8 levels={background_percent:.3f}%"
        )
    return overlay_mae, overlay_percent, background_mae, background_percent


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
    comparisons: dict[str, tuple[list[object], list[object], float, str, bool]] = {}
    for phase in sorted(aot_phases):
        aot_accessibility = canonical_accessibility(aot / f"{phase}.json")
        dev_accessibility = canonical_accessibility(dev / f"{phase}.json")
        vertical_offset = 0.0
        offset_unit = "pt"
        aligned_offset = None
        if any(
            isinstance(node, dict)
            and isinstance(node.get("frame"), dict)
            and "left" in node["frame"]
            for nodes in (aot_accessibility, dev_accessibility)
            for node in nodes
        ):
            offset_unit = "px"
        accessibility_matches = aot_accessibility == dev_accessibility
        if aot_accessibility != dev_accessibility:
            aligned_offset = uniform_scroll_offset(
                aot_accessibility,
                dev_accessibility,
                phase,
                Image.open(aot / f"{phase}.png").height,
            )
            if aligned_offset is not None:
                vertical_offset, offset_unit = aligned_offset
                print(f"{phase}: accessibility matches with {vertical_offset:+.1f}{offset_unit} content scroll offset")
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
        comparisons[phase] = (
            aot_accessibility,
            dev_accessibility,
            vertical_offset,
            offset_unit,
            accessibility_matches or aligned_offset is not None,
        )

    for phase in sorted(aot_phases):
        try:
            underlay = OVERLAY_UNDERLAY_PHASES.get(phase)
            _, _, _, phase_unit, _ = comparisons[phase]
            if underlay is not None and phase_unit == "px":
                if underlay not in comparisons:
                    raise AssertionError(f"missing paired Android {underlay} capture")
                _, _, background_offset, background_unit, underlay_matches = comparisons[underlay]
                if not underlay_matches or background_unit != "px":
                    raise AssertionError(f"could not establish a shared Android {underlay} background position")
                overlay_mae, overlay_changed, background_mae, background_changed = compare_overlay_screenshots(
                    aot / f"{phase}.png",
                    dev / f"{phase}.png",
                    phase,
                    background_offset,
                    accessibility=aot_accessibility,
                )
                print(
                    f"{phase}: modal/background match "
                    f"(surface MAE={overlay_mae:.3f}, changed={overlay_changed:.3f}%; "
                    f"background MAE={background_mae:.3f}, changed={background_changed:.3f}%)"
                )
                continue

            aot_a11y, dev_a11y, vertical_offset, offset_unit, _ = comparisons[phase]
            scale = 1.0 if offset_unit == "px" else pixels_per_point(aot_a11y, Image.open(aot / f"{phase}.png").width)
            mae, changed = compare_screenshots(
                aot / f"{phase}.png",
                dev / f"{phase}.png",
                vertical_offset,
                scale,
                aot_accessibility=aot_a11y,
                dev_accessibility=dev_a11y,
                phase=phase,
            )
            print(f"{phase}: screenshot matches (MAE={mae:.3f}, changed pixels={changed:.3f}%)")
        except (AssertionError, OSError, ValueError) as error:
            failures += 1
            print(f"{phase}: {error}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
