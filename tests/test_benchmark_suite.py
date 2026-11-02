"""Unit tests for the Nexa performance benchmark suite and budget comparator."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("benchmark-suite.py")
SPEC = importlib.util.spec_from_file_location("benchmark_suite", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {SCRIPT}")
BENCHMARK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCHMARK)


class BenchmarkSuiteTests(unittest.TestCase):
    def make_sample_data(
        self,
        compiler_micros: int = 1000,
        codegen_micros: int = 500,
        source_bytes: int = 2000,
    ) -> dict[str, object]:
        return {
            "fixtures": {
                "counter": {
                    "median": {
                        "dependencyResolutionMicros": 100,
                        "compilerMicros": compiler_micros,
                        "targets": {
                            "swift": {
                                "codegenMicros": codegen_micros,
                                "generatedSourceBytes": source_bytes,
                            },
                            "kotlin": {
                                "codegenMicros": codegen_micros,
                                "generatedSourceBytes": source_bytes,
                            },
                        },
                    }
                }
            }
        }

    def test_median_calculation(self) -> None:
        self.assertEqual(BENCHMARK.median([10, 20, 30]), 20)
        self.assertEqual(BENCHMARK.median([10, 20, 30, 40]), 25)

    def test_compare_against_baseline_passes_within_budget(self) -> None:
        baseline = self.make_sample_data(1000, 500, 2000)
        current = self.make_sample_data(1020, 510, 2010)  # +2% compiler, +2% codegen, +0.5% bytes

        regressions = BENCHMARK.compare_against_baseline(
            current, baseline, latency_budget_percent=5.0, size_budget_percent=3.0
        )
        self.assertEqual(regressions, [])

    def test_compare_against_baseline_detects_compiler_latency_regression(self) -> None:
        baseline = self.make_sample_data(1000, 500, 2000)
        current = self.make_sample_data(1100, 500, 2000)  # +10% compiler

        regressions = BENCHMARK.compare_against_baseline(
            current, baseline, latency_budget_percent=5.0, size_budget_percent=3.0
        )
        self.assertEqual(len(regressions), 1)
        self.assertIn("compilerMicros regressed by 10.0%", regressions[0])

    def test_compare_against_baseline_detects_codegen_latency_regression(self) -> None:
        baseline = self.make_sample_data(1000, 500, 2000)
        current = self.make_sample_data(1000, 600, 2000)  # +20% codegen

        regressions = BENCHMARK.compare_against_baseline(
            current, baseline, latency_budget_percent=5.0, size_budget_percent=3.0
        )
        self.assertEqual(len(regressions), 2)  # swift and kotlin targets
        self.assertIn("codegenMicros regressed by 20.0%", regressions[0])

    def test_compare_against_baseline_detects_source_size_regression(self) -> None:
        baseline = self.make_sample_data(1000, 500, 2000)
        current = self.make_sample_data(1000, 500, 2100)  # +5% bytes (> 3% budget)

        regressions = BENCHMARK.compare_against_baseline(
            current, baseline, latency_budget_percent=5.0, size_budget_percent=3.0
        )
        self.assertEqual(len(regressions), 2)  # swift and kotlin targets
        self.assertIn("generatedSourceBytes regressed by 5.0%", regressions[0])

    def test_compare_against_baseline_detects_runtime_device_metrics_regressions(self) -> None:
        baseline = self.make_sample_data(1000, 500, 2000)
        baseline["fixtures"]["counter"]["median"]["deviceMetrics"] = {
            "tapToFrameLatencyP95Ms": 16.0,
            "jankyFramePercent": 0.5,
            "gcPauseMs": 2.0,
            "steadyMemoryMb": 45.0,
            "binarySizeKb": 8500.0,
        }
        current = self.make_sample_data(1000, 500, 2000)
        current["fixtures"]["counter"]["median"]["deviceMetrics"] = {
            "tapToFrameLatencyP95Ms": 22.0,  # +37.5% latency regression
            "jankyFramePercent": 0.5,
            "gcPauseMs": 2.0,
            "steadyMemoryMb": 55.0,  # +22.2% memory regression
            "binarySizeKb": 8500.0,
        }

        regressions = BENCHMARK.compare_against_baseline(
            current, baseline, latency_budget_percent=5.0, size_budget_percent=3.0
        )
        self.assertEqual(len(regressions), 2)
        self.assertIn("tapToFrameLatencyP95Ms regressed by 37.5%", regressions[0])
        self.assertIn("steadyMemoryMb regressed by 22.2%", regressions[1])

    def test_compare_against_baseline_rejects_missing_device_metrics_when_required(self) -> None:
        baseline = self.make_sample_data(1000, 500, 2000)
        current = self.make_sample_data(1000, 500, 2000)  # No deviceMetrics

        regressions = BENCHMARK.compare_against_baseline(
            current, baseline, latency_budget_percent=5.0, size_budget_percent=3.0, require_device_metrics=True
        )
        self.assertEqual(len(regressions), 1)
        self.assertIn("missing deviceMetrics section (required in release budget mode)", regressions[0])


if __name__ == "__main__":
    unittest.main()
