# FOOTPRINT-01: release-binary footprint investigation

## Scope and branch provenance

This experiment is isolated on `experiment/footprint-01`, based directly on
`integration/finecore-orca-main-review`, not on PR #114. It carries only the
MCP stdio correction as a source transplant (commit `102dd520`), so baseline
full tests are reliable; it does not import PR #114's workflow, runner, or
report history. PR #114 and its head remain untouched. The canonical comparison
is `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`.

The objective is causal measurement, not satisfying the inherited size check
at any cost. That check remains strictly `<6,700,000` bytes. No size budget,
source-size rule, lint, default behavior, or security setting is changed to
turn a failure green.

The existing paired Linux evidence reports a stripped FineCore release binary
of 9,334,616 bytes and Orca at 9,342,840 bytes. FineCore would need to shed
2,634,616 bytes (28.22% of its baseline) to reach the inherited cap. An older
pre-MCP-fix Windows measurement was 7,861,248 bytes; it is not a measurement of
this experimental HEAD and still exceeded the cap.

## Experimental design

`.github/workflows/footprint-01.yml` checks out the experimental FineCore tree
and the pinned donor on Ubuntu 24.04, installs the repository's stable Rust
toolchain once, and runs sequentially on one host. The script records toolchain,
commit, lock graph, environment, commands, exit codes, elapsed compile times,
raw outputs, binary size/SHA-256, ELF sections and startup/workload samples.
Each build uses a clean, separate target directory. Target directories are
deleted after their binaries and test harnesses have been copied into the
artifact, avoiding Cargo build-target artifacts and disk-pressure bias.

Every baseline and candidate is built a second time from an empty target
directory. Exact binary size and SHA-256 equality are the reproducibility
check. The unmodified FineCore and canonical Orca baselines also run formatting,
source-size, strict Clippy, debug and release workspace tests, benchmark smoke,
release size checks, compatibility invocations, and 20 immediate-exit MCP
repetitions. Each FineCore variant receives release workspace tests, benchmark
smoke, valid `--version`/top-level `--help`/`plugin list` compatibility checks
and the same MCP repetitions. Legacy unsupported plugin-help probes remain in
raw records but do not gate functional eligibility. Every ordinary command
runs even after earlier failures; all exit codes remain in the artifact and the
workflow does not mask inherited failures. Full command output is flushed
unbuffered to per-command files; starts/ends and high-signal build/test progress
are streamed to Actions logs. After each completed baseline/variant, an
atomic JSON checkpoint and cumulative partial-results snapshot are written; a
separate `if: always()` artifact upload publishes completed variants even if
later analysis fails. `--render-existing` regenerates the report from preserved
JSON/logs without repeating measurements.

| Isolated variant | Hypothesis / status |
| --- | --- |
| Baseline | Fat LTO, `opt-level=z`, one codegen unit, `panic=abort`, stripped. Reference. |
| Thin LTO | Test whether faster/less-global LTO changes size or workload performance. |
| LTO disabled | Negative control for link-time dead-code/inlining effects. |
| `opt-level=s` | Compare with Cargo's more aggressive size setting `z`. |
| 16 codegen units | Test compile-time versus code-size/runtime tradeoff. |
| `panic=unwind` | Measure the cost of unwind-capable panic behavior; do not assume it is smaller or interchangeable. |
| `-C force-unwind-tables=no` | Diagnostic-only measurement of unwind-table footprint. Not eligible for promotion if stack traces, crash diagnosis, debugging, or profiling support is degraded. |
| `--no-default-features` | Negative control. Cargo feature graphs are captured; if dependency defaults change or functionality is removed, reject rather than call it a safe size optimization. |

No variant combination is assembled after inspecting results. A combination is
justified only if an isolated setting is reproducibly smaller, passes all
functional checks, and has no relevant performance regression. The reported
profile experiments are Linux measurements; no setting is approved for other
supported platforms without platform-specific build and test evidence.

## Attribution and measurements

For stripped release binaries, the workflow captures `size -A`, ELF section
tables, dynamic entries, dynamic symbols, exact file hashes, and binary-size
check status. A separate **diagnostic-only** release build retains symbols and
level-1 debug information for pinned `cargo-bloat` crate/function attribution;
that build's size is not treated as a candidate. `cargo-bloat` reports text
contribution and cannot apportion all read-only data, relocation, or unwind
sections, especially after LTO/inlining. The ELF section accounting is retained
alongside it; debug/symbol metadata is kept distinct from the shipped artifact.

Startup and maximum RSS are sampled in alternating paired order for `--version`
using the same host. Local `core_tools_perf` test executables validate actual
filesystem operations and accuracy; candidate-vs-baseline samples alternate
order, emit all five scenarios, and have paired bootstrap intervals. These are
microbenchmarks, not agent/model effectiveness. No model/provider calls are
made. Intervals and medians are exploratory with multiple comparisons; they are
not a performance claim by themselves.

The GitHub Actions artifact contains `FOOTPRINT_01_REPORT.md`,
`footprint-01-results.json`, environment and version metadata, raw logs, ELF and
symbol reports, copied executable/test binaries, and all paired raw samples.
A partial checkpoint artifact is uploaded independently. If report rendering
fails, it can be regenerated offline from the preserved JSON/logs using
`scripts/footprint_01.py --render-existing <json> --out <dir>` without rerunning
builds or tests. The report includes the required table for absolute bytes, savings, compile
time, repeat-build identity, tests, MCP, compatibility, benchmark results, and
decision disposition. The JSON is the machine-readable source of truth.

## Run 40351250854 result and recovery

Run [40351250854](https://github.com/oigorbrito/finecore/actions/runs/40351250854) used FineCore HEAD `066229b4eb35f061e837197ebd105710fb42c80b` and completed all eight FineCore profile builds plus the pinned Orca baseline, repeat-build checks, tests, MCP repetitions, startup/RSS samples, and paired workload measurements. The runner uploaded artifact [12678024189](https://github.com/oigorbrito/finecore/actions/runs/40351250854/artifacts/12678024189), then failed while rendering the report: the toolchain log reference already began with `logs/`, and the renderer prepended `logs/` a second time. This was not a build timeout, memory failure, or disk exhaustion. The report has been regenerated from the preserved JSON/log artifact only; no experiment was rerun.

The same run recorded 14 FineCore versus 15 Orca source-size violations and 13 strict-Clippy errors in each tree; every baseline/variant binary-size gate also failed at the unchanged cap. The four `cargo-bloat` commands failed because the offline audit lacked cached `bumpalo v3.20.3`. Unsupported plugin-help probes also returned nonzero and are preserved, but do not indicate product test failures; valid `--version` and top-level `--help` passed. The replacement runner uses supported probes and records every nonzero result transparently.

## Decision rule

A smaller binary is not automatically a safe optimization. Do not promote a
variant if it changes functional feature graphs, fails tests/compatibility,
loses diagnostic/security-relevant behavior, has a reproducible material
regression, or lacks supported-platform validation. Do not confuse LTO/dead
code elimination with removal of source features, and do not assume removing
unwind metadata or embedded syntax/theme data is free. If no variant is
acceptable, document the smallest measured *eligible* binary and the remaining
gap; leave the inherited size gate failed and present product-budget or
architectural alternatives for separate review.
