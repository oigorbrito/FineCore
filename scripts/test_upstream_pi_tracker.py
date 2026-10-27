"""Independent offline regression tests for Pi upstream metadata watch."""
import datetime as dt
import json
import unittest
from unittest.mock import patch

import upstream_pi_tracker as tracker


class CodecTests(unittest.TestCase):
    def test_marker_roundtrip_and_comment_escape(self):
        state = {"head": "a" * 40, "title": "evil --> closing", "nested": {"a": 2}}
        body = tracker.marker(state)
        self.assertEqual(body.count("-->"), 1)
        self.assertEqual(tracker.parse_marker(body), state)
        self.assertIsNone(tracker.parse_marker("user-created issue"))
        with self.assertRaises(ValueError):
            tracker.parse_marker(body + body)

    def test_truncated_marker_fails_closed(self):
        with self.assertRaises((ValueError, json.JSONDecodeError)):
            tracker.parse_marker("<!-- finecore-pi-monitor:{not-json} -->")


class PaginationTests(unittest.TestCase):
    def test_recent_feed_stops_at_cutoff(self):
        values = [
            {"number": 3, "updated_at": "2026-10-25T10:00:00Z"},
            {"number": 2, "updated_at": "2026-10-12T10:00:00Z"},
            {"number": 1, "updated_at": "2026-10-11T10:00:00Z"},
        ]
        with patch.object(tracker, "api", return_value=values):
            got = list(tracker.collection("/repos/earendil-works/pi/pulls?state=all",
                                          cutoff="2026-10-12T00:00:00Z"))
        self.assertEqual([r["number"] for r in got], [3, 2])

    def test_out_of_order_recent_feed_fails_closed(self):
        values = [
            {"updated_at": "2026-10-21T10:00:00Z"},
            {"updated_at": "2026-10-23T10:00:00Z"},
        ]
        with patch.object(tracker, "api", return_value=values):
            with self.assertRaisesRegex(RuntimeError, "ordering"):
                list(tracker.collection("/repos/x/pulls", cutoff="2026-10-01T00:00:00Z"))

    def test_max_pages_is_hard_failure_not_partial_inventory(self):
        with patch.object(tracker, "MAX_PAGES", 2), patch.object(tracker, "api",
            return_value=[{"number": 1, "updated_at": "2026-10-26T00:00:00Z"}] * 100):
            with self.assertRaisesRegex(RuntimeError, "Pagination exceeded"):
                list(tracker.collection("/repos/earendil-works/pi/pulls"))


class RestInventoryTests(unittest.TestCase):
    def test_full_pr_history_is_paged_and_type_checked(self):
        def pages(path, cutoff=None):
            self.assertIn("/pulls?state=all", path)
            return [
                {"number": 10, "head": {"sha": "a" * 40},
                 "html_url": "https://github.com/earendil-works/pi/pull/10"},
                {"number": 9, "head": {"sha": "b" * 40},
                 "html_url": "https://github.com/earendil-works/pi/pull/9"},
            ]
        with patch.object(tracker, "collection", side_effect=pages) as called:
            items = tracker.complete_pi_pull_requests()
        self.assertEqual(len(items), 2)
        called.assert_called_once()

    def test_duplicate_or_non_pr_fails_without_downstream_write(self):
        valid = {"number": 1, "head": {"sha": "a" * 40},
                 "html_url": "https://github.com/earendil-works/pi/pull/1"}
        for records in [
            [valid, valid],
            [{"number": 2, "head": {"sha": "b" * 40},
              "html_url": "https://github.com/earendil-works/pi/issues/2"}],
            [{"number": 3, "html_url": "https://github.com/earendil-works/pi/pull/3"}],
        ]:
            with self.subTest(records=records), patch.object(tracker, "collection", return_value=records):
                with self.assertRaises(RuntimeError):
                    tracker.complete_pi_pull_requests()

    def test_since_query_collects_full_issue_feed_and_excludes_pull_requests(self):
        records = [
            {"number": 1, "updated_at": "2026-10-25T00:00:00Z"},
            {"number": 2, "pull_request": {}, "updated_at": "2026-10-25T00:00:00Z"},
            {"number": 3, "updated_at": "2026-10-01T00:00:00Z"},
        ]
        with patch.object(tracker, "collection", return_value=records) as called:
            issues = tracker.complete_recent_issues("2026-10-12T00:00:00Z")
        self.assertEqual([i["number"] for i in issues], [1])
        self.assertIn("since=", called.call_args.args[0])


