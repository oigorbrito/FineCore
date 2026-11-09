import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import audit_crates


class SourceSizeBaselineTests(unittest.TestCase):
    def test_unchanged_imported_overage_is_reported_but_passes(self):
        self.assertEqual(
            "baseline",
            audit_crates.classify_size("crates/a.rs", 650, 599, {"crates/a.rs": 650}),
        )

    def test_reduced_imported_file_is_within_policy(self):
        self.assertEqual(
            "ok",
            audit_crates.classify_size("crates/a.rs", 599, 599, {"crates/a.rs": 650}),
        )

    def test_growth_past_imported_ceiling_fails(self):
        self.assertEqual(
            "regression",
            audit_crates.classify_size("crates/a.rs", 651, 599, {"crates/a.rs": 650}),
        )

    def test_new_over_limit_file_fails(self):
        self.assertEqual(
            "regression",
            audit_crates.classify_size("crates/new.rs", 600, 599, {}),
        )

    def test_scanner_keeps_all_original_source_extensions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            crate = root / "crates/example"
            crate.mkdir(parents=True)
            for suffix in (".rs", ".inc", ".part", ".md"):
                (crate / f"file{suffix}").write_text("line\n", encoding="utf-8")
            paths = {str(stat.path) for stat in audit_crates.audit_crates(root, 599)}
        self.assertEqual(
            {"crates/example/file.rs", "crates/example/file.inc", "crates/example/file.part"},
            paths,
        )


if __name__ == "__main__":
    unittest.main()
