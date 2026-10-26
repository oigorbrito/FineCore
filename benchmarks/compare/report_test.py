import json
import os
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO

import report


class ComparisonReportTests(unittest.TestCase):
    def test_active_results_use_finecore_identity(self):
        with tempfile.TemporaryDirectory() as result_dir:
            measurements = [
                ("process baseline", 0.001),
                ("finecore --help", 0.002),
                ("finecore (startup)", 0.003),
            ]
            for index, (command, mean) in enumerate(measurements):
                path = os.path.join(result_dir, f"{index}.json")
                with open(path, "w", encoding="utf-8") as output:
                    json.dump({"results": [{"command": command, "mean": mean}]}, output)

            output = StringIO()
            with redirect_stdout(output):
                result = report.report(result_dir)

        self.assertEqual(result, 0)
        self.assertIn("finecore --help", output.getvalue())
        self.assertIn("finecore (startup)", output.getvalue())
        self.assertNotIn("orcacode", output.getvalue().lower())


if __name__ == "__main__":
    unittest.main()
