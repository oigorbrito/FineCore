# INHERITED-GATES-01: donor-equivalent gate investigation

## Scope and provenance

Reference snapshot at investigation start: `main` was
`95e666924e672dba4749ab02b26d86782b0bf78a`; integration was
`2f980015a733ec449ca1ea8be51e97da08220fb1`. PRs #110–#113 were all OPEN and
Draft: #110 `092ff1ff0fb654c814663a296318abc06266584f` (base integration), #111
`2f980015a733ec449ca1ea8be51e97da08220fb1` (base main), #112
`d2487e6aa046d77c15aed988ee335fa547a3c34d` (base integration), and #113
`45aaef0cf63ddf7e20816687331e9e11a858c172` (base integration). This review
branch was created directly from the integration review branch at
`2f980015a733ec449ca1ea8be51e97da08220fb1` (`integration/finecore-orca-main-review`).
The comparison source is the canonical Orca commit
`2a9fd399cfbbaad773e3cbed92bb6e24d7561f01` (`okikorg/orca-harness`). The
comparison does not change `main` or PRs #110–#113. It makes no production Rust
changes and does not merge, publish a release, or waive a gate.

The comparison runner is `scripts/compare_inherited_gates.py`. The pinned
`inherited-gates-01` GitHub Actions workflow checks out both revisions and runs
them sequentially on the same Ubuntu 24.04 runner, stable Rust toolchain,
workspace dependency declarations, external lock graph, and release profile.
It records compiler versions, revision IDs, lock/profile fingerprints, gate
logs, binary hashes/sizes/sections, and alternating paired core-tools samples.
Every ordinary gate runs even after an earlier gate fails. An expected gate
failure leaves the workflow failed and is retained in the artifact.

## Local reproduction (Windows x86_64 MSVC)

A fresh paired run used Rust/Cargo 1.99.1, Clippy 0.1.99, LLVM 23.1.1, each
repository's pinned `stable` toolchain, the same host, default `RUSTFLAGS`, and
separate target directories. The release profiles are identical:
`opt-level = "z"`, fat LTO, one codegen unit, `panic = "abort"`, and
`strip = true`. Both workspaces declare the same members and workspace
dependencies. Their 281 external locked package records (including dependency
edges and checksums) are identical. Lockfile hashes differ because local
workspace package metadata differs.

| Gate | FineCore | Orca | Result |
| --- | ---: | ---: | --- |
| Source size | 15 violations | 15 violations | FAIL in both |
| Clippy, `-D warnings` | 13 errors | 13 errors | FAIL in both |
| Release build | passed | passed | both compiled |
| Binary size | 7,861,248 bytes | 7,868,416 bytes | FAIL in both; limit is `<6,700,000` |

The FineCore binary is 7,168 bytes smaller in this single paired build. This is
not a performance result and is not statistically meaningful. Both exceed the
limit by over 1.16 MB. The exact compiler output, gate status, size, and binary
SHA-256 were captured locally; the Ubuntu workflow uploads the durable complete
logs and section breakdown.

## Findings and cause classification

### 1. Source-size — inherited policy/content mismatch

The rule in `ci/check-source-size.sh` is byte-for-byte the same in both trees:
every `.rs`, `.inc`, or `.part` under `crates/` must be at most 599 lines. Both
runs report the same 15 paths and line counts:

| Lines | File |
| ---: | --- |
| 662 | `crates/cli/src/main.rs` |
| 644 | `crates/cli/src/main_tests.rs` |
| 750 | `crates/cli/src/runtime/worker.rs` |
| 600 | `crates/cli/src/tui/commands/dispatch.rs` |
| 686 | `crates/cli/src/tui/events/harness.rs` |
| 618 | `crates/cli/src/tui/state.rs` |
| 777 | `crates/cli/src/tui/tests/background_subagents.rs` |
| 600 | `crates/extensions/src/memory.rs` |
| 622 | `crates/sdk/examples/host_assembly.rs` |
| 649 | `crates/tool-extensions/src/mcp/client.rs` |
| 643 | `crates/tools/examples/bench_workflow_graph.rs` |
| 722 | `crates/tools/src/subagent/background/manager.rs` |
| 909 | `crates/tools/src/subagent/host.rs` |
| 944 | `crates/tools/tests/subagent/host.rs` |
| 601 | `crates/tools/tests/tools/glob_process_admin.rs` |

