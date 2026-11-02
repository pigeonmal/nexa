#!/usr/bin/env python3
"""Repeat Nexa audit measurements for a small, representative app set."""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import subprocess
import sys
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


def compare_against_baseline(
    current: dict[str, object],
    baseline: dict[str, object],
    latency_budget_percent: float = 5.0,
    size_budget_percent: float = 3.0,
    require_device_metrics: bool = False,
) -> list[str]:
    """Compare current benchmark results against baseline and return list of regressions."""
    regressions: list[str] = []
    current_fixtures = current.get("fixtures", {})
    baseline_fixtures = baseline.get("fixtures", {})

    for fixture_name, current_data in current_fixtures.items():
        if not isinstance(current_data, dict):
            continue
        if fixture_name not in baseline_fixtures:
            continue
        base_data = baseline_fixtures[fixture_name]
        if not isinstance(base_data, dict):
            continue
        current_median = current_data.get("median", {})
        base_median = base_data.get("median", {})
        if not isinstance(current_median, dict) or not isinstance(base_median, dict):
            continue

        # Compare compilerMicros
        curr_comp = current_median.get("compilerMicros", 0)
        base_comp = base_median.get("compilerMicros", 0)
        if isinstance(curr_comp, (int, float)) and isinstance(base_comp, (int, float)) and base_comp > 0:
            diff_pct = (curr_comp - base_comp) / base_comp * 100.0
            if diff_pct > latency_budget_percent:
                regressions.append(
                    f"{fixture_name}: compilerMicros regressed by {diff_pct:.1f}% "
                    f"(baseline {base_comp}µs -> current {curr_comp}µs, budget +{latency_budget_percent}%)"
                )

        # Compare targets: codegenMicros and generatedSourceBytes
        curr_targets = current_median.get("targets", {})
        base_targets = base_median.get("targets", {})
        if isinstance(curr_targets, dict) and isinstance(base_targets, dict):
            for target, curr_t_data in curr_targets.items():
                if target not in base_targets or not isinstance(curr_t_data, dict):
                    continue
                base_t_data = base_targets[target]
                if not isinstance(base_t_data, dict):
                    continue
                curr_codegen = curr_t_data.get("codegenMicros", 0)
                base_codegen = base_t_data.get("codegenMicros", 0)
                if isinstance(curr_codegen, (int, float)) and isinstance(base_codegen, (int, float)) and base_codegen > 0:
                    diff_pct = (curr_codegen - base_codegen) / base_codegen * 100.0
                    if diff_pct > latency_budget_percent:
                        regressions.append(
                            f"{fixture_name} ({target}): codegenMicros regressed by {diff_pct:.1f}% "
                            f"(baseline {base_codegen}µs -> current {curr_codegen}µs, budget +{latency_budget_percent}%)"
                        )

                curr_bytes = curr_t_data.get("generatedSourceBytes", 0)
                base_bytes = base_t_data.get("generatedSourceBytes", 0)
                if isinstance(curr_bytes, (int, float)) and isinstance(base_bytes, (int, float)) and base_bytes > 0:
                    diff_pct = (curr_bytes - base_bytes) / base_bytes * 100.0
                    if diff_pct > size_budget_percent:
                        regressions.append(
                            f"{fixture_name} ({target}): generatedSourceBytes regressed by {diff_pct:.1f}% "
                            f"(baseline {base_bytes} bytes -> current {curr_bytes} bytes, budget +{size_budget_percent}%)"
                        )

        # Compare runtime device metrics when available
        curr_device = current_median.get("deviceMetrics", {})
        base_device = base_median.get("deviceMetrics", {})

        if require_device_metrics:
            required_metrics = [
                "tapToFrameLatencyP95Ms",
                "jankyFramePercent",
                "gcPauseMs",
                "steadyMemoryMb",
                "binarySizeKb",
            ]
            if not isinstance(curr_device, dict) or not curr_device:
                regressions.append(
                    f"{fixture_name}: missing deviceMetrics section (required in release budget mode)"
                )
            else:
                for req in required_metrics:
                    if req not in curr_device:
                        regressions.append(
                            f"{fixture_name}: missing required device metric '{req}' (required in release budget mode)"
                        )

        if isinstance(curr_device, dict) and isinstance(base_device, dict):
            curr_tap_p95 = curr_device.get("tapToFrameLatencyP95Ms", 0)
            base_tap_p95 = base_device.get("tapToFrameLatencyP95Ms", 0)
            if isinstance(curr_tap_p95, (int, float)) and isinstance(base_tap_p95, (int, float)) and base_tap_p95 > 0:
                diff_pct = (curr_tap_p95 - base_tap_p95) / base_tap_p95 * 100.0
                if diff_pct > latency_budget_percent:
                    regressions.append(
                        f"{fixture_name}: tapToFrameLatencyP95Ms regressed by {diff_pct:.1f}% "
                        f"(baseline {base_tap_p95}ms -> current {curr_tap_p95}ms, budget +{latency_budget_percent}%)"
                    )

            curr_jank = curr_device.get("jankyFramePercent", 0)
            base_jank = base_device.get("jankyFramePercent", 0)
            if isinstance(curr_jank, (int, float)) and isinstance(base_jank, (int, float)) and base_jank > 0:
                diff_pct = (curr_jank - base_jank) / base_jank * 100.0
                if diff_pct > latency_budget_percent:
                    regressions.append(
                        f"{fixture_name}: jankyFramePercent regressed by {diff_pct:.1f}% "
                        f"(baseline {base_jank}% -> current {curr_jank}%, budget +{latency_budget_percent}%)"
                    )

            curr_gc = curr_device.get("gcPauseMs", 0)
            base_gc = base_device.get("gcPauseMs", 0)
            if isinstance(curr_gc, (int, float)) and isinstance(base_gc, (int, float)) and base_gc > 0:
                diff_pct = (curr_gc - base_gc) / base_gc * 100.0
                if diff_pct > latency_budget_percent:
                    regressions.append(
                        f"{fixture_name}: gcPauseMs regressed by {diff_pct:.1f}% "
                        f"(baseline {base_gc}ms -> current {curr_gc}ms, budget +{latency_budget_percent}%)"
                    )

            curr_mem = curr_device.get("steadyMemoryMb", 0)
            base_mem = base_device.get("steadyMemoryMb", 0)
            if isinstance(curr_mem, (int, float)) and isinstance(base_mem, (int, float)) and base_mem > 0:
                diff_pct = (curr_mem - base_mem) / base_mem * 100.0
                if diff_pct > size_budget_percent:
                    regressions.append(
                        f"{fixture_name}: steadyMemoryMb regressed by {diff_pct:.1f}% "
                        f"(baseline {base_mem}MB -> current {curr_mem}MB, budget +{size_budget_percent}%)"
                    )

            curr_bin = curr_device.get("binarySizeKb", 0)
            base_bin = base_device.get("binarySizeKb", 0)
            if isinstance(curr_bin, (int, float)) and isinstance(base_bin, (int, float)) and base_bin > 0:
                diff_pct = (curr_bin - base_bin) / base_bin * 100.0
                if diff_pct > size_budget_percent:
                    regressions.append(
                        f"{fixture_name}: binarySizeKb regressed by {diff_pct:.1f}% "
                        f"(baseline {base_bin}KB -> current {curr_bin}KB, budget +{size_budget_percent}%)"
                    )

    return regressions


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("nexa", type=Path, help="path to the built Nexa CLI")
    parser.add_argument("output", type=Path, help="where to write benchmark.json")
    parser.add_argument("--runs", type=int, default=3, help="audits per app (default: 3)")
    parser.add_argument(
        "--baseline",
        type=Path,
        default=None,
        help="path to baseline benchmark.json to compare against",
    )
    parser.add_argument(
        "--budget-percent",
        type=float,
        default=5.0,
        help="latency regression budget percentage (default: 5.0)",
    )
    parser.add_argument(
        "--size-budget-percent",
        type=float,
        default=3.0,
        help="binary/source size regression budget percentage (default: 3.0)",
    )
    parser.add_argument(
        "--alert-only",
        action="store_true",
        help="warn on budget violation instead of exiting with failure",
    )
    parser.add_argument(
        "--device-metrics",
        type=Path,
        default=None,
        help="path to external JSON containing captured device metrics per fixture",
    )
    parser.add_argument(
        "--require-device-metrics",
        action="store_true",
        help="reject absent deviceMetrics during budget comparison (release mode)",
    )
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

    if args.device_metrics is not None:
        metrics_file = args.device_metrics.expanduser().resolve()
        if metrics_file.is_file():
            metrics_data = json.loads(metrics_file.read_text())
            for fixture_key, fixture_val in fixtures.items():
                if isinstance(fixture_val, dict) and "median" in fixture_val:
                    if fixture_key in metrics_data:
                        fixture_val["median"]["deviceMetrics"] = metrics_data[fixture_key]
                    elif "deviceMetrics" in metrics_data:
                        fixture_val["median"]["deviceMetrics"] = metrics_data["deviceMetrics"]

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

    if args.baseline is not None:
        baseline_path = args.baseline.expanduser().resolve()
        if not baseline_path.is_file():
            print(f"Baseline benchmark file not found: {baseline_path}", file=sys.stderr)
            return 1
        baseline_data = json.loads(baseline_path.read_text())
        regressions = compare_against_baseline(
            result,
            baseline_data,
            latency_budget_percent=args.budget_percent,
            size_budget_percent=args.size_budget_percent,
            require_device_metrics=args.require_device_metrics,
        )
        if regressions:
            print(f"\n⚠️  Benchmark budget violations detected ({len(regressions)}):", file=sys.stderr)
            for regression in regressions:
                print(f"  - {regression}", file=sys.stderr)
            if not args.alert_only:
                return 1
        else:
            print("\n✅ All benchmark measurements within performance budgets.")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
