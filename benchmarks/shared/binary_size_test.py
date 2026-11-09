"""Exercise the release size gate used by CI with exact byte boundaries."""

import pathlib
import subprocess
import tempfile
import unittest


GATE = pathlib.Path(__file__).resolve().parents[2] / "ci/check-binary-size.sh"


class BinarySizeTests(unittest.TestCase):
    def test_strict_decimal_mb_limit(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / "orcacode"
            for size, expected in [(6_699_999, 0), (6_700_000, 1), (6_700_001, 1)]:
                with self.subTest(size=size):
                    with binary.open("wb") as handle:
                        handle.truncate(size)
                    result = subprocess.run(
                        ["bash", str(GATE), str(binary)], capture_output=True, text=True
                    )
                    self.assertEqual(expected, result.returncode, result.stdout + result.stderr)

    def test_linux_musl_targets_have_their_own_limit(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = pathlib.Path(tmp) / "x86_64-unknown-linux-musl" / "release"
            directory.mkdir(parents=True)
            binary = directory / "orcacode"
            for size, expected in [(8_999_999, 0), (9_000_000, 1)]:
                with self.subTest(size=size):
                    with binary.open("wb") as handle:
                        handle.truncate(size)
                    result = subprocess.run(
                        ["bash", str(GATE), str(binary)], capture_output=True, text=True
                    )
                    self.assertEqual(expected, result.returncode, result.stdout + result.stderr)

    def test_missing_binary_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = subprocess.run(
                ["bash", str(GATE), str(pathlib.Path(tmp) / "missing")],
                capture_output=True, text=True,
            )
            self.assertEqual(1, result.returncode)
            self.assertIn("missing release binary", result.stderr)

    def test_imported_linux_baseline_passes_without_growth(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / "target" / "release" / "orcacode"
            binary.parent.mkdir(parents=True)
            with binary.open("wb") as handle:
                handle.truncate(9_415_792)
            result = subprocess.run(
                ["bash", str(GATE), "--baseline-orca-d89"],
                cwd=tmp,
                capture_output=True,
                text=True,
            )
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            self.assertIn("inherited Orca d89bf274 baseline", result.stdout)

    def test_imported_linux_baseline_rejects_growth(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / "target" / "release" / "orcacode"
            binary.parent.mkdir(parents=True)
            with binary.open("wb") as handle:
                handle.truncate(9_415_793)
            result = subprocess.run(
                ["bash", str(GATE), "--baseline-orca-d89"],
                cwd=tmp,
                capture_output=True,
                text=True,
            )
            self.assertEqual(1, result.returncode)
            self.assertIn("exceeds the imported Orca d89bf274 baseline", result.stderr)

    def test_baseline_allowance_cannot_be_applied_to_other_targets(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = pathlib.Path(tmp) / "other" / "orcacode"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"x")
            result = subprocess.run(
                ["bash", str(GATE), "--baseline-orca-d89", str(binary)],
                cwd=tmp,
                capture_output=True,
                text=True,
            )
            self.assertEqual(1, result.returncode)
            self.assertIn("restricted to target/release/orcacode", result.stderr)


if __name__ == "__main__":
    unittest.main()
