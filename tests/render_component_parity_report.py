#!/usr/bin/env python3
"""Render the compact parity dashboard; detailed evidence stays in the fixture."""

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


def render() -> str:
    app = APP.read_text(encoding="utf-8")
    screens = re.findall(r"^\s*screen\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{", app, re.M)
    routes = [screen for screen in screens if screen != "Catalog"]
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    if status.get("schema_version") != 2:
        raise ValueError("unsupported component parity status schema")

    approved = set(status["human_approved_routes"])
    if not approved.issubset(routes):
        raise ValueError(f"approved routes missing from App.nx: {sorted(approved - set(routes))}")
    reviews = status["agent_review"]
    missing_reviews = sorted(set(routes) - set(reviews))
    unexpected_reviews = sorted(set(reviews) - set(routes))
    if missing_reviews or unexpected_reviews:
        raise ValueError(
            f"agent review routes do not match App.nx; missing={missing_reviews}, unexpected={unexpected_reviews}"
        )
    valid_statuses = {"Approved", "Reviewed", "Pending", "In progress", "Not applicable"}
    invalid_statuses = {route: value for route, value in reviews.items() if value not in valid_statuses}
    if invalid_statuses:
        raise ValueError(f"invalid agent review statuses: {invalid_statuses}")
    approval_mismatches = sorted(route for route in approved if reviews.get(route) != "Approved")
    if approval_mismatches:
        raise ValueError(
            "human-approved routes must also be agent-approved: "
            f"{approval_mismatches}"
        )
    current_review = status.get("current_agent_review")
    complete = all(value in {"Approved", "Reviewed", "Not applicable"} for value in reviews.values())
    if current_review is None:
        if not complete:
            raise ValueError("current_agent_review can be null only after every route has been reviewed")
    else:
        if current_review not in routes:
            raise ValueError(f"current review route missing from App.nx: {current_review}")
        if reviews.get(current_review) in {"Approved", "Reviewed", "Not applicable"}:
            raise ValueError(f"current review route is already agent-reviewed: {current_review}")
    review_focus = status["agent_review_focus"]
    seen_components: set[str] = set()
    for row in review_focus:
        if not {"component", "route", "focus"}.issubset(row):
            raise ValueError("agent review focus rows require component, route, and focus")
        if row["route"] not in routes:
            raise ValueError(f"agent review focus route missing from App.nx: {row['route']}")
        if row["component"] in seen_components:
            raise ValueError(f"duplicate agent review focus component: {row['component']}")
        seen_components.add(row["component"])

    summary = status["report_summary"]
    reviewed_count = sum(value in {"Approved", "Reviewed", "Not applicable"} for value in reviews.values())
    in_progress_count = sum(value == "In progress" for value in reviews.values())
    pending_count = sum(value == "Pending" for value in reviews.values())
    state = (
        f"**Agent Review complete** · {reviewed_count}/{len(routes)} routes"
        if complete
        else f"**In progress** · {reviewed_count} reviewed · {in_progress_count} in progress · {pending_count} pending"
    )
    lines = [
        "# Nexa Agent Review",
        "",
        f"> {state} · iOS + Android · native AOT + DevRuntime",
        "",
        "Review shared behavior and appearance across iOS and Android. Build success and sampled device checks do not establish exhaustive parity.",
        "",
        "```bash",
        "python3 tests/render_component_parity_report.py --check",
        "```",
        "",
        "## Review summary" if complete else "## Current work",
        "",
    ]
    lines.extend(f"- {item}" for item in summary["current_work"])
    lines.extend([
        "",
        "## Route review",
        "",
        (
            "All routes have completed Agent Review. Human approval is recorded separately and also approves Agent Review."
            if current_review is None
            else f"Active route: **{current_review}**. Human approval also approves Agent Review; Agent Review can be completed before human approval."
        ),
        "",
        "| Route | Human review | Agent Review |",
        "|---|---|---|",
    ])
    for route in routes:
        review = "Approved" if route in approved else "Pending"
        agent_review = status["agent_review"].get(route, "Pending")
        lines.append(f"| {route} | {review} | {markdown_cell(agent_review)} |")

    lines.extend([
        "",
        "## Records",
        "",
        "- [Status fixture](tests/fixtures/component-parity-status.json): Agent Review notes, device evidence, and human approvals.",
        "- [Parity app](examples/component-parity/App.nx): route and argument fixtures.",
        "- [Report generator](tests/render_component_parity_report.py): regenerate after changing the fixture or route inventory.",
        "",
    ])
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