class InventoryTests(unittest.TestCase):
    def test_merges_older_open_pr_with_recent_and_tracks_main(self):
        main = "a" * 40
        def request(method, path, payload=None):
            self.assertEqual(method, "GET")
            if path == "/repos/earendil-works/pi":
                return {"default_branch": "main"}
            if path.endswith("/commits/main"):
                return {"sha": main}
            raise AssertionError(path)
        def pages(path, cutoff=None):
            if "/pulls?state=open" in path:
                return [self.pr(8, "2026-01-01T00:00:00Z"),
                        self.pr(10, "2026-10-25T01:00:00Z")]
            if "/releases" in path:
                return [{"id": 1, "tag_name": "v1", "published_at": "2026-10-24T00:00:00Z",
                         "updated_at": "2026-10-24T00:00:00Z", "draft": False}]
            raise AssertionError(path)
        def search(kind, cutoff):
            if kind == "pr":
                return [self.pr(10, "2026-10-25T01:00:00Z")]
            if kind == "issue":
                return [{"number": 7, "state": "open", "updated_at": "2026-10-25T01:00:00Z"}]
            raise AssertionError(kind)
        with patch.object(tracker, "api", side_effect=request), patch.object(
            tracker, "collection", side_effect=pages), patch.object(
            tracker, "search_recent", side_effect=search):
            state, report = tracker.source_snapshot(
                dt.datetime(2026, 10, 26, tzinfo=dt.timezone.utc))
        self.assertEqual(state["head"], main)
        self.assertEqual(report["counts"]["open_prs"], 2)
        self.assertEqual(report["counts"]["recent_prs"], 1)
        self.assertEqual(len(report["full_recent_prs"]), 2)
        self.assertEqual(report["counts"]["recent_issues"], 1)
        self.assertEqual(state["latest_release"]["tag"], "v1")

    @staticmethod
    def pr(num, updated):
        return {"number": num, "updated_at": updated, "state": "open",
                "head": {"sha": "b" * 40}, "merged_at": None,
                "title": "fix auth speed", "html_url": f"https://github.com/earendil-works/pi/pull/{num}"}


class MutationTests(unittest.TestCase):
    def setUp(self):
        self.state = {
            "head": "a" * 40, "latest_release": {"tag": "v1"},
            "open_pr_count": 2, "recent_pr_count": 2, "recent_issue_count": 1,
            "pr_digest": "p", "issue_digest": "i", "release_digest": "r",
            "recent_pr_ids": [], "recent_issue_ids": [],
        }
        self.report = {"candidates": [], "counts": {}}

    def test_create_then_replay_is_idempotent(self):
        saved = {}
        issue = {"number": 3, "title": "", "body": ""}
        def existing():
            return issue if saved else None
        def api(method, path, payload=None):
            self.assertEqual(method, "POST")
            self.assertEqual(path, "/repos/oigorbrito/finecore/issues")
            saved.update(payload)
            issue.update(payload)
            return issue
        with patch.object(tracker, "TARGET", "oigorbrito/finecore"), patch.object(
            tracker, "tracked_issue", side_effect=existing), patch.object(
            tracker, "api", side_effect=api) as fake:
            a = tracker.sync(self.state, self.report)
            b = tracker.sync(self.state, self.report)
        self.assertEqual(a["action"], "created")
        self.assertEqual(b["action"], "unchanged")
        fake.assert_called_once()

    def test_change_updates_once_and_preserves_human_suffix(self):
        prior = tracker.describe(self.state, self.report) + "\n\nMANTER_NOTAS_HUMANAS"
        issue = {"number": 9, "body": prior}
        revised = dict(self.state, head="b" * 40)
        def api(method, path, payload=None):
            self.assertEqual(method, "PATCH")
            self.assertTrue(path.endswith("/issues/9"))
            self.assertIn("MANTER_NOTAS_HUMANAS", payload["body"])
            return {"number": 9}
        with patch.object(tracker, "TARGET", "oigorbrito/finecore"), patch.object(
            tracker, "tracked_issue", return_value=issue), patch.object(
            tracker, "api", side_effect=api) as fake:
            result = tracker.sync(revised, self.report)
        self.assertEqual(result["action"], "updated")
        fake.assert_called_once()

    def test_never_allows_upstream_mutation(self):
        with patch.object(tracker, "TARGET", "oigorbrito/finecore"), patch.object(
            tracker, "TOKEN", "fake-only"):
            with self.assertRaisesRegex(RuntimeError, "Only FineCore"):
                tracker.api("PATCH", "/repos/earendil-works/pi/issues/1", {"title": "x"})

    def test_requires_exact_target(self):
        with patch.object(tracker, "TARGET", "other/repository"):
            with self.assertRaisesRegex(RuntimeError, "Wrong destination"):
                tracker.sync(self.state, self.report)

    def test_title_is_never_a_benchmark_claim(self):
        report = {"candidates": [{"pr": 10, "title": "<script>faster</script>",
                                   "url": "https://github.com/earendil-works/pi/pull/10",
                                   "head": "a" * 40, "topics_title_heuristic": ["performance"]}]}
        body = tracker.describe(self.state, report)
        self.assertNotIn("<script>", body)
        self.assertIn("NÃO benchmark", body)


if __name__ == "__main__":
    unittest.main()
