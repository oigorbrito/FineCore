import hashlib
import io
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from release_budget_01 import make_archive, render


class ReleaseBudgetArchiveTests(unittest.TestCase):
    def test_tar_gz_is_deterministic_and_contains_one_file(self):
        data = b"FineCore executable fixture\0" * 100
        first = make_archive(data, "tar.gz", "finecore")
        second = make_archive(data, "tar.gz", "finecore")
        self.assertEqual(first, second)
        import tarfile

        with tarfile.open(fileobj=io.BytesIO(first), mode="r:gz") as archive:
            self.assertEqual(archive.getnames(), ["finecore"])
            self.assertEqual(archive.extractfile("finecore").read(), data)

    def test_zip_is_deterministic_and_contains_one_file(self):
        data = b"windows executable fixture" * 100
        first = make_archive(data, "zip", "finecore.exe")
        second = make_archive(data, "zip", "finecore.exe")
        self.assertEqual(first, second)
        with zipfile.ZipFile(io.BytesIO(first)) as archive:
            self.assertEqual(archive.namelist(), ["finecore.exe"])
            self.assertEqual(archive.read("finecore.exe"), data)

    def test_manifest_checks_input_hash_before_measurement(self):
        data = b"recorded binary"
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / "finecore"
            binary.write_bytes(data)
            manifest = root / "inputs.json"
            manifest.write_text(json.dumps({"inputs": [{
                "label": "finecore-test",
                "path": str(binary),
                "sha256": hashlib.sha256(data).hexdigest(),
                "format": "tar.gz",
                "member": "finecore",
            }]}), encoding="utf-8")
            result = render(manifest)["rows"][0]
            self.assertEqual(result["binary_bytes"], len(data))
            self.assertEqual(result["binary_sha256"], hashlib.sha256(data).hexdigest())
            self.assertGreater(result["archive_bytes"], 0)

            manifest.write_text(manifest.read_text(encoding="utf-8").replace(
                hashlib.sha256(data).hexdigest(), "0" * 64), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                render(manifest)

    def test_rejects_paths_and_unknown_formats(self):
        with self.assertRaisesRegex(ValueError, "root-level filename"):
            make_archive(b"x", "zip", "../finecore.exe")
        with self.assertRaisesRegex(ValueError, "unsupported archive format"):
            make_archive(b"x", "7z", "finecore")


if __name__ == "__main__":
    unittest.main()