Thirteen over-limit files are byte-identical to the canonical donor. The other
two (`main.rs` and `dispatch.rs`) have only FineCore CLI naming/help/error
message substitutions; neither changes line count or control flow. Thus none of
the violations was introduced by the FineCore transplant. Splitting 15 files
solely to satisfy a universal line count would be a wide module/test refactor,
not a minimal migration fix. Mechanical chunking or `include!` fragments would
only disguise the same maintainability problem. No such changes were made.

### 2. Clippy — inherited public error-type layout

Both strict Clippy runs fail with the same 13 `clippy::result_large_err`
diagnostics. Twelve are results returning public `HarnessError`; the remaining
diagnostic is a `ModelError` result in `crates/harness-core/src/testing.rs`.
The largest payload is the public
`ModelError::MalformedToolArguments` variant, and `HarnessError::Model`
contains `ModelError` directly. Clippy reports a variant of at least 160 bytes.
The relevant error definitions and diagnostic sites are donor-identical.

Boxing the variant payload or changing result error types would change public
enum construction/pattern matching or public signatures. A broad lint
suppression would conceal the diagnostic, not fix it. Because preserving the
public API is a constraint, this investigation does not make that change or
silence the lint. The log/artifact is the authoritative list of all 13 sites.

### 3. Binary size — inherited limit not calibrated to either product build

The two release profiles, workspace dependency declarations, and external
locked dependency graph match. Both builds use size-oriented optimization,
fat LTO, a single codegen unit, abort-on-panic, and symbol stripping, yet both
Windows binaries exceed the shared non-musl `<6,700,000` byte check by more than
1.16 MB. The CI Linux build is also measured by the same check in the paired
workflow.

The check itself documents the 6.7/9 MB values as inherited Orca release
constraints and says FineCore has no established release baseline. The data
therefore demonstrates a stale/unvalidated threshold, not a FineCore-only
regression. Raising it without a product-supported size budget, changing
features/dependencies, or switching release settings without runtime
measurements would be arbitrary. No size exception or optimization was added.
The ELF section report and hashes in the workflow artifact show where the
measured bytes reside.

## Tests and performance

The paired workflow runs, in order for each repository: formatting, source
size, strict Clippy, full workspace tests, benchmark smoke tests, release build,
and binary-size gate. It then warms both `core_tools_perf` test binaries and
runs 20 alternating FineCore/Orca samples of five local-filesystem scenarios.
The benchmark makes no model/provider calls. Paired median percentage changes
and deterministic bootstrap intervals are evidence only; one shared runner is
not enough to claim a speedup or regressions. The current branch makes no
runtime code change, so there is no code-level before/after optimization to
attribute to this PR.

**Ubuntu paired-run evidence:** pending the workflow on the published Draft PR.
The overall workflow is expected to remain FAIL if the inherited source-size,
Clippy, or binary-size checks reproduce; all logs are uploaded regardless.

## Decision

| Gate | Decision |
| --- | --- |
| Source size | Preserve FAIL. Inherited in the canonical donor; no safe minimal split justified in this migration review. |
| Clippy | Preserve FAIL. Diagnostics are inherited; the clean fix would alter public error API. No lint waiver. |
| Binary size | Preserve FAIL. Both canonical and FineCore builds exceed the inherited limit; no justified limit change or measured safe optimization yet. |

No inherited gate is represented as passing, waived, or fixed. A later change
may address module boundaries, error representation, or a product-specific
release budget as separately reviewed work with API and runtime evidence.
