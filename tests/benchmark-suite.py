#!/usr/bin/env python3
"""Repeat Nexa audit measurements for a small, representative app set."""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ("counter", "todo_app", "virtual_list", "showcase")


def git_revision() -> str | None:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def git_worktree_dirty() -> bool | None:
    try:
        return bool(
            subprocess.check_output(
                ["git", "status", "--porcelain"],
                cwd=ROOT,
                text=True,
                stderr=subprocess.DEVNULL,
            ).strip()
        )
    except (OSError, subprocess.CalledProcessError):
        return None


def median(values: list[int]) -> int:
    return round(statistics.median(values))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("nexa", type=Path, help="path to the built Nexa CLI")
    parser.add_argument("output", type=Path, help="where to write benchmark.json")
    parser.add_argument("--runs", type=int, default=3, help="audits per app (default: 3)")
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be at least 1")

    nexa = args.nexa.expanduser().resolve()
    if not nexa.is_file():
        parser.error(f"Nexa CLI does not exist: {nexa}")

    args.output.mkdir(parents=True, exist_ok=True)
    fixtures: dict[str, object] = {}
    with tempfile.TemporaryDirectory(prefix="nexa-benchmark-") as temp:
        for fixture in FIXTURES:
            source = ROOT / "examples" / f"{fixture}.nx"
            samples = []
            for run in range(args.runs):
                report_path = Path(temp) / f"{fixture}-{run}.json"
                subprocess.run(
                    [str(nexa), "audit", str(source), "--target", "all", "--out", str(report_path)],
                    cwd=ROOT,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                report = json.loads(report_path.read_text())
                performance = report["performance"]
                targets = {
                    target["target"]: {
                        "codegenMicros": target["codegenMicros"],
                        "generatedSourceBytes": target["generatedSourceBytes"],
                    }
                    for target in report["targets"]
                }
                samples.append(
                    {
                        "dependencyResolutionMicros": performance["dependencyResolutionMicros"],
                        "compilerMicros": performance["compilerMicros"],
                        "targets": targets,
                    }
                )

            target_names = samples[0]["targets"].keys()
            fixtures[fixture] = {
                "source": str(source.relative_to(ROOT)),
                "median": {
                    "dependencyResolutionMicros": median(
                        [sample["dependencyResolutionMicros"] for sample in samples]
                    ),
                    "compilerMicros": median([sample["compilerMicros"] for sample in samples]),
                    "targets": {
                        target: {
                            "codegenMicros": median(
                                [sample["targets"][target]["codegenMicros"] for sample in samples]
                            ),
                            "generatedSourceBytes": median(
                                [sample["targets"][target]["generatedSourceBytes"] for sample in samples]
                            ),
                        }
                        for target in target_names
                    },
                },
                "samples": samples,
            }

    result = {
        "format": 1,
        "tool": "Nexa audit benchmark suite",
        "createdAt": datetime.now(timezone.utc).isoformat(),
        "gitRevision": git_revision(),
        "gitWorktreeDirty": git_worktree_dirty(),
        "nexaBinary": str(nexa),
        "environment": {
            "system": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "runsPerApp": args.runs,
        "fixtures": fixtures,
    }
    destination = args.output / "benchmark.json"
    destination.write_text(json.dumps(result, indent=2) + "\n")
    print(destination)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
