#!/usr/bin/env python3
"""Focused regression tests for contract literal detection; no donor checkout required."""
import importlib.util
import pathlib
import unittest

MODULE = pathlib.Path(__file__).with_name("audit-orca-contracts.py")
spec = importlib.util.spec_from_file_location("orca_contracts", MODULE)
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)

class ContractPatternTests(unittest.TestCase):
    def matches(self, category, text):
        return [m.group(0) for m in audit.PATTERNS[category].finditer(text)]

    def test_environment(self):
        self.assertEqual(self.matches("environment", 'std::env::var("ORCA_CONFIG_DIR")'), ["ORCA_CONFIG_DIR"])
        self.assertEqual(self.matches("environment", "NOT_ORCA_MODEL"), [])

    def test_persistence_dot_prefix(self):
        self.assertEqual(self.matches("persistence", 'workspace/.orca/config.json'),
                         [".orca", "config.json"])

    def test_headless_flags(self):
        self.assertEqual(self.matches("protocol", 'orcacode --json --resume --continue'),
                         ["--json", "--resume", "--continue"])

    def test_rust_event_serialization(self):
        self.assertEqual(self.matches("protocol", "serde_json::to_string(&HarnessEvent::AgentStart)"),
                         ["serde_json::to_string", "HarnessEvent"])

    def test_identity(self):
        self.assertEqual(self.matches("identity", "orca_harness_sdk and orca-harness-core"),
                         ["orca_harness_sdk", "orca-harness-core"])

if __name__ == "__main__":
    unittest.main()
