"""Offline contract tests for FineCore's independent Orca metadata tracker."""
import unittest
from unittest.mock import patch

import upstream_tracker as tracker


class MarkerTests(unittest.TestCase):
    def test_roundtrip_and_stable_state(self):
        key = "okikorg/orca-harness:tag:orcacode-v0.9.0"
        state = {"sha": "abc", "deleted": False}
        body = "hello\n" + tracker.marker(key, state)
        self.assertEqual(tracker.parse(body), (key, state))
        self.assertIsNone(tracker.parse("ordinary issue without a marker"))


class SnapshotTests(unittest.TestCase):
    def test_initial_tag_creates_one_issue(self):
        indexed = {}
        state = {"sha": "abc", "deleted": False}
        with patch.object(tracker, "request", return_value={"number": 9}) as request:
            result = tracker.sync_record(
                "okikorg/orca-harness", "tag", "v1",
                "https://github.com/okikorg/orca-harness/releases/tag/v1",
                state, indexed)
        self.assertEqual(result, "created")
        request.assert_called_once()
        self.assertEqual(request.call_args.args[:2],
                         ("POST", f"/repos/{tracker.DEST}/issues"))
        self.assertEqual(indexed["okikorg/orca-harness:tag:v1"], (9, state))

    def test_replay_is_noop(self):
        state = {"sha": "abc", "deleted": False}
        indexed = {"okikorg/orca-harness:tag:v1": (9, state)}
        with patch.object(tracker, "request") as request:
            result = tracker.sync_record(
                "okikorg/orca-harness", "tag", "v1", "https://example.com", state, indexed)
        self.assertEqual(result, "unchanged")
        request.assert_not_called()

    def test_tag_advancement_updates_once_without_comments(self):
        original = {"sha": "abc", "deleted": False}
        advanced = {"sha": "def", "deleted": False}
        indexed = {"okikorg/orca-harness:tag:v1": (9, original)}
        with patch.object(tracker, "request") as request:
            result = tracker.sync_record(
                "okikorg/orca-harness", "tag", "v1", "https://example.com", advanced, indexed)
        self.assertEqual(result, "updated")
        request.assert_called_once()
        self.assertEqual(request.call_args.args[:2], ("PATCH", f"/repos/{tracker.DEST}/issues/9"))
        self.assertEqual(indexed["okikorg/orca-harness:tag:v1"], (9, advanced))

    def test_pr_snapshot_changed_updates_without_duplicate_comment(self):
        old_state = {"title": "old"}
        new_state = {"title": "new"}
        item = {"number": 7, "title": "new", "html_url": "https://github.com/okikorg/orca-harness/pull/7"}
        indexed = {"okikorg/orca-harness:pr:7": (12, old_state)}
        with patch.object(tracker, "snapshot", return_value=new_state), \
             patch.object(tracker, "request") as request:
            result = tracker.sync_one("okikorg/orca-harness", "pr", item, indexed)
        self.assertEqual(result, "updated")
        request.assert_called_once()
        self.assertEqual(request.call_args.args[:2], ("PATCH", f"/repos/{tracker.DEST}/issues/12"))

    def test_default_branch_changed_updates_without_comment(self):
        old_state = {"branch": "main", "sha": "old", "url": "https://example.com/old"}
        indexed = {"okikorg/orca-harness:default-branch": (11, old_state)}
        def fake_request(method, path, payload=None):
            self.assertEqual(method, "GET" if path.startswith("/repos/okikorg") else "PATCH")
            if path == "/repos/okikorg/orca-harness":
                return {"default_branch": "main"}
            if path == "/repos/okikorg/orca-harness/commits/main":
                return {"sha": "new", "html_url": "https://example.com/new"}
            return {}
        with patch.object(tracker, "request", side_effect=fake_request) as request:
            result = tracker.sync_branch("okikorg/orca-harness", indexed)
        self.assertEqual(result, "updated")
        self.assertEqual(request.call_count, 3)
        self.assertEqual(indexed["okikorg/orca-harness:default-branch"][1]["sha"], "new")


class CollectionTests(unittest.TestCase):
    def test_tag_and_release_are_independent_and_replay_is_idempotent(self):
        upstream = "okikorg/orca-harness"
        indexed = {}
        release = {
            "id": 17, "html_url": "https://example.com/release",
            "tag_name": "v1", "name": "Version 1",
            "draft": False, "prerelease": False,
            "published_at": "2026-10-01T00:00:00Z",
            "updated_at": "2026-10-01T00:00:00Z",
            "target_commitish": "main",
        }
        tag = {"name": "v1", "commit": {"sha": "abc"}}
        def fake_pages(path):
            return [release] if path.endswith("/releases") else [tag]
        next_number = iter((10, 11))
        def fake_request(method, path, payload=None):
            if method != "POST":
                raise AssertionError("Unexpected update during replay")
            return {"number": next(next_number)}
        stats = {"created": 0, "updated": 0, "unchanged": 0}
        with patch.object(tracker, "pages", side_effect=fake_pages), \
             patch.object(tracker, "request", side_effect=fake_request) as request:
            tracker.sync_releases_and_tags(upstream, indexed, stats)
            tracker.sync_releases_and_tags(upstream, indexed, stats)
        self.assertEqual(stats, {"created": 2, "updated": 0, "unchanged": 2})
        self.assertEqual(request.call_count, 2)
        self.assertIn(f"{upstream}:tag:v1", indexed)
        self.assertIn(f"{upstream}:release:17", indexed)

    def test_missing_tag_is_marked_deleted_once(self):
        upstream = "okikorg/orca-harness"
        key = f"{upstream}:tag:old"
        indexed = {key: (9, {"sha": "previous", "deleted": False})}
        stats = {"created": 0, "updated": 0, "unchanged": 0}
        with patch.object(tracker, "pages", return_value=[]), \
             patch.object(tracker, "request") as request:
            tracker.sync_releases_and_tags(upstream, indexed, stats)
            tracker.sync_releases_and_tags(upstream, indexed, stats)
        self.assertEqual(stats, {"created": 0, "updated": 1, "unchanged": 0})
        request.assert_called_once()
        self.assertTrue(indexed[key][1]["deleted"])

    def test_partial_pagination_does_not_trigger_deletion(self):
        upstream = "okikorg/orca-harness"
        key = f"{upstream}:tag:old"
        indexed = {key: (9, {"sha": "previous", "deleted": False})}
        stats = {"created": 0, "updated": 0, "unchanged": 0}
        def failing_pages(path):
            if path.endswith("/tags"):
                raise RuntimeError("incomplete pagination")
            return []
        with patch.object(tracker, "pages", side_effect=failing_pages), \
             patch.object(tracker, "request") as request:
            with self.assertRaisesRegex(RuntimeError, "incomplete"):
                tracker.sync_releases_and_tags(upstream, indexed, stats)
        request.assert_not_called()
        self.assertFalse(indexed[key][1]["deleted"])


if __name__ == "__main__":
    unittest.main()
