import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from qualify_swebench import evaluate

class QualificationTests(unittest.TestCase):
    def test_fail_closed_unfrozen(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); p=root/"selection.json"
            p.write_text(json.dumps({"qualification":"NOT_EXECUTED","proposed_first":[{"instance_id":"x"}]}))
            with self.assertRaises(ValueError), patch("qualify_swebench.subprocess.run") as runner:
                evaluate(p,root,root/"out")
                runner.assert_not_called()
    def test_gold_run_is_not_agent_pass(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); p=root/"selection.json"
            p.write_text(json.dumps({"qualification":"NOT_EXECUTED","snapshot_revision":"rev","parquet_sha256":"hash","proposed_first":[{"instance_id":"x"}]}))
            class Proc: returncode=0; stdout="official command exited zero"
            with patch("qualify_swebench.subprocess.run",return_value=Proc()) as run:
                self.assertEqual(evaluate(p,root,root/"out"),0)
            m=json.loads((root/"out/qualification-manifest.json").read_text())
            self.assertEqual(m["status"],"AWAIT_OFFICIAL_REPORT_INSPECTION")
            self.assertNotIn("qualified",m)
            self.assertEqual(m["kind"],"OFFICIAL_GOLD_ORACLE")
            self.assertIn("gold",run.call_args.args[0])
if __name__=="__main__":unittest.main()
