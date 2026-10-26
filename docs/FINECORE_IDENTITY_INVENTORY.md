# FineCore canonical identity inventory

## Status

This inventory records identity-bearing surfaces found on branch `migration/orca-faithful-import` at import commit `e036dd7a7e2a040b4da1152d99aede0552cb1641`, whose source is `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`. A first identity pass now updates FineCore repository metadata, top-level product/CLI presentation, and the CLI crate/binary name. Runtime and external ecosystem identifiers remain unchanged.

The broad text search for `orca`, `orcacode`, `orcapods`, and `okikorg` found matches in 452 text files (excluding `Cargo.lock`, HTML, and PNG from that count). It is a discovery aid, not proof that every identity-bearing value is found. The current tree/blob comparison is recorded in [`identity-divergence.csv`](identity-divergence.csv); it classifies source files, additions, and intentional edits, but human review remains required. Search results include historical benchmark/release material and compatibility-sensitive identifiers, so match presence does not imply replacement.

## Decisions needed before transformation

| Decision | Proposed FineCore value | Status / impact |
|---|---|---|
| Canonical repository URL | `https://github.com/oigorbrito/finecore` | Approved by user-provided repository URL; applied to workspace metadata. |
| User-facing host/product name | `FineCore` | Approved; root README now identifies this as the FineCore-branded derivative and names the pinned source. |
| CLI crate and executable | `finecore` | Approved as the FineCore CLI identity; crate/binary rename is applied. Upstream install/release URLs remain explicitly identified as upstream and are not activated for FineCore. |
| Rust library package prefix | Keep `orca-harness-*` / `orca_harness_*` | Preserve upstream public crate/API identifiers for compatibility and provenance; no mass rename. |
| Persistent state/config directory | Keep `.orca` and existing OS config paths initially | Preserve user data and compatibility. Any move needs an explicit migration/dual-read policy; do not rename by global replacement. |
| Environment variables | Keep `ORCA_*` names initially; optionally add FineCore aliases later | Public configuration contract. Define precedence, deprecation, and compatibility before changing. |
| Agent Plugin namespace | Keep `io.github.okikorg.orcacode` initially | Plugin ecosystem identifier; changing it can make installed plugins undiscoverable. Requires a compatibility/alias and migration policy. |
| MCP client identity / provider headers | Review individually; do not mass-replace | Values such as `orca-harness`, `orcacode`, and `ORCACODE_USER_AGENT` cross process/network boundaries. Confirm protocol/provider expectations before changing. |
| Release channel, tags, artifacts, installer URLs | No FineCore release channel is configured | Orcacode release files/scripts remain as upstream provenance, but the workflow is guarded to execute only in `okikorg/orca-harness`, and `ci/release.sh` fails closed in FineCore. The CLI updater/background check are disabled, so FineCore cannot install, query, or publish Orcacode releases through these paths. A FineCore release destination needs a separate plan. |
| npm package | `finecore` registry name | The registry currently lists a separate `finecore@0.0.1` bootstrap CLI for local Rust toolchain diagnostics/workspace inspection (verified 2026-10-26). `npm/finecore/` is a different, internal placeholder marked `private: true`; it is not the registry package and is not the FineCore Rust CLI. Never publish the placeholder or imply the bootstrap package is the Rust agent. |
| Historical releases and benchmark records | Preserve as historical source evidence | `docs/releases/`, benchmark results, reports, and their old commands/names should not be rewritten as if they described FineCore. New FineCore results should be separately identified. |
| Source/history notices | Preserve | `LICENSE` and `NOTICE` are not branding cleanup targets; retain required notices and provenance. |

## Inventory by surface

### 1. Rust workspace and public package/API identity

Root `Cargo.toml` defines a ten-member workspace. Current package names and likely FineCore candidates are shown below. Candidates are illustrative only; changing them requires explicit API/release approval.

| Current package / Rust library | Manifest | Candidate only |
|---|---|---|
| `orca-harness-core` / `orca_harness_core` | `crates/harness-core/Cargo.toml` | `finecore-core` / `finecore_core` |
| `orca-harness-dag` / `orca_harness_dag` | `crates/harness-dag/Cargo.toml` | `finecore-dag` / `finecore_dag` |
| `orca-harness-provider-auth` / `orca_harness_provider_auth` | `crates/provider-auth/Cargo.toml` | `finecore-provider-auth` / `finecore_provider_auth` |
| `orca-harness-model-providers` / `orca_harness_model_providers` | `crates/model-providers/Cargo.toml` | `finecore-model-providers` / `finecore_model_providers` |
| `orca-harness-sandbox-providers` / `orca_harness_sandbox_providers` | `crates/sandbox-providers/Cargo.toml` | `finecore-sandbox-providers` / `finecore_sandbox_providers` |
| `orca-harness-tools` / `orca_harness_tools` | `crates/tools/Cargo.toml` | `finecore-tools` / `finecore_tools` |
| `orca-harness-tool-extensions` / `orca_harness_tool_extensions` | `crates/tool-extensions/Cargo.toml` | `finecore-tool-extensions` / `finecore_tool_extensions` |
| `orca-harness-extensions` / `orca_harness_extensions` | `crates/extensions/Cargo.toml` | `finecore-extensions` / `finecore_extensions` |
| `orca-harness-sdk` / `orca_harness_sdk` | `crates/sdk/Cargo.toml` | `finecore-sdk` / `finecore_sdk` |
| `orcacode` / binary `orcacode` | `crates/cli/Cargo.toml` | `finecore` / binary `finecore` (applied; invocation docs updated where they describe current FineCore usage) |

