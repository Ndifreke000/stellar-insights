"""Tests for scripts/check_gas_regression.py (#2787).

Run with: python3 -m unittest discover -s scripts/tests
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_gas_regression as gas  # noqa: E402

LINE = (
    "GAS payraider::submit_snapshot cpu=1000 mem=500 read_entries=4 "
    "write_entries=2 read_bytes=0 write_bytes=800"
)

BASELINE = f"""
{gas.MARKER_START}
| Entrypoint | CPU instructions | Memory bytes | Entries read | Entries written | Bytes read | Bytes written | Max regression |
|---|---:|---:|---:|---:|---:|---:|---:|
| `payraider::submit_snapshot` | 1000 | 500 | 4 | 2 | 0 | 800 | 5% |
{gas.MARKER_END}
"""


class ParseTests(unittest.TestCase):
    def test_parses_all_metrics(self):
        results = gas.parse_gas_lines([LINE, "noise"])
        self.assertEqual(results["payraider::submit_snapshot"]["write_bytes"], 800)
        self.assertEqual(results["payraider::submit_snapshot"]["read_entries"], 4)

    def test_parses_legacy_cpu_mem_line(self):
        results = gas.parse_gas_lines(["GAS payraider::pause cpu=10 mem=20"])
        self.assertEqual(results, {"payraider::pause": {"cpu": 10, "mem": 20}})

    def test_parses_legacy_baseline_table(self):
        text = f"{gas.MARKER_START}\n| `payraider::pause` | 10 | 20 | 10% |\n{gas.MARKER_END}"
        self.assertEqual(gas.parse_baseline_text(text), {"payraider::pause": {"cpu": 10, "mem": 20}})

    def test_parses_full_baseline_table(self):
        base = gas.parse_baseline_text(BASELINE)
        self.assertEqual(base["payraider::submit_snapshot"]["write_entries"], 2)


class CompareTests(unittest.TestCase):
    thresholds = {"payraider::submit_snapshot": 0.05}

    def test_unchanged_is_ok(self):
        rows = gas.compare(gas.parse_gas_lines([LINE]), gas.parse_baseline_text(BASELINE), self.thresholds)
        self.assertEqual(rows[0]["status"], "OK")
        self.assertEqual(gas.regression_lines(rows), [])

    def test_extra_storage_read_fails_with_clear_diff(self):
        # Regression test from the issue: one more ledger read must fail CI.
        line = LINE.replace("read_entries=4", "read_entries=5")
        rows = gas.compare(gas.parse_gas_lines([line]), gas.parse_baseline_text(BASELINE), self.thresholds)
        self.assertEqual(rows[0]["status"], "FAIL")
        lines = gas.regression_lines(rows)
        self.assertEqual(len(lines), 1)
        self.assertIn("Entries read 4 → 5", lines[0])
        self.assertIn("**5 (+25.0%)**", gas.markdown_report(rows))

    def test_within_threshold_passes(self):
        line = LINE.replace("cpu=1000", "cpu=1049")
        rows = gas.compare(gas.parse_gas_lines([line]), gas.parse_baseline_text(BASELINE), self.thresholds)
        self.assertEqual(rows[0]["status"], "OK")

    def test_just_over_threshold_fails(self):
        line = LINE.replace("cpu=1000", "cpu=1051")
        rows = gas.compare(gas.parse_gas_lines([line]), gas.parse_baseline_text(BASELINE), self.thresholds)
        self.assertEqual(rows[0]["status"], "FAIL")

    def test_zero_baseline_growth_is_a_regression(self):
        line = LINE.replace("read_bytes=0", "read_bytes=64")
        rows = gas.compare(gas.parse_gas_lines([line]), gas.parse_baseline_text(BASELINE), self.thresholds)
        self.assertEqual(rows[0]["status"], "FAIL")

    def test_new_entrypoint_is_reported_not_failed(self):
        rows = gas.compare(gas.parse_gas_lines([LINE]), {}, self.thresholds)
        self.assertEqual(rows[0]["status"], "NEW")


class CoverageTests(unittest.TestCase):
    def test_missing_and_stale_thresholds(self):
        problems = gas.coverage_problems(
            {"payraider::a": {"cpu": 1, "mem": 1}}, {"payraider::b": 0.05}, Path("t.toml")
        )
        text = "\n".join(problems)
        self.assertIn("payraider::a", text)
        self.assertIn("payraider::b", text)


if __name__ == "__main__":
    unittest.main()
