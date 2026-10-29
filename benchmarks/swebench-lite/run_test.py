import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import run


class RunTest(unittest.TestCase):
    def test_parse_args_supports_finecore_and_legacy_binary_flags(self):
        cases = [
            (["--finecore", "/tmp/finecore"], "/tmp/finecore"),
            (["--orcacode", "/tmp/orcacode"], "/tmp/orcacode"),
            (
                ["--finecore", "/tmp/new", "--orcacode", "/tmp/legacy"],
                "/tmp/legacy",
            ),
            (
                ["--orcacode", "/tmp/legacy", "--finecore", "/tmp/new"],
                "/tmp/new",
            ),
        ]
        for arguments, expected in cases:
            with self.subTest(arguments=arguments), patch.object(
                sys, "argv", ["run.py", *arguments]
            ):
                self.assertEqual(run.parse_args().finecore, expected)

    def test_selection_is_deterministic(self):
        instances = [{"instance_id": str(index)} for index in range(5)]
        self.assertEqual(run.select_instances(instances, 3), instances[:3])

    def test_prediction_uses_official_fields(self):
        value = run.prediction({"instance_id": "owner__repo-1"}, "model", "diff")
        self.assertEqual(
            value,
            {
                "instance_id": "owner__repo-1",
                "model_name_or_path": "finecore+model",
                "model_patch": "diff",
            },
        )
        json.dumps(value)

    def test_decoded_output_handles_timeout_bytes(self):
        self.assertEqual(run.decoded_output(b"partial\xff"), "partial�")
        self.assertEqual(run.decoded_output(None), "")

    def test_command_is_isolated_and_can_edit_and_test(self):
        args = SimpleNamespace(
            model="example/model", max_steps=12, max_output_tokens=2048
        )
        instance = {"instance_id": "owner__repo-1", "problem_statement": "Fix it."}
        command = run.build_command("/bin/finecore", Path("/tmp/repo"), instance, args)
        self.assertIn("--bare", command)
        self.assertIn("--yolo", command)
        self.assertIn("--no-session", command)
        tools = command[command.index("--tools") + 1]
        self.assertIn("shell", tools)
        self.assertIn("edit_file", tools)
        self.assertIn("Do not modify\ntests", command[-1])

    def test_capture_patch_excludes_untracked_scratch_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            run.run_checked(["git", "init", "--quiet"], cwd=repo)
            run.run_checked(["git", "config", "user.email", "bench@example.com"], cwd=repo)
            run.run_checked(["git", "config", "user.name", "Benchmark"], cwd=repo)
            (repo / "tracked.py").write_text("old\n", encoding="utf-8")
            run.run_checked(["git", "add", "."], cwd=repo)
            run.run_checked(["git", "commit", "--quiet", "-m", "base"], cwd=repo)
            (repo / "tracked.py").write_text("new\n", encoding="utf-8")
            (repo / "added.py").write_text("added\n", encoding="utf-8")
            patch = run.capture_patch(repo)
        self.assertIn("tracked.py", patch)
        self.assertNotIn("added.py", patch)

    def test_parse_summary_extracts_usage(self):
        stdout = '\n'.join(
            [
                '{"type":"text_delta","delta":"done"}',
                '{"type":"summary","modelSteps":3,"toolCalls":2,"usage":{"outputTokens":9}}',
            ]
        )
        self.assertEqual(
            run.parse_summary(stdout),
            {"model_steps": 3, "tool_calls": 2, "usage": {"outputTokens": 9}},
        )


if __name__ == "__main__":
    unittest.main()
