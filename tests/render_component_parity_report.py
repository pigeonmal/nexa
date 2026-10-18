#!/usr/bin/env python3
"""Render the component and runtime status tables in the root PARITY.md."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "examples/component-parity/App.nx"
STATUS = ROOT / "tests/fixtures/component-parity-status.json"
REPORT = ROOT / "PARITY.md"


def markdown_cell(value: object) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def balanced_end(source: str, opening: int, left: str, right: str) -> int:
    depth = 0
    in_string = False
    escaped = False
    for index in range(opening, len(source)):
        char = source[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == left:
            depth += 1
        elif char == right:
            depth -= 1
            if depth == 0:
                return index
    raise ValueError(f"unclosed {left}{right} group in parity fixture")


def top_level_arguments(source: str) -> list[str]:
    arguments: list[str] = []
    start = 0
    stack: list[str] = []
    pairs = {"(": ")", "[": "]", "{": "}"}
    in_string = False
    escaped = False
    for index, char in enumerate(source):
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in pairs:
            stack.append(pairs[char])
        elif stack and char == stack[-1]:
            stack.pop()
        elif char == "," and not stack:
            arguments.append(source[start:index].strip())
            start = index + 1
    tail = source[start:].strip()
    if tail:
        arguments.append(tail)
    return arguments


def screen_body(source: str, route: str) -> str:
    match = re.search(rf"^\s*screen\s+{re.escape(route)}\s*\{{", source, re.M)
    if match is None:
        raise ValueError(f"missing parity route: {route}")
    opening = source.find("{", match.start())
    return source[opening + 1 : balanced_end(source, opening, "{", "}")]


def fixture_arguments(source: str, component_names: set[str]) -> dict[str, set[str]]:
    found: dict[str, set[str]] = {}
    identifier = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
    index = 0
    in_string = False
    escaped = False
    while index < len(source):
        char = source[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            index += 1
            continue
        if char == '"':
            in_string = True
            index += 1
            continue
        match = identifier.match(source, index)
        if match is None:
            index += 1
            continue
        name = match.group(0)
        index = match.end()
        if name not in component_names:
            continue
        cursor = index
        while cursor < len(source) and source[cursor].isspace():
            cursor += 1
        if cursor >= len(source) or source[cursor] != "(":
            continue
        end = balanced_end(source, cursor, "(", ")")
        args = top_level_arguments(source[cursor + 1 : end])
        names = found.setdefault(name, set())
        for position, argument in enumerate(args):
            named = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:", argument)
            if named:
                names.add(named.group(1))
            elif position == 0:
                names.add("positional")
        index = end + 1
    return found


def route_parameter_inventory(app: str, routes: list[str]) -> dict[str, str]:
    catalog = (ROOT / "crates/nexa-syntax/src/catalog.rs").read_text(encoding="utf-8")
    component_section = catalog.split("pub const COMPONENTS: &[ComponentEntry] = &[", 1)[1].split("\n];", 1)[0]
    component_names = set(re.findall(r'name:\s*"([A-Za-z_][A-Za-z0-9_]*)"', component_section))
    inventory: dict[str, str] = {}
    for route in routes:
        by_component = fixture_arguments(screen_body(app, route), component_names)
        entries = [
            f"{component}[{', '.join(sorted(arguments))}]"
            for component, arguments in sorted(by_component.items())
        ]
        inventory[route] = "; ".join(entries) if entries else "No component calls found"
    return inventory


def render() -> str:
    app = APP.read_text(encoding="utf-8")
    screens = re.findall(r"^\s*screen\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{", app, re.M)
    routes = [screen for screen in screens if screen != "Catalog"]
    parameter_inventory = route_parameter_inventory(app, routes)
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    if status.get("schema_version") != 2:
        raise ValueError("unsupported component parity status schema")

    approved = set(status["human_approved_routes"])
    if not approved.issubset(routes):
        raise ValueError(f"approved routes missing from App.nx: {sorted(approved - set(routes))}")
    current_gate = status["current_human_gate"]
    if current_gate not in routes:
        raise ValueError(f"current human review gate missing from App.nx: {current_gate}")
    if current_gate in approved:
        raise ValueError(f"current human review gate is already approved: {current_gate}")
    values = status["parameter_value_audit"]
    device = status["device_review"]
    approved_components = status["human_approved_components"]
    customization_audit = status["customization_audit_goal"]
    seen_customization_components = set()
    for row in customization_audit:
        if not {"component", "route", "focus"}.issubset(row):
            raise ValueError("customization audit rows require component, route, and focus")
        if row["route"] not in routes:
            raise ValueError(f"customization audit route missing from App.nx: {row['route']}")
        if row["component"] in seen_customization_components:
            raise ValueError(f"duplicate customization audit component: {row['component']}")
        seen_customization_components.add(row["component"])

    lines = [
        "# Nexa framework parity",
        "",
        "> [!NOTE]",
        f"> **Status:** in progress · **Feature routes:** {len(routes)} · **Human-approved components:** {len(approved_components)} · **Current human gate:** {current_gate} · **Targets:** iOS and Android",
        "",
        "This report tracks component parameter coverage, native AOT builds, DevRuntime mappings, device review, and remaining runtime capability checks. The route inventory and tables are generated from the parity app and the status fixture so new screens cannot disappear from this report.",
        "",
        "**10-second check:** regenerate this report and verify it is current.",
        "",
        "```bash",
        "python3 tests/render_component_parity_report.py --check",
        "```",
        "",
        "## Customization audit goal",
        "",
        "Review these controls for meaningful typed visual parameters and matched iOS/Android behavior. Reopened approvals stay pending until the new variants have been checked on both devices and approved by a human.",
        "",
        "| Component | Route | Audit focus |",
        "|---|---|---|",
    ]
    for row in customization_audit:
        lines.append(
            f"| {markdown_cell(row['component'])} | {markdown_cell(row['route'])} | {markdown_cell(row['focus'])} |"
        )

    lines.extend(
        [
            "",
        "## Component routes",
        "",
        "The parity app exercises every declared component argument name and shared accessibility argument. `Values / edges` tracks the deeper per-parameter audit of enum choices, boundaries, empty states, interactions, and failure states. A whole-app AOT build compiles every route; paired DevRuntime runs are listed below.",
        "",
        "| Route | Fixture components / arguments | Values / edge cases | AOT iOS + Android | DevRuntime iOS + Android | Device interaction / visual | Human review |",
        "|---|---|---|---|---|---|---|",
        ]
    )
    for route in routes:
        value_status = values.get(route, "Not started")
        device_status = device.get(route, "Not started")
        human_status = "Approved" if route in approved else "Pending"
        lines.append(
            f"| {route} | {markdown_cell(parameter_inventory[route])} | {markdown_cell(value_status)} | Whole app builds | Dispatch gate | {markdown_cell(device_status)} | {human_status} |"
        )

    lines.extend(
        [
            "",
            "## Matched AOT and DevRuntime device runs",
            "",
            "| Device | Paired states | Accessibility and screenshots | Interaction coverage |",
            "|---|---:|---|---|",
        ]
    )
    for row in status["device_runs"]:
        lines.append(
            "| "
            + " | ".join(
                markdown_cell(row[key])
                for key in (
                    "platform",
                    "paired states",
                    "accessibility and screenshots",
                    "interaction coverage",
                )
            )
            + " |"
        )

    lines.extend(
        [
            "",
            "## Runtime and framework capabilities",
            "",
            "| Capability | Automated evidence | iOS AOT / Android AOT | iOS DevRuntime / Android DevRuntime | Remaining status |",
            "|---|---|---|---|---|",
        ]
    )
    for row in status["runtime_capabilities"]:
        lines.append(
            "| "
            + " | ".join(
                markdown_cell(row[key])
                for key in (
                    "capability",
                    "automated evidence",
                    "iOS AOT / Android AOT",
                    "iOS DevRuntime / Android DevRuntime",
                    "status",
                )
            )
            + " |"
        )

    lines.extend(
        [
            "",
            "## Current review notes",
            "",
            "- All 41 AOT catalog routes have opened and produced paired iOS 26.5 / Android API 31 screenshots. These route captures complement, but do not replace, each route's parameter-value and edge-state audit.",
            "- Android title menu actions and the focused iOS selected-tab title check passed. A route assertion initially expected the route name instead of the selected AppBottomBar title; the test now checks the native tab title.",
            "- TextCases has a complete fixture-value audit across its arguments and enum choices. Every built-in argument name is exercised by the parity app; other component-specific value and boundary audits remain open.",
            "- The Android API 31 AOT-to-DevRuntime run passed all 32 paired accessibility and screenshot states across controls, state, navigation, lists, styling, and overlays. The iOS 26.5 run passed 29 paired states. Fresh Picker AOT and DevRuntime runs now pass first-tap selection on both platforms; the eight iOS captures match after the screenshot tool masks the status bar. Comparisons allow only the report's measured image tolerance and uniform content-scroll alignment.",
            "- A bottom-bar content alignment mismatch was traced to Swift tab-page `.top` centering; Swift AOT and DevRuntime now use `.topLeading`. Both platform AOT/DevRuntime screenshot pairs pass.",
            "- The Android DevRuntime component smoke build exposed shared generated navigation and bottom-sheet helpers missing imports in minimal apps. The generator now keeps feature-specific imports gated while explicitly importing the shared back-icon symbols and reading animated height through the state value; the Kotlin backend test suite passes.",
            "- The Android DevRuntime component smoke passed live source updates, button and conditional-state interactions, and hot restart. Its UI hierarchy does not expose the nested Link and KeyboardAware child labels, so those labels are not used as UIAutomator-only assertions; the component and argument remain in the AOT catalog and DevRuntime fixtures.",
            "- FastList parity fixes: Android AOT sectioned lists now fill the available width like flat lists and DevRuntime; refreshable short lists keep scroll gesture dispatch active so pull-to-refresh works. Swift section headers now explicitly return their VStack, fixing generated Swift inference.",
            "- Generic RefreshControl had a second Android hit-area issue: short child content made the vertical scroll surface only 40 px tall, so a normal pull elsewhere on screen was ignored. The shared Android primitive now fills the viewport and its inner scroll container fills that area; iOS and Android DevRuntime pull callbacks pass after live reload.",
            "- Focused FastList DevRuntime tests now pass on both devices for sections, headers, grid, scroll/end-reached callbacks, pull-to-refresh state, and live FastList hot reload with scroll retention. The iOS hot-reload test uses unique run markers to prevent stale test-runner state from skipping the initial-state check.",
            "- Fresh `nexa create` Android scaffolds now use minSdk 24, matching the Material 3 Expressive menu DevRuntime minimum; previously a generated API 23 config prevented a fresh Android `nexa dev` app from launching.",
            "- Android pager dots still have an accessibility-bounds mismatch: the UIAutomator-reported Page 2 bounds select Page 1 at their center, while the visible Page 2 dot selects Page 2. This remains under PagerCases.",
            "- Device viewport and density were left unchanged.",
            "- Previous human approvals: " + ", ".join(approved_components[:-1]) + ", and " + approved_components[-1] + "; Title includes its title variants.",
            "- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets`, and `cargo test --workspace` pass. The core task integration test passes after the SQLite analyzer walker learned `AppBottomBar` tab and custom-bar children; the test's Swift spring expectation now matches the shared physical spring mapping.",
            "- The Android API 31 emulator has no Play Store / Play-services Cronet provider, so network requests were not tested on-device; network codegen and integration coverage remain available.",
            "- Native OS status bars, navigation gestures, and platform-owned controls may retain documented native differences; shared content, layout, color, typography, and interaction remain parity targets.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if PARITY.md is stale")
    args = parser.parse_args()
    expected = render()
    if args.check:
        current = REPORT.read_text(encoding="utf-8") if REPORT.exists() else ""
        if current != expected:
            print("PARITY.md is stale; run python3 tests/render_component_parity_report.py", file=sys.stderr)
            return 1
        return 0
    REPORT.write_text(expected, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
