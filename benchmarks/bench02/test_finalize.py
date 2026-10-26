import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import finalize


def successful_manifest():
    statuses = {
        "reporting_tests": 0,
        "paired_core_tools_valid": True,
    }
    for side in ("finecore", "orca"):
        statuses[f"core_tools_accuracy_{side}"] = 0
        statuses[f"core_tools_build_{side}"] = 0
        statuses[f"core_tools_budget_{side}"] = 0
        for suite in ("tests", "accuracy", "distribution", "pareto"):
            statuses[f"mcp_{suite}_{side}"] = 0
        for suite in ("concurrency", "simulation"):
            statuses[f"subagent_{suite}_{side}"] = 0
    return {
        "donor_pin_valid": True,
        "benchmark_source_hashes": {"identical": True},
        "build_inputs": {
            "rust_toolchain_identical": True,
            "normalized_lock_graph_identical": True,
        },
        "statuses": statuses,
    }


class FinalizeTest(unittest.TestCase):
    def run_finalizer(self, data):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "manifest.json"
            path.write_text(json.dumps(data), encoding="utf-8")
            output = io.StringIO()
            with patch("sys.argv", ["finalize.py", str(path)]), contextlib.redirect_stdout(output):
                result = finalize.main()
        return result, output.getvalue()

    def test_budget_failure_stays_red_after_other_suites_pass(self):
        data = successful_manifest()
        data["statuses"]["core_tools_budget_finecore"] = 1
        result, output = self.run_finalizer(data)
        self.assertEqual(result, 1)
        self.assertIn("PASS mcp_tests_finecore: 0", output)
        self.assertIn("PASS subagent_simulation_orca: 0", output)
        self.assertIn("BENCH-02 overall: FAIL", output)

    def test_all_required_suites_and_budgets_must_be_recorded(self):
        data = successful_manifest()
        del data["statuses"]["mcp_pareto_orca"]
        result, output = self.run_finalizer(data)
        self.assertEqual(result, 1)
        self.assertIn("required suite did not record a status: mcp_pareto_orca", output)


if __name__ == "__main__":
    unittest.main()
