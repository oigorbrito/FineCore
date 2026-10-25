# FineCore — independent Orca upstream tracker

This monitoring workflow is separate from the Rust engine migration and from the historical subtree import. It does **not** fetch/merge code, update downstream branches, approve PRs, or publish releases.

## Source and destination

- Read-only upstream: `okikorg/orca-harness`.
- Destination: GitHub Issues in `oigorbrito/finecore`.
- Source events: upstream issues (open and closed) and, via a **separate `/pulls?state=all` listing**, pull requests and their commit-list digest; upstream default-branch HEAD, tags, releases and deleted/restored tags/releases. The first production run created 33 issue/tag/release/HEAD records but zero PR records, so the PR listing is now mandatory and fails closed if unavailable.
- Schedule: at minute 17 every six hours (UTC). Manual `workflow_dispatch` is available on the default branch. A **push to main modifying the tracker implementation** also runs validation and tracking, so the reviewed repair can be verified immediately after manual merge. Ordinary application pushes do not trigger it.
- Idempotence: records are keyed by an HTML marker in the destination issue. A repeated snapshot is a no-op. Updates use a single issue PATCH and do not post per-update comments, avoiding duplicate notifications after a retry.
- An upstream HEAD change records the newly observed SHA; it does not claim to enumerate every intermediate commit between polls.
- A newly created issue may remain if its creation succeeds but the process fails before learning its number. The next run re-indexes markers before continuing.
- A failed paginated scan fails closed; it never treats an incomplete list as deletion.

## Permissions and deployment

The `validate` job uses `contents: read` only, including on pull requests. The `track` job exists only for scheduled or manually dispatched runs, after successful validation, and uses job-scoped `contents: read`, `issues: write` with the repository-scoped `GITHUB_TOKEN`. Both checkout steps disable credential persistence. No personal token, `contents: write`, or `pull-requests: write` is used. Source repository API calls are GET only.

**Operational activation requires manual review and merge into `main`**, as scheduled GitHub Actions workflows run from the default branch. A branch-only copy of this workflow is not active. GitHub Actions must also be enabled and repository settings must permit issue creation by Actions; validate the first `workflow_dispatch` execution and confirm the downstream markers before claiming activation.

## Verification

Run `python -m unittest discover -s scripts -p 'test_upstream_tracker.py' -v` locally. The tests use mocks: no source or destination API writes.

A successful first run reports `created > 0` on a new destination. A second run with unchanged upstream should report only `unchanged`; upstream changes should update the corresponding destination issue, not duplicate it. Review branch protections separately: this workflow does not configure GitHub rulesets.

## Security: upstream-controlled snapshot titles

PR and issue titles may contain characters that look like HTML comment endings (`--> `) or JSON closing braces. The original non-greedy regex misparsed a title containing `} -->` (reproduced as `JSONDecodeError`). The tracker now parses the JSON state with `JSONDecoder.raw_decode` and escapes embedded `-->` as JSON Unicode escapes when writing markers. Existing unescaped markers remain readable; corrupt marker data fails closed. Regression tests cover this and nested JSON states. No upstream issue is modified.

## Verification evidence and current blocker (2026-10-25)

- **Local offline validation:** Python 3.13.5, `16 passed / 0 failed` (2026-10-25). The contract suite includes a full in-memory three-poll test (initial create, unchanged replay, two updated records); tags/releases, PR digests, pagination failure, and retries without ambiguous POST duplication. No real GitHub issue was created by this test.
- **Remote Actions:** run [40153604231](https://github.com/oigorbrito/finecore/actions/runs/40153604231) reported `failure` for the `validate` job **with zero executed steps and no assigned runner**; `track` was `skipped`. A prior run [40153078961](https://github.com/oigorbrito/finecore/actions/runs/40153078961) likewise reported zero executed steps, runner ID 0, and zero billable runner milliseconds. Treat remote CI as **BLOCKED_BEFORE_RUNNER**, not as a Python test failure. The precise account/runner reason is not confirmed through available API evidence.
- **Release gate:** do not represent the GitHub Actions tracker as active until the workflow has been integrated into `main`, the runner successfully executes, an authorized manual dispatch creates the initial downstream snapshots, and a repeat dispatch produces only `unchanged`. Merge remains manual.
- **Rate-limiting:** content-changing requests are paced at a minimum of one second apart (up to 60/min), below GitHub's documented general secondary content limit of 80/min. Rate-limited API errors fail closed; ambiguous issue creation POST requests are not retried without first re-indexing the next run. Reference: https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api
- **Fallback continuity:** an independent ChatGPT monitoring task is enabled, but it is not equivalent to this GitHub-Issues mirror and does not change the repository.
