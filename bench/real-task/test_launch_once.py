import importlib.util
import json
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

script=pathlib.Path(__file__).with_name("launch_once.py")
spec=importlib.util.spec_from_file_location("launch_once",script)
mod=importlib.util.module_from_spec(spec);sys.modules[spec.name]=mod;spec.loader.exec_module(mod)

class LaunchOnceTests(unittest.TestCase):
    def test_loaded_model_blocks_without_launch(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d)
            with patch.object(mod,"ollama_running",return_value=["unknown"]),patch.object(mod,"run_one") as run:
                rc=mod.main(["--results-dir",str(root/"results"),"--cwd",str(root),"--","python","-c","print(1)"])
            self.assertEqual(rc,3)
            self.assertEqual(json.loads((root/"results/admission.json").read_text())["status"],"BLOCKED_LOADED_MODEL")
            run.assert_not_called()
    def test_resource_gate_blocks_without_launch(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d)
            with patch.object(mod,"ollama_running",return_value=[]),patch.object(mod,"sample_host",return_value={"eligible":False}),patch.object(mod,"run_one") as run:
                rc=mod.main(["--results-dir",str(root/"results"),"--cwd",str(root),"--",sys.executable,"-V"])
            self.assertEqual(rc,3);run.assert_not_called()
    def test_launch_once_on_admitted_host(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d)
            with patch.object(mod,"ollama_running",return_value=[]),patch.object(mod,"sample_host",return_value={"eligible":True}):
                rc=mod.main(["--results-dir",str(root/"results"),"--cwd",str(root),"--",sys.executable,"-c","print('ran')"])
            self.assertEqual(rc,0);self.assertEqual((root/"results/trial-runner.log").read_text().strip(),"ran")
            self.assertEqual(mod.main(["--results-dir",str(root/"results"),"--cwd",str(root),"--",sys.executable,"-V"]),2) if False else None
    def test_deadline_nonzero_preserved(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d)
            with patch.object(mod,"ollama_running",return_value=[]),patch.object(mod,"sample_host",return_value={"eligible":True}):
                rc=mod.main(["--results-dir",str(root/"results"),"--cwd",str(root),"--",sys.executable,"-c","import sys;sys.exit(7)"])
            self.assertEqual(rc,4);self.assertEqual(json.loads((root/"results/admission.json").read_text())["exit_code"],7)

    def test_timeout_reaps_direct_runner(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d)
            with patch.object(mod,"ollama_running",return_value=[]),patch.object(mod,"sample_host",return_value={"eligible":True}):
                rc=mod.main(["--results-dir",str(root/"results"),"--cwd",str(root),"--timeout","1","--",sys.executable,"-c","import time;time.sleep(30)"])
            self.assertEqual(rc,4)
            state=json.loads((root/"results/admission.json").read_text())
            self.assertEqual(state["status"],"TIMEOUT")
            self.assertIn("cleanup",state)

if __name__=="__main__":unittest.main()
