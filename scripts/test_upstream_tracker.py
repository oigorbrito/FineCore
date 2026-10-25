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


    def test_upstream_title_with_html_comment_terminator_is_safe(self):
        key = "okikorg/orca-harness:issue:42"
        state = {"title": 'text "} --> injected tail', "updated_at": "today"}
        body = tracker.marker(key, state)
        self.assertEqual(body.count("-->"), 1)
        self.assertEqual(tracker.parse(body), (key, state))

    def test_preexisting_unescaped_marker_with_nested_json_is_readable(self):
        key = "okikorg/orca-harness:pr:10"
        body = ('<!-- finecore-upstream:' + key +
                ' state:{"title":"value } --> suffix","nested":{"old":"new"}} -->')
        self.assertEqual(tracker.parse(body),
                         (key, {"title": "value } --> suffix", "nested": {"old": "new"}}))

    def test_corrupt_tracking_marker_fails_closed(self):
        with self.assertRaises((ValueError, __import__("json").JSONDecodeError)):
            tracker.parse('<!-- finecore-upstream:repo:issue:1 state:{"broken": -->')


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


class EndToEndTrackerTests(unittest.TestCase):
    def test_three_polls_create_replay_and_update_without_upstream_writes(self):
        """Exercise main() against a fully in-memory GitHub API on each fresh poll."""
        from io import StringIO
        from contextlib import redirect_stdout
        import json

        destination = "oigorbrito/finecore"
        upstream = "okikorg/orca-harness"
        created = {}
        calls = []
        state = {"pr_title": "Initial", "pr_head": "head-A", "branch_head": "branch-A"}

        def fake_pages(path):
            if path == f"/repos/{destination}/issues?state=all":
                return list(created.values())
            if path.startswith(f"/repos/{upstream}/issues?state=all"):
                return [
                    {"number": 22, "title": state["pr_title"], "updated_at": "2026-10-25T00:00:00Z",
                     "state": "open", "html_url": f"https://github.com/{upstream}/pull/22", "pull_request": {}},
                    {"number": 23, "title": "A real issue", "updated_at": "2026-10-25T00:00:00Z",
                     "state": "open", "html_url": f"https://github.com/{upstream}/issues/23"},
                ]
            if path == f"/repos/{upstream}/pulls/22/commits":
                return [{"sha": state["pr_head"]}]
            if path == f"/repos/{upstream}/releases":
                return [{"id": 8, "html_url": "https://example.com/release", "tag_name": "v1",
                         "name": "Version 1", "published_at": "2026-10-01", "updated_at": "2026-10-01",
                         "draft": False, "prerelease": False, "target_commitish": "main"}]
            if path == f"/repos/{upstream}/tags":
                return [{"name": "v1", "commit": {"sha": "tag-A"}}]
            raise AssertionError(f"unexpected endpoint pages: {path}")

        def fake_request(method, path, payload=None):
            calls.append((method, path))
            if path.startswith(f"/repos/{upstream}/") or path == f"/repos/{upstream}":
                self.assertEqual(method, "GET", "never write to the upstream")
                if path == f"/repos/{upstream}":
                    return {"default_branch": "main"}
                if path == f"/repos/{upstream}/commits/main":
                    return {"sha": state["branch_head"], "html_url": f"https://example.com/{state['branch_head']}"}
                if path == f"/repos/{upstream}/pulls/22":
                    return {"head": {"sha": state["pr_head"]}, "base": {"sha": "base-A"},
                            "draft": False, "merged_at": None}
                raise AssertionError(path)
            self.assertTrue(path.startswith(f"/repos/{destination}/issues"))
            self.assertIn(method, {"POST", "PATCH"})
            if method == "POST":
                number = len(created) + 1
                created[number] = {"number": number, "title": payload["title"], "body": payload["body"]}
                return {"number": number}
            number = int(path.rsplit("/", 1)[1])
            created[number].update(payload)
            return created[number]

        with patch.object(tracker, "TOKEN", "test-token"), \
             patch.object(tracker, "DEST", destination), \
             patch.object(tracker, "UPSTREAMS", [upstream]), \
             patch.object(tracker, "pages", side_effect=fake_pages), \
             patch.object(tracker, "request", side_effect=fake_request):
            def poll():
                out = StringIO()
                with redirect_stdout(out):
                    tracker.main()
                return json.loads(out.getvalue())

            first = poll()
            self.assertEqual(first["stats"], {"created": 5, "updated": 0, "unchanged": 0})
            self.assertEqual(len(created), 5)
            first_mutations = len([v for v in calls if v[0] in {"POST", "PATCH"}])
            second = poll()
            self.assertEqual(second["stats"], {"created": 0, "updated": 0, "unchanged": 5})
            self.assertEqual(len([v for v in calls if v[0] in {"POST", "PATCH"}]), first_mutations)
            state.update(pr_title="Renamed PR", pr_head="head-B", branch_head="branch-B")
            third = poll()
            self.assertEqual(third["stats"], {"created": 0, "updated": 2, "unchanged": 3})
            self.assertEqual(len(created), 5)
            mutations = [v for v in calls if v[0] in {"POST", "PATCH"}]
            self.assertEqual(len(mutations), first_mutations + 2)
            self.assertTrue(all(p.startswith(f"/repos/{destination}/issues") for m,p in mutations))
            self.assertFalse(any("/comments" in p for _, p in mutations))


class RetrySafetyTests(unittest.TestCase):
    def test_ambiguous_post_http_failure_is_not_retried(self):
        """A repeated issue POST could duplicate a record after a lost response."""
        from urllib.error import HTTPError
        error = HTTPError("https://api.github.com/repos/oigorbrito/finecore/issues", 503,
                          "Service Unavailable", None, None)
        with patch.object(tracker.urllib.request, "urlopen", side_effect=error) as send, \
             patch.object(tracker.time, "sleep") as sleep:
            with self.assertRaisesRegex(RuntimeError, "HTTP 503"):
                tracker.request("POST", "/repos/oigorbrito/finecore/issues", {"title": "test"})
        send.assert_called_once()
        sleep.assert_not_called()

    def test_get_http_failure_retries_only_bounded_attempts(self):
        from urllib.error import HTTPError
        error = HTTPError("https://api.github.com/repos/okikorg/orca-harness", 503,
                          "Service Unavailable", None, None)
        with patch.object(tracker.urllib.request, "urlopen", side_effect=error) as send, \
             patch.object(tracker.time, "sleep") as sleep:
            with self.assertRaisesRegex(RuntimeError, "HTTP 503"):
                tracker.request("GET", "/repos/okikorg/orca-harness")
        self.assertEqual(send.call_count, 4)
        self.assertEqual(sleep.call_count, 3)


class MutationRateTests(unittest.TestCase):
    def test_mutation_spacing_is_at_least_one_second(self):
        with patch.object(tracker, "LAST_WRITE_STARTED", 100.0), \
             patch.object(tracker.time, "monotonic", side_effect=[100.2, 101.0]), \
             patch.object(tracker.time, "sleep") as sleep:
            tracker.pace_write()
            sleep.assert_called_once()
            self.assertAlmostEqual(sleep.call_args.args[0], 0.8)
            self.assertEqual(tracker.LAST_WRITE_STARTED, 101.0)
