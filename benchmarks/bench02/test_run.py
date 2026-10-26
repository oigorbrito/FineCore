import unittest
from pathlib import Path

import run


class Bench02StatisticsTest(unittest.TestCase):
    def test_parses_real_rust_harness_prefixes_before_reporting(self):
        report = run.load_report_module(Path(__file__).resolve().parents[2])
        record = '{"tool":"edit_file","scenario":"distinct","wall_seconds":0.1}'
        prefixed = f"test distinct_file_latency_accuracy_and_concurrency ... {report.PREFIX}{record}\\nok\\n"
        self.assertEqual(report.parse(prefixed), {})
        for output in (prefixed, f"{report.PREFIX}{record}\\n", f".{report.PREFIX}{record}\\n"):
            with self.subTest(output=output[:60]):
                parsed = run.parse_core_tools_output(report, output)
                self.assertEqual(parsed[("edit_file", "distinct")][0]["wall_seconds"], 0.1)

    def test_rejects_unexpected_marker_context(self):
        report = run.load_report_module(Path(__file__).resolve().parents[2])
        output = 'warning: [core-tools-bench] {"tool":"edit_file","scenario":"distinct"}'
        with self.assertRaisesRegex(ValueError, "unexpected benchmark marker context"):
            run.parse_core_tools_output(report, output)

    def test_percentile_uses_nearest_rank_and_p99_is_max_at_twenty(self):
        values = list(range(1, 21))
        self.assertEqual(run.percentile(values, 0.50), 10)
        self.assertEqual(run.percentile(values, 0.99), 20)

    def test_paired_bootstrap_is_deterministic_and_reports_a_ci(self):
        differences = [0.001] * 12
        first = run.bootstrap_median_ci(differences, 99)
        second = run.bootstrap_median_ci(differences, 99)
        self.assertEqual(first, second)
        self.assertEqual(first, [0.001, 0.001])

    def test_bootstrap_requires_at_least_two_pairs(self):
        self.assertIsNone(run.bootstrap_median_ci([0.1], 1))


if __name__ == "__main__":
    unittest.main()
