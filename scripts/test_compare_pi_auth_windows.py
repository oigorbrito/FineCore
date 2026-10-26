#!/usr/bin/env python3
"""Unit tests for the Windows comparison report parser."""

import unittest

from compare_pi_auth_windows import SUMMARY, failure_names, totals


class WindowsComparisonTests(unittest.TestCase):
    def test_failure_parser_reads_only_final_failure_name_list(self) -> None:
        log = """test a ... FAILED

failures:

---- module::a stdout ----
assertion failed

failures:
    module::a
    module::b

test result: FAILED. 2 passed; 2 failed; 1 ignored; 0 measured
"""
        self.assertEqual(failure_names(log), ["module::a", "module::b"])

    def test_failure_parser_handles_successful_suites(self) -> None:
        self.assertEqual(failure_names("test result: ok. 4 passed; 0 failed; 0 ignored"), [])

    def test_test_summaries_are_aggregated(self) -> None:
        log = "test result: FAILED. 10 passed; 2 failed; 1 ignored\n"
        summaries = [
            {"passed": int(passed), "failed": int(failed), "ignored": int(ignored)}
            for passed, failed, ignored in SUMMARY.findall(log)
        ]
        self.assertEqual(totals({"summaries": summaries}), {"passed": 10, "failed": 2, "ignored": 1})


if __name__ == "__main__":
    unittest.main()
