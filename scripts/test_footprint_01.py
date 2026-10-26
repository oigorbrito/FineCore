import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import footprint_01 as f


class FootprintAnalysisTests(unittest.TestCase):
    def test_parses_benchmark_json_lines(self):
        output = '[core-tools-bench] {"scenario":"append_files","wall_seconds":0.2}\n'
        self.assertEqual(f.benchmark_json(output)["append_files"]["wall_seconds"], 0.2)

    def test_paired_summary_reports_zero_for_equal_samples(self):
        result = f.summarize([0.0, 0.0, 0.0], seed=7)
        self.assertEqual(result["median_percent"], 0.0)
        self.assertEqual(result["ci95_percent"], [0.0, 0.0])
        self.assertEqual(result["n_pairs"], 3)

    def test_variants_are_isolated_and_do_not_change_size_policy(self):
        self.assertEqual(len({v["name"] for v in f.VARIANTS}), len(f.VARIANTS))
        self.assertEqual(f.VARIANTS[0]["release"], {})
        self.assertTrue(all("strip" not in v.get("release", {}) for v in f.VARIANTS))
        self.assertTrue(f.VARIANTS[-2]["diagnostic_only"])

    def test_bootstrap_is_deterministic(self):
        values = [-2.0, 0.5, 1.0, 2.0]
        self.assertEqual(f.summarize(values, 123), f.summarize(values, 123))

    def test_log_reference_does_not_duplicate_logs_directory(self):
        root = Path("/tmp/evidence/logs")
        self.assertEqual(f.resolve_log_path(root, "logs/rustc.log"), root / "rustc.log")

    def test_unsupported_legacy_plugin_help_probes_are_not_functional_gates(self):
        record = {"compatibility": [
            {"command": ["finecore", "--version"], "exit_code": 0},
            {"command": ["finecore", "--help"], "exit_code": 0},
            {"command": ["finecore", "plugin", "--help"], "exit_code": 1},
            {"command": ["finecore", "plugin", "validate", "--help"], "exit_code": 1},
        ]}
        self.assertEqual(len(f.valid_compatibility_rows(record)), 2)

    def test_invoke_streams_and_preserves_command_output(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            logs = root / "logs"
            records = []
            result = f.invoke("stream-test", [sys.executable, "-c", "print('live-output')"],
                              root, f.os.environ.copy(), logs, records)
            self.assertEqual(result["exit_code"], 0)
            self.assertEqual((logs / "stream-test.log").read_text().strip(), "live-output")
            self.assertEqual(records, [result])

    def test_completed_variant_checkpoint_is_atomic_and_cumulative(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            evidence = {"commands": [], "variants": {}}
            f.checkpoint_variant(root, evidence, "finecore", "baseline", {"binary": {"bytes": 1}})
            self.assertTrue((root / "checkpoints/finecore-baseline.json").is_file())
            partial = __import__("json").loads((root / "footprint-01-partial-results.json").read_text())
            self.assertEqual(partial["completed_variants"], ["finecore/baseline"])

    def test_report_includes_unmodified_gate_and_candidate_disposition(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            out = root / "evidence"
            logs = out / "logs"
            logs.mkdir(parents=True)
            evidence = {
                "repositories": {
                    "finecore": {"head": "fine-head"},
                    "orca": {"head": "orca-head"},
                },
                "environment": {"platform": "Linux-test", "machine": "x86_64"},
                "dependency_graph_match": True,
                "release_profile_baseline_match": True,
                "tool_versions": {"rustc": {"log": "logs/rustc.log"}},
                "variants": {
                    "baseline": {
                        "finecore": {"binary": {"bytes": 9_000_000}, "commands": []},
                        "orca": {"binary": {"bytes": 9_100_000}, "commands": []},
                    },
                    "thin-lto": {
                        "finecore": {
                            "profile": {"name": "thin-lto", "release": {"LTO": "thin"}},
                            "binary": {"bytes": 8_900_000},
                            "commands": [
                                {"label": "finecore-thin-lto-release-build", "exit_code": 0},
                                {"label": "finecore-thin-lto-release-workspace-tests", "exit_code": 0},
                                {"label": "finecore-thin-lto-benchmark-smoke", "exit_code": 0},
                                *[{"label": f"finecore-thin-lto-mcp-{i:02d}", "exit_code": 0} for i in range(1, 21)],
                            ],
                            "mcp_passed": 20,
                            "reproducibility": {"same_sha256": True},
                            "compatibility": [
                                {"command": ["finecore", "--version"], "exit_code": 0},
                                {"command": ["finecore", "--help"], "exit_code": 0},
                                {"command": ["finecore", "plugin", "list"], "exit_code": 0},
                            ],
                        }
                    },
                },
                "summary": {
                    "minimum_functionally_passing_non_diagnostic_linux_build": {
                        "variant": "thin-lto",
                        "bytes": 8_900_000,
                        "bytes_over_limit": 2_200_000,
                    },
                    "baseline_bytes_over_limit": 2_300_000,
                },
                "feature_graphs_identical": True,
                "startup": {},
                "benchmarks": {},
            }
            (logs / "rustc.log").write_text("rustc test-version\n", encoding="utf-8")
            f.render_report(evidence, logs, out)
            report = (out / "FOOTPRINT_01_REPORT.md").read_text(encoding="utf-8")
            self.assertIn("thin-lto", report)
            self.assertIn("6,700,000", report)
            self.assertIn("Reference; keep unchanged", report)


if __name__ == "__main__":
    unittest.main()
