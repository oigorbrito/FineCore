# FineCore — independent Orca upstream tracker

This monitoring workflow is separate from the Rust engine migration and from the historical subtree import. It does **not** fetch/merge code, update downstream branches, approve PRs, or publish releases.

## Source and destination

- Read-only upstream: `okikorg/orca-harness`.
- Destination: GitHub Issues in `oigorbrito/finecore`.
- Source events: upstream issues (open and closed), pull requests and their commit-list digest, upstream default-branch HEAD, tags, releases and deleted/restored tags/releases.
- Schedule: at minute 17 every six hours (UTC); manual `workflow_dispatch` is available **once this workflow exists on the repository default branch**.
- Idempotence: records are keyed by an HTML marker in the destination issue. A repeated snapshot is a no-op. Updates use a single issue PATCH and do not post per-update comments, avoiding duplicate notifications after a retry.
- An upstream HEAD change records the newly observed SHA; it does not claim to enumerate every intermediate commit between polls.
- A newly created issue may remain if its creation succeeds but the process fails before learning its number. The next run re-indexes markers before continuing.
- A failed paginated scan fails closed; it never treats an incomplete list as deletion.

## Permissions and deployment

The workflow uses `contents: read`, `issues: write`, and the repository-scoped `GITHUB_TOKEN`; it does not use a personal token. It does not grant or use `contents: write` or `pull-requests: write`. Source repository API calls are GET only.

**Operational activation requires manual review and merge into `main`**, as scheduled GitHub Actions workflows run from the default branch. A branch-only copy of this workflow is not active. GitHub Actions must also be enabled and repository settings must permit issue creation by Actions; validate the first `workflow_dispatch` execution and confirm the downstream markers before claiming activation.

## Verification

Run `python -m unittest discover -s scripts -p 'test_upstream_tracker.py' -v` locally. The tests use mocks: no source or destination API writes.

A successful first run reports `created > 0` on a new destination. A second run with unchanged upstream should report only `unchanged`; upstream changes should update the corresponding destination issue, not duplicate it. Review branch protections separately: this workflow does not configure GitHub rulesets.
