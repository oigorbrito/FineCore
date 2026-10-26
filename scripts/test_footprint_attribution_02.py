from __future__ import annotations

import hashlib
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import footprint_attribution_02 as f


class AttributionPreflightTests(unittest.TestCase):
    def test_lock_graph_is_stable_and_external_only(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "Cargo.lock").write_text(
                'version = 4\n\n[[package]]\nname = "workspace-app"\nversion = "0.1.0"\n\n'
                '[[package]]\nname = "bumpalo"\nversion = "3.20.3"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                'checksum = "abc"\ndependencies = ["serde 1.0.0"]\n', encoding="utf-8")
            _lock, packages, graph = f.lock_data(root)
            self.assertEqual(len(packages), 1)
            self.assertEqual(packages[0][0:2], ("bumpalo", "3.20.3"))
            self.assertEqual(graph, f.sha256(
                __import__("json").dumps(packages, separators=(",", ":")).encode()))

    def test_cache_verification_requires_exact_registry_checksum(self):
        payload = b"verified registry crate fixture"
        checksum = hashlib.sha256(payload).hexdigest()
        with tempfile.TemporaryDirectory() as temp:
            cargo_home = Path(temp)
            cache = cargo_home / "registry/cache/index-fixture"
            cache.mkdir(parents=True)
            (cache / "bumpalo-3.20.3.crate").write_bytes(payload)
            package = ("bumpalo", "3.20.3",
                       "registry+https://github.com/rust-lang/crates.io-index", checksum, [])
            verified, missing = f.verify_registry_cache([package], cargo_home)
            self.assertEqual(len(verified), 1)
            self.assertEqual(missing, [])

    def test_cache_verification_rejects_checksum_mismatch(self):
        with tempfile.TemporaryDirectory() as temp:
            cargo_home = Path(temp)
            cache = cargo_home / "registry/cache/index-fixture"
            cache.mkdir(parents=True)
            (cache / "bumpalo-3.20.3.crate").write_bytes(b"wrong")
            package = ("bumpalo", "3.20.3",
                       "registry+https://github.com/rust-lang/crates.io-index", "0" * 64, [])
            verified, missing = f.verify_registry_cache([package], cargo_home)
            self.assertEqual(verified, [])
            self.assertEqual(missing[0]["name"], "bumpalo")
            self.assertTrue(missing[0]["archive_found"])


if __name__ == "__main__":
    unittest.main()
