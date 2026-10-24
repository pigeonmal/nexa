#!/usr/bin/env python3
"""Open each component catalog destination on an already-running Android emulator."""

from __future__ import annotations

import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "examples/component-parity/App.nx"
DEVICE = "emulator-5554"
APP_ID = "dev.nexa.componentparity"


def run(*args: str, capture: bool = False) -> str:
    result = subprocess.run(
        ["adb", "-s", DEVICE, *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture else subprocess.DEVNULL,
        stderr=subprocess.DEVNULL if not capture else None,
    )
    return result.stdout if capture else ""


def source_block(name: str) -> str:
    text = SOURCE.read_text()
    match = re.search(rf"\bscreen\s+{re.escape(name)}\s*\{{", text)
    if match is None:
        raise RuntimeError(f"missing screen declaration: {name}")
    opening = text.find("{", match.start())
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[opening + 1 : index]
    raise RuntimeError(f"unclosed screen declaration: {name}")


def links_in(block: str) -> list[tuple[str, str]]:
    return re.findall(
        r'NavigationLink\(destination:\s*(\w+)\)\s*\{\s*Text\("([^"]+)"\)\s*\}',
        block,
    )


def hierarchy(path: Path) -> ET.Element:
    for _ in range(4):
        run("shell", "rm", "-f", "/sdcard/nexa-component-catalog.xml")
        subprocess.run(
            [
                "adb",
                "-s",
                DEVICE,
                "shell",
                "uiautomator",
                "dump",
                "/sdcard/nexa-component-catalog.xml",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        result = subprocess.run(
            ["adb", "-s", DEVICE, "exec-out", "cat", "/sdcard/nexa-component-catalog.xml"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        if result.returncode == 0 and b"<hierarchy" in result.stdout:
            path.write_bytes(result.stdout)
            return ET.fromstring(result.stdout)
        time.sleep(0.15)
    raise RuntimeError("could not read Android accessibility hierarchy")


def label_node(root: ET.Element, label: str) -> tuple[ET.Element, list[ET.Element]] | None:
    def visit(node: ET.Element, ancestors: list[ET.Element]):
        if node.tag == "node" and (
            node.attrib.get("text") == label or node.attrib.get("content-desc") == label
        ):
            yield node, ancestors
        next_ancestors = ancestors + [node] if node.tag == "node" else ancestors
        for child in node:
            yield from visit(child, next_ancestors)

    return next(visit(root, []), None)


def bounds_center(node: ET.Element) -> tuple[int, int]:
    left, top, right, bottom = map(
        int, node.attrib["bounds"].replace("][", ",").strip("[]").split(",")
    )
    return (left + right) // 2, (top + bottom) // 2


def tap_label(label: str, xml_path: Path, *, scroll: bool = True) -> None:
    for _ in range(28 if scroll else 1):
        root = hierarchy(xml_path)
        match = label_node(root, label)
        if match is not None:
            node, ancestors = match
            clickable = next(
                (candidate for candidate in (node, *reversed(ancestors)) if candidate.attrib.get("clickable") == "true"),
                node,
            )
            x, y = bounds_center(clickable)
            run("shell", "input", "tap", str(x), str(y))
            time.sleep(0.35)
            return
        if not scroll:
            break
        size = run("shell", "wm", "size", capture=True)
        match = re.search(r"(\d+)x(\d+)", size)
        if match is None:
            raise RuntimeError("could not determine Android display size")
        width, height = map(int, match.groups())
        center = width // 2
        run(
            "shell",
            "input",
            "swipe",
            str(center),
            str(height * 7 // 10),
            str(center),
            str(height * 5 // 10),
            "700",
        )
        time.sleep(0.2)
    raise RuntimeError(f"could not reach Android control: {label}")


def tap_last_label(label: str, xml_path: Path) -> None:
    root = hierarchy(xml_path)
    matches: list[tuple[ET.Element, list[ET.Element]]] = []

    def visit(node: ET.Element, ancestors: list[ET.Element]) -> None:
        if node.tag == "node" and (
            node.attrib.get("text") == label or node.attrib.get("content-desc") == label
        ):
            matches.append((node, ancestors))
        next_ancestors = ancestors + [node] if node.tag == "node" else ancestors
        for child in node:
            visit(child, next_ancestors)

    visit(root, [])
    if not matches:
        raise RuntimeError(f"could not find Android control: {label}")
    node, ancestors = matches[-1]
    clickable = next(
        (candidate for candidate in (node, *reversed(ancestors)) if candidate.attrib.get("clickable") == "true"),
        node,
    )
    x, y = bounds_center(clickable)
    run("shell", "input", "tap", str(x), str(y))
    time.sleep(0.35)


def tap_child_label(group_label: str, child_label: str, xml_path: Path) -> None:
    for _ in range(28):
        root = hierarchy(xml_path)
        group_match = label_node(root, group_label)
        if group_match is not None:
            group, _ = group_match

            def visit(node: ET.Element, ancestors: list[ET.Element]):
                if node.tag == "node" and node.attrib.get("text") == child_label:
                    yield node, ancestors
                next_ancestors = ancestors + [node] if node.tag == "node" else ancestors
                for child in node:
                    yield from visit(child, next_ancestors)

            match = next(visit(group, []), None)
            if match is not None:
                node, ancestors = match
                clickable = next(
                    (candidate for candidate in (node, *reversed(ancestors)) if candidate.attrib.get("clickable") == "true"),
                    node,
                )
                x, y = bounds_center(clickable)
                run("shell", "input", "tap", str(x), str(y))
                time.sleep(0.35)
                return

        size = run("shell", "wm", "size", capture=True)
        match = re.search(r"(\d+)x(\d+)", size)
        if match is None:
            raise RuntimeError("could not determine Android display size")
        width, height = map(int, match.groups())
        run("shell", "input", "swipe", str(width // 2), str(height * 7 // 10), str(width // 2), str(height * 5 // 10), "700")
        time.sleep(0.2)
    raise RuntimeError(f"could not find {child_label} in segmented control {group_label}")


def scroll_to_label(label: str, xml_path: Path) -> None:
    for _ in range(28):
        root = hierarchy(xml_path)
        if label_node(root, label) is not None:
            return
        size = run("shell", "wm", "size", capture=True)
        match = re.search(r"(\d+)x(\d+)", size)
        if match is None:
            raise RuntimeError("could not determine Android display size")
        width, height = map(int, match.groups())
        center = width // 2
        run("shell", "input", "swipe", str(center), str(height * 7 // 10), str(center), str(height * 5 // 10), "700")
        time.sleep(0.2)
    raise RuntimeError(f"could not scroll to Android label: {label}")


def long_press_label(label: str, xml_path: Path) -> None:
    for _ in range(28):
        root = hierarchy(xml_path)
        match = label_node(root, label)
        if match is not None:
            node, ancestors = match
            clickable = next(
                (candidate for candidate in (node, *reversed(ancestors)) if candidate.attrib.get("clickable") == "true"),
                node,
            )
            x, y = bounds_center(clickable)
            run("shell", "input", "swipe", str(x), str(y), str(x), str(y), "800")
            time.sleep(0.5)
            return
        size = run("shell", "wm", "size", capture=True)
        match = re.search(r"(\d+)x(\d+)", size)
        if match is None:
            raise RuntimeError("could not determine Android display size")
        width, height = map(int, match.groups())
        center = width // 2
        run(
            "shell",
            "input",
            "swipe",
            str(center),
            str(height * 7 // 10),
            str(center),
            str(height * 5 // 10),
            "700",
        )
        time.sleep(0.2)
    raise RuntimeError(f"could not reach Android long-press target: {label}")


def wait_for_label(label: str, xml_path: Path, timeout: float = 10.0) -> ET.Element:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        root = hierarchy(xml_path)
        node = label_node(root, label)
        if node is not None:
            return root
        time.sleep(0.2)
    raise RuntimeError(f"timed out waiting for Android text or accessibility label: {label}")


def wait_for_text(label: str, xml_path: Path, timeout: float = 10.0) -> ET.Element:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        root = hierarchy(xml_path)
        node = next(
            (node for node in root.iter("node") if node.attrib.get("text") == label),
            None,
        )
        if node is not None:
            return root
        time.sleep(0.2)
    raise RuntimeError(f"timed out waiting for Android text: {label}")


def scroll_to_top(xml_path: Path) -> None:
    size = run("shell", "wm", "size", capture=True)
    match = re.search(r"(\d+)x(\d+)", size)
    if match is None:
        raise RuntimeError("could not determine Android display size")
    width, height = map(int, match.groups())
    center = width // 2
    for _ in range(24):
        root = hierarchy(xml_path)
        match = label_node(root, "Column spacing and padding")
        if match is not None:
            node, _ = match
            top, bottom = map(
                int,
                re.findall(r"-?\d+", node.attrib.get("bounds", ""))[1:4:2],
            )
            if bottom > 0 and top < height:
                return
        run(
            "shell",
            "input",
            "swipe",
            str(center),
            str(height * 3 // 10),
            str(center),
            str(height * 7 // 10),
            "500",
        )
        time.sleep(0.2)
    raise RuntimeError("could not return the Android catalog to its first route")


def capture(name: str, directory: Path) -> None:
    output = subprocess.run(
        ["adb", "-s", DEVICE, "exec-out", "screencap", "-p"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    (directory / f"{name}.png").write_bytes(output)


def changed_pixel_count(before_path: Path, after_path: Path, bounds: tuple[int, int, int, int]) -> int:
    with Image.open(before_path) as before_image, Image.open(after_path) as after_image:
        before = before_image.convert("RGB")
        after = after_image.convert("RGB")
        if before.size != after.size:
            raise RuntimeError("switch screenshots use different dimensions")
        before_crop = before.crop(bounds)
        after_crop = after.crop(bounds)
        return sum(
            1
            for y in range(before_crop.height)
            for x in range(before_crop.width)
            if max(
                abs(left - right)
                for left, right in zip(
                    before_crop.getpixel((x, y)), after_crop.getpixel((x, y))
                )
            ) > 24
        )


def assert_dark_status_bar_foreground(image_path: Path) -> None:
    with Image.open(image_path) as source:
        image = source.convert("RGB")
    width, height = image.size
    status_area = image.crop((0, 0, max(1, round(width * 0.2)), max(1, round(height * 0.04))))
    pixels = status_area.load()
    dark_pixels = sum(
        1
        for y in range(status_area.height)
        for x in range(status_area.width)
        if max(pixels[x, y]) < 160
    )
    if dark_pixels < 20:
        raise RuntimeError(
            "StatusBarCases has no dark status-bar foreground on its light background; "
            f"found only {dark_pixels} dark pixels in the status clock area"
        )


def open_route(
    label: str,
    screen: str,
    xml_path: Path,
    capture_directory: Path,
    passed: list[str],
) -> None:
    tap_label(label, xml_path)
    # AppBottomBar owns the native title for its selected tab. The screen route
    # is BottomBarCases, while the visible title and body come from the Inbox
    # tab inside that screen.
    expected = "Inbox tab content" if screen == "BottomBarCases" else screen
    wait_for_label(expected, xml_path)
    if screen == "StatusBarCases":
        # Wait for the preceding Appearance destination's Compose effects to
        # dispose before reading the native status glyphs.
        time.sleep(0.4)
    screenshot = capture_directory / f"{screen}.png"
    capture(screen, capture_directory)
    if screen == "StatusBarCases":
        assert_dark_status_bar_foreground(screenshot)
    run("shell", "input", "keyevent", "4")
    wait_for_label(label, xml_path)
    if screen not in passed:
        passed.append(screen)
    print(f"PASS {screen}: {label} opens and returns")


def run_text_tab_interactions(xml_path: Path, capture_directory: Path) -> None:
    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)
    tap_label("Text styles and sizing", xml_path)
    wait_for_label("TextCases", xml_path)

    cases = [
        ("Text layout and effects tests", "Leading alignment\nshort", "TextLayoutFirstTap"),
        ("Text semantic font styles", "LargeTitle", "TextFontStylesFirstTap"),
        ("Text typography tests", "Normal 14", "TextTypographyFirstTap"),
    ]
    for tab, expected, screenshot in cases:
        tap_label(tab, xml_path, scroll=False)
        wait_for_label(expected, xml_path, timeout=5)
        capture(screenshot, capture_directory)
        print(f"PASS TextCases: {tab} shows {expected} after one tap")


def run_switch_interaction(xml_path: Path, capture_directory: Path) -> None:
    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)
    tap_label("Switch states", xml_path)
    wait_for_label("SwitchCases", xml_path)
    capture("SwitchInitial", capture_directory)

    root = hierarchy(xml_path)
    toggle = next(
        (
            node
            for node in root.iter("node")
            if node.attrib.get("content-desc") == "Notifications"
        ),
        None,
    )
    if toggle is None:
        raise RuntimeError("Notifications switch is missing its Android accessibility bounds")
    x, y = bounds_center(toggle)
    bounds = tuple(
        int(value)
        for value in toggle.attrib["bounds"].replace("][", ",").strip("[]").split(",")
    )
    initial = capture_directory / "SwitchInitial.png"
    enabled = capture_directory / "SwitchEnabled.png"
    restored = capture_directory / "SwitchRestored.png"

    run("shell", "input", "tap", str(x), str(y))
    time.sleep(0.7)
    capture("SwitchEnabled", capture_directory)
    changed = changed_pixel_count(initial, enabled, bounds)
    if changed < 80:
        raise RuntimeError(f"Notifications did not visibly toggle on Android ({changed} changed pixels)")

    run("shell", "input", "tap", str(x), str(y))
    time.sleep(0.7)
    capture("SwitchRestored", capture_directory)
    residual = changed_pixel_count(initial, restored, bounds)
    if residual > 100:
        raise RuntimeError(f"Notifications did not return to its initial value ({residual} changed pixels)")
    print(
        "PASS SwitchCases: bound value toggles on and off "
        f"({changed} changed pixels, {residual} residual pixels)"
    )


def run_navigation_transitions(xml_path: Path, capture_directory: Path) -> None:
    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)
    tap_label("NavigationBack behavior", xml_path)
    wait_for_label("NavigationBackCases", xml_path)

    tap_label("Conditional navigation link", xml_path, scroll=False)
    wait_for_label("DirectionCases", xml_path)
    capture("NavigationTransitionPush", capture_directory)

    tap_label("Open nested navigation level", xml_path, scroll=False)
    wait_for_label("NavigationBackCases", xml_path)
    capture("NavigationTransitionNestedPush", capture_directory)

    run("shell", "input", "keyevent", "4")
    wait_for_label("Open nested navigation level", xml_path)
    capture("NavigationTransitionFirstPop", capture_directory)

    run("shell", "input", "keyevent", "4")
    wait_for_label("Conditional navigation link", xml_path)
    capture("NavigationTransitionRootRestored", capture_directory)
    print("PASS NavigationBackCases: nested pushes and system-back pops restore each previous screen")


def run_segmented_control_interaction(xml_path: Path, capture_directory: Path) -> None:
    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)
    tap_label("Segmented control", xml_path)
    wait_for_label("SegmentedCases", xml_path)
    controls = (
        ("Default time range", "Default"),
        ("Spring time range", "Spring"),
        ("Tween time range", "Tween"),
    )
    for group_label, prefix in controls:
        wait_for_label(f"{prefix}: Day", xml_path)
        capture(f"Segmented{prefix}Day", capture_directory)
        for selection in ("Week", "Month", "Day"):
            tap_child_label(group_label, selection, xml_path)
            wait_for_label(f"{prefix}: {selection}", xml_path, timeout=5)
            capture(f"Segmented{prefix}{selection}", capture_directory)
            print(f"PASS SegmentedCases: {group_label} selects {selection} on first tap")

    for index, label in enumerate(("Single selection: Only", "Empty selection: Missing", "Missing selection: Year")):
        scroll_to_label(label, xml_path)
        capture(f"SegmentedEdge{index}", capture_directory)


def run_picker_interaction(xml_path: Path, capture_directory: Path) -> None:
    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)
    tap_label("Picker options", xml_path)
    wait_for_label("PickerCases", xml_path)
    wait_for_label("Selected: Due date", xml_path)
    capture("PickerDueDate", capture_directory)

    current = "Due date"
    for selection in ("Priority", "Title", "Due date"):
        tap_label(current, xml_path, scroll=False)
        wait_for_label(selection, xml_path, timeout=5)
        if selection == "Priority":
            capture("PickerMenuOpen", capture_directory)
        tap_label(selection, xml_path, scroll=False)
        wait_for_label(f"Selected: {selection}", xml_path, timeout=5)
        capture(f"Picker{selection.replace(' ', '')}", capture_directory)
        print(f"PASS PickerCases: first tap selects {selection}")
        current = selection

    for selection in ("Due date", "Priority", "Title"):
        tap_label("Sort by", xml_path, scroll=False)
        wait_for_label("Due date", xml_path, timeout=5)
        if selection == "Due date":
            capture("PickerRowIconsMenu", capture_directory)
        tap_label(selection, xml_path, scroll=False)
        wait_for_label(f"Selected row icon: {selection}", xml_path, timeout=5)
        capture(f"PickerRowIcon{selection.replace(' ', '')}", capture_directory)
        print(f"PASS PickerCases: per-row icon picker selects {selection} on first tap")

    tap_label("Category", xml_path, scroll=False)
    wait_for_label("Title", xml_path, timeout=5)
    tap_label("Title", xml_path, scroll=False)
    wait_for_label("Selected label-only: Title", xml_path, timeout=5)
    capture("PickerLabelOnly", capture_directory)
    print("PASS PickerCases: label-only picker selects Title on first tap")

    tap_last_label("Title", xml_path)
    wait_for_label("Due date", xml_path, timeout=5)
    tap_label("Due date", xml_path, scroll=False)
    wait_for_label("Selected icon-only: Due date", xml_path, timeout=5)
    capture("PickerIconOnly", capture_directory)
    print("PASS PickerCases: tinted icon-only picker selects Due date on first tap")

    tap_last_label("Due date", xml_path)
    wait_for_label("Priority", xml_path, timeout=5)
    tap_label("Priority", xml_path, scroll=False)
    wait_for_label("Selected default: Priority", xml_path, timeout=5)
    capture("PickerDefaults", capture_directory)
    print("PASS PickerCases: default picker selects Priority on first tap")


def scrollable_ancestor(
    root: ET.Element, label: str
) -> tuple[ET.Element, list[ET.Element]]:
    match = label_node(root, label)
    if match is None:
        raise RuntimeError(f"could not find scroll content: {label}")
    _, ancestors = match
    container = next(
        (node for node in reversed(ancestors) if node.attrib.get("scrollable") == "true"),
        None,
    )
    if container is None:
        raise RuntimeError(f"no scrollable container contains: {label}")
    bounds = tuple(
        int(value)
        for value in re.findall(r"-?\d+", container.attrib["bounds"])
    )
    if len(bounds) != 4:
        raise RuntimeError(f"invalid scroll container bounds: {container.attrib.get('bounds')}")
    return container, list(bounds)


def swipe_scrollable_content(
    label: str,
    xml_path: Path,
    *,
    up: bool,
    travel_fraction: float = 0.4,
) -> None:
    root = hierarchy(xml_path)
    _, (left, top, right, bottom) = scrollable_ancestor(root, label)
    x = (left + right) // 2
    height = bottom - top
    half_travel = travel_fraction / 2
    start_fraction = 0.5 + half_travel if up else 0.5 - half_travel
    end_fraction = 0.5 - half_travel if up else 0.5 + half_travel
    start_y = top + int(height * start_fraction)
    end_y = top + int(height * end_fraction)
    run("shell", "input", "swipe", str(x), str(start_y), str(x), str(end_y), "650")


def swipe_pager_page(xml_path: Path, *, forward: bool) -> None:
    root = hierarchy(xml_path)
    match = label_node(root, "FastList cases")
    if match is None:
        raise RuntimeError("FastList pager container is missing")
    node, _ = match
    left, top, right, bottom = map(
        int, re.findall(r"-?\d+", node.attrib.get("bounds", ""))
    )
    width = right - left
    y = (top + bottom) // 2
    start_x = left + width * (4 if forward else 1) // 5
    end_x = left + width * (1 if forward else 4) // 5
    run("shell", "input", "swipe", str(start_x), str(y), str(end_x), str(y), "500")
    time.sleep(0.45)


def current_scroll_position(root: ET.Element) -> int | None:
    for node in root.iter("node"):
        text = node.attrib.get("text", "")
        match = re.fullmatch(r"Scroll position: (-?\d+)", text)
        if match is not None:
            return int(match.group(1))
    return None


def wait_for_scroll_position(
    xml_path: Path, predicate, *, timeout: float = 8.0
) -> int:
    deadline = time.monotonic() + timeout
    last_position: int | None = None
    while time.monotonic() < deadline:
        root = hierarchy(xml_path)
        last_position = current_scroll_position(root)
        if last_position is not None and predicate(last_position):
            return last_position
        time.sleep(0.2)
    raise RuntimeError(f"FastList scroll position did not reach expected value; last={last_position}")


def run_fast_list_interaction(xml_path: Path, capture_directory: Path) -> None:
    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)
    tap_label("FastList row sizing", xml_path)
    wait_for_label("FastListCases", xml_path)
    wait_for_text("FastList · vertical", xml_path)
    wait_for_label("Parity row 0", xml_path)
    capture("FastListPage1", capture_directory)

    # Android's 48 dp minimum touch target expands these tightly spaced dots
    # into overlapping hit regions; accessibility frame-centre taps can hit
    # the adjacent dot. Use the pager's horizontal swipe surface here and
    # keep the dot labels as accessibility assertions instead.
    swipe_pager_page(xml_path, forward=True)
    wait_for_label("Scroll position: 0", xml_path)
    wait_for_text("FastList · reverse and position", xml_path)
    wait_for_label("Reverse row 0", xml_path)
    capture("FastListReverseInitial", capture_directory)
    swipe_scrollable_content("Reverse row 0", xml_path, up=True)
    moved = wait_for_scroll_position(xml_path, lambda position: position > 0)
    # Logical position zero starts at row 0; after scrolling, reverseLayout
    # exposes the next logical item at the top (position + 1).
    wait_for_label(f"Reverse row {moved + 1}", xml_path)
    capture("FastListReverseScrolled", capture_directory)
    print(f"PASS FastListCases: reverse swipe updates logical scroll position to {moved}")

    swipe_scrollable_content(f"Reverse row {moved + 1}", xml_path, up=False)
    returned = wait_for_scroll_position(xml_path, lambda position: position == 0)
    wait_for_label("Reverse row 0", xml_path)
    print(f"PASS FastListCases: reverse swipe returns bound position to {returned}")

    current_page = 1
    for page, title, row in [
        (2, "FastList · vertical", "Vertical row 0"),
        (3, "FastList · horizontal", "Horizontal row 0"),
        (4, "FastList · fixed grid", "Fixed grid 0"),
        (5, "FastList · responsive grid", "Adaptive grid 0"),
        (6, "FastList · page snap", "Page 0"),
    ]:
        while current_page < page:
            swipe_pager_page(xml_path, forward=True)
            current_page += 1
        wait_for_text(title, xml_path)
        wait_for_label(row, xml_path)
        capture(f"FastListPage{page}", capture_directory)
        print(f"PASS FastListCases: {title} renders {row}")

        if page == 5:
            tap_label("Allow adaptive grid scrolling", xml_path, scroll=False)
            wait_for_text("FastList · responsive grid", xml_path)
            capture("FastListAdaptiveGridScrollToggled", capture_directory)
            tap_label("Allow adaptive grid scrolling", xml_path, scroll=False)
            print("PASS FastListCases: adaptive grid scrollEnabled toggles both ways")

        if page == 6:
            swipe_scrollable_content(
                "Page 0", xml_path, up=True, travel_fraction=0.8
            )
            wait_for_text("Page 1", xml_path)
            capture("FastListPageSnapNext", capture_directory)
            print("PASS FastListCases: pageSnap swipe advances to the next page")


def main() -> int:
    interaction_mode = sys.argv[2] if len(sys.argv) == 3 else None
    if len(sys.argv) not in (2, 3, 4) or (
        len(sys.argv) == 3
        and interaction_mode not in (
            "--text-tabs-only",
            "--switch-only",
            "--navigation-transitions-only",
            "--segmented-only",
            "--picker-only",
            "--fast-list-only",
        )
    ) or (len(sys.argv) == 4 and sys.argv[2] != "--start-after"):
        print(
            "usage: component-catalog-android-smoke.py <capture-directory> [--start-after <screen> | --text-tabs-only | --switch-only | --navigation-transitions-only | --segmented-only | --picker-only]",
            file=sys.stderr,
        )
        return 2

    capture_directory = Path(sys.argv[1]).resolve()
    start_after = sys.argv[3] if len(sys.argv) == 4 else None
    capture_directory.mkdir(parents=True, exist_ok=True)
    xml_path = capture_directory / "current.xml"
    if interaction_mode == "--text-tabs-only":
        run_text_tab_interactions(xml_path, capture_directory)
        return 0
    if interaction_mode == "--switch-only":
        run_switch_interaction(xml_path, capture_directory)
        return 0
    if interaction_mode == "--navigation-transitions-only":
        run_navigation_transitions(xml_path, capture_directory)
        return 0
    if interaction_mode == "--segmented-only":
        run_segmented_control_interaction(xml_path, capture_directory)
        return 0
    if interaction_mode == "--picker-only":
        run_picker_interaction(xml_path, capture_directory)
        return 0
    if interaction_mode == "--fast-list-only":
        run_fast_list_interaction(xml_path, capture_directory)
        return 0

    passed: list[str] = []

    run("shell", "am", "force-stop", APP_ID)
    run("shell", "monkey", "-p", APP_ID, "-c", "android.intent.category.LAUNCHER", "1")
    wait_for_label("Column spacing and padding", xml_path, timeout=30)

    routes = links_in(source_block("Catalog"))
    if start_after is not None:
        route_screens = [screen for screen, _ in routes]
        if start_after not in route_screens:
            raise RuntimeError(f"unknown catalog route to resume after: {start_after}")
        routes = routes[route_screens.index(start_after) + 1 :]

    for screen, label in routes:
        open_route(label, screen, xml_path, capture_directory, passed)

    scroll_to_top(xml_path)
    tap_label("Navigation title system", xml_path)
    wait_for_label("TitleCases", xml_path)
    tap_label("More actions", xml_path, scroll=False)
    wait_for_label("Action count: 1", xml_path)
    long_press_label("More actions", xml_path)
    wait_for_label("Archive", xml_path)
    capture("TitleMenuOpen", capture_directory)
    tap_label("Archive", xml_path, scroll=False)
    wait_for_label("Action count: 11", xml_path)
    long_press_label("More actions", xml_path)
    wait_for_label("Delete", xml_path)
    tap_label("Delete", xml_path, scroll=False)
    wait_for_label("Action count: 111", xml_path)
    print("PASS TitleCases: toolbar tap and Archive/Delete context actions update state")
    for label, screen in [
        ("Title with a leading toolbar item", "TitleLeadingCases"),
        ("Title without toolbar items", "TitleNoToolbarCases"),
    ]:
        tap_label(label, xml_path)
        wait_for_label(screen, xml_path)
        capture(screen, capture_directory)
        run("shell", "input", "keyevent", "4")
        wait_for_label(label, xml_path)
        if screen not in passed:
            passed.append(screen)
        print(f"PASS {screen}: {label} opens and returns")
    run("shell", "input", "keyevent", "4")
    scroll_to_top(xml_path)
    wait_for_label("Column spacing and padding", xml_path)

    (capture_directory / "passed.txt").write_text("\n".join(passed) + "\n")
    print(f"Verified {len(passed)} component destinations on Android.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
