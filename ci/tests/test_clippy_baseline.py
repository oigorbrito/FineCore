import importlib.util
import json
import unittest
from collections import Counter
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[2] / "scripts/check_clippy_baseline.py"
spec = importlib.util.spec_from_file_location("check_clippy_baseline", SCRIPT)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class ClippyBaselineTests(unittest.TestCase):
    def test_exact_inherited_diagnostics_are_accepted(self):
        baseline = Counter({("crates/a.rs", 10, 2): 1})
        self.assertEqual(([], 1), checker.compare(baseline.copy(), baseline))

    def test_new_location_is_rejected(self):
        current = Counter({("crates/a.rs", 10, 2): 1, ("crates/b.rs", 20, 4): 1})
        baseline = Counter({("crates/a.rs", 10, 2): 1})
        self.assertEqual([("crates/b.rs", 20, 4)], checker.compare(current, baseline)[0])

    def test_additional_occurrence_at_inherited_location_is_rejected(self):
        current = Counter({("crates/a.rs", 10, 2): 2})
        baseline = Counter({("crates/a.rs", 10, 2): 1})
        self.assertEqual([("crates/a.rs", 10, 2)], checker.compare(current, baseline)[0])

    def test_removed_inherited_diagnostic_is_reported_as_progress(self):
        current = Counter()
        baseline = Counter({("crates/a.rs", 10, 2): 1})
        self.assertEqual(([], 0), checker.compare(current, baseline))

    def test_clippy_failure_from_only_pinned_diagnostics_is_accepted(self):
        inherited = Counter({("crates/a.rs", 10, 2): 1})
        self.assertFalse(checker.should_fail(inherited.copy(), inherited, [], 101))

    def test_non_clippy_compiler_error_is_rejected(self):
        inherited = Counter({("crates/a.rs", 10, 2): 1})
        self.assertTrue(checker.should_fail(inherited.copy(), inherited, ["error"], 101))

    def test_only_primary_spans_for_the_requested_lint_are_counted(self):
        data = {
            "reason": "compiler-message",
            "message": {
                "code": {"code": "clippy::result_large_err"},
                "spans": [
                    {"is_primary": True, "file_name": "crates/a.rs", "line_start": 5, "column_start": 3},
                    {"is_primary": False, "file_name": "crates/b.rs", "line_start": 9, "column_start": 1},
                ],
            },
        }
        found, errors = checker.diagnostic_keys([json.dumps(data)], "clippy::result_large_err")
        self.assertEqual(Counter({("crates/a.rs", 5, 3): 1}), found)
        self.assertEqual([], errors)


if __name__ == "__main__":
    unittest.main()