Affected surfaces include workspace and crate manifests, dependency keys, Rust `use` paths, Cargo.lock package records, examples, tests, `env!("CARGO_BIN_EXE_...")`, shell/CI/benchmark commands, docs and generated output. Keep source directory paths (for example `crates/harness-core`) unchanged unless a separate path-layout decision is approved.

### 2. Repository and public product presentation

Observed in root `Cargo.toml`, root `README.md`, crate READMEs, badges, install examples, product screenshots, diagram labels, issue/release links, and docs site links. First pass updates root workspace repository/homepage, current root CLI examples, the CLI/SDK READMEs, and the Makefile development build target. Upstream install/release URLs and historical docs/assets remain explicitly upstream or unchanged; audit is incomplete.

### 3. Runtime configuration and persisted data (compatibility-sensitive)

Observed examples include `ORCA_CONFIG_DIR`, `ORCA_MODEL`, `ORCA_BASE_URL`, `ORCA_THEME`, `ORCA_SUBAGENT_DEPTH`, `ORCA_MODEL_CONCURRENCY`, benchmark/test variables, `~/.config/orcacode`, `%APPDATA%\\orcacode`, and workspace `.orca/` state. Main implementation surfaces include `crates/cli/src/config/storage.rs`, `crates/cli/src/run_args.rs`, `crates/cli/src/instructions.rs`, `crates/sdk/src/harness.rs`, `crates/sdk/src/skills.rs`, and tool/skill discovery modules. Tests encode those paths and variables. Preserve them during the first identity pass or explicitly design backward-compatible aliases and data migration.

### 4. Protocol and ecosystem identifiers

Observed values include `io.github.okikorg.orcacode` in Agent Plugin discovery/scaffolding/tests, MCP `clientInfo.name` (`orca-harness`), web user-agent (`orca-harness-web/...`), provider headers/user-agent (`orcacode`), and memory markup tags (`<orcacode_memory>`). These are not merely visual strings: external plugins, providers, or consumers may depend on them. Review each contract independently and retain old identifiers or add compatibility behavior where required.

### 5. Release, packaging, and external automation

Observed coupling includes release tags `orcacode-v*`, binary/archive names, package selection in `.github/workflows/release-orcacode.yml`, release-server paths and secrets, installer/update URLs (`crates/cli/src/update.rs`), `ci/release.sh`, `Makefile`, size checks, and CI/release docs. Do not activate, rename, or run release publication as part of this inventory. Establish FineCore ownership, destination, credentials, package identifiers, and release approval separately.

### 6. Benchmarks, fixtures, and tests

Observed benchmark and test names encode `orca`/`orcacode` in Criterion IDs, temporary directories, task manifests, command invocations, budget keys, reports, HTML/CSS labels, and fixtures. Preserve benchmark workload semantics, raw historical results, and scoring contracts. If the CLI executable is renamed, update only invocation plumbing and assertions required to run the same workload; keep stable metric/task IDs unless there is an explicit schema/version decision. Re-run benchmark unit tests and compare equivalent workloads after changes.

### 7. Historical and provenance material

`docs/releases/`, benchmark result artifacts, old upstream URLs, screenshots, `LICENSE`, and `NOTICE` have historical or provenance meaning. Do not rewrite history or remove notices in the name of identity cleanup. Decide separately whether new FineCore documentation should clearly distinguish historical Orca release/benchmark material from FineCore releases/results.

## First-pass transformation status

Applied: confirmed FineCore repository metadata; FineCore product presentation; CLI package/executable rename; current build/test command examples; CI and benchmark build targets; startup/compare/SWE-bench binary resolution; and FineCore startup benchmark labels/budgets. The inherited Orcacode release workflow is guarded so it can only execute in its canonical upstream repository; the inherited `ci/release.sh` now fails closed and the Makefile labels its release targets disabled. Disabled the inherited updater and background check because they target Orcacode's server/installer; added a fail-closed test. Preserved: `orca-harness-*` library crates/APIs, runtime config/state paths, `ORCA_*` variables, plugin/MCP/provider protocol identifiers, upstream release scripts and artifacts, Criterion IDs, and historical results. Active benchmark metric labels changed only where they named the CLI; historical measurements are explicitly marked as Orca data. The source-to-import inventory is in `docs/upstream-import-manifest.csv`; current divergence is in `docs/identity-divergence.csv`. Remaining: review all remaining historical/external docs and run the supported Linux CI gates.

## Validation checklist for the identity pass

- Capture `git diff --name-status` and content diff against import commit `e036dd7`; see `docs/identity-divergence.csv`.
- Confirm every changed file maps to an approved decision above; the report currently records no missing upstream files or unexplained content changes.
- Run targeted tests for changed CLI identity, scripts, CI workflows, and benchmark plumbing; classify every failure against the pristine source baseline.
- Run upstream-prescribed workspace tests, Clippy and formatting gates; Windows-specific gaps and timeouts must remain explicit.
- Compare benchmark workload and stable metric IDs; do not claim performance results from changed or incomparable workloads.
- Verify `LICENSE`/`NOTICE` retained, no secrets or release side effects, and no changes to `main` or unrelated branches.
