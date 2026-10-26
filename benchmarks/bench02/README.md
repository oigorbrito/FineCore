# BENCH-02 controlled comparison

This experimental workflow is separate from migration PR #111. It checks out
FineCore and `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`
on one Ubuntu runner. It does not modify `main` or the migration PR.

## Procedure

- Verify the pinned donor and hash the tracked `crates/tools`, core-tools, MCP,
  and subagent trees. The manifests and lockfiles are also hashed; both use the
  same `rust-toolchain.toml`, and Cargo runs with `--locked`. The dependency
  graph must match after normalizing only the FineCore/Orca CLI package identity.
- Run core-tools accuracy tests and prebuild both release test executables.
  Execute 20 FineCore/Orca sample pairs, alternating order. Each side runs the
  same five real-filesystem scenarios and creates fresh fixtures per sample.
  Keep each raw output, command, exit code, elapsed time, and runner load
  readings. Compute paired median deltas and seeded 95% bootstrap intervals;
  p99 from 20 samples is still effectively a maximum, not a stable tail bound.
- Run the full MCP unit/report commands and the same `--quick` subagent
  concurrency and simulation profiles used by the existing CI, independently
  on both trees. Capture every command's result and generated files.
- The reporter continues after a suite/budget failure. Consolidation checks all
  required statuses after collection; a failing core-tools budget remains FAIL
  even when later MCP and subagent evidence is available. Artifacts contain only
  this run's output, never the repository's historical result snapshots.

## Interpretation limits

A GitHub-hosted runner is shared; its CPU/disk background load cannot be
reserved or disabled. Alternation, same-worker pairing, recorded load averages,
and task-cluster resampling reduce order and sampling bias but do not prove
between-run reproducibility. Re-run the workflow in independent sessions and
compare the full raw artifacts before attributing small effects to code.

The task-level FineCore/Orca/Pi comparison is a separate live-provider phase in
`../harness-comparison/README.md`. Its triad and 95% task-cluster analysis are
implemented and dry-run tested, but it was not invoked here. It requires a
reviewed current price schedule and authorized spend; no model/API calls are
made by this workflow.
