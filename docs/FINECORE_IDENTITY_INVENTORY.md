# FineCore canonical identity inventory

## Status

Inventory only; no upstream code or configuration has been changed. This document records identity-bearing surfaces found on branch `migration/orca-faithful-import` at import commit `e036dd7a7e2a040b4da1152d99aede0552cb1641`, whose source is `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`.

The broad text search for `orca`, `orcacode`, `orcapods`, and `okikorg` found matches in 452 text files (excluding `Cargo.lock`, HTML, and PNG from that count). It is a discovery aid, not proof that every identity-bearing value is found; final comparison must also inspect binary assets, generated files, URL/config behavior, and all tree diffs. Search results include historical benchmark/release material and tests, so match presence does not imply replacement.

## Decisions needed before transformation

| Decision | Proposed FineCore value | Status / impact |
|---|---|---|
| Canonical repository URL | `https://github.com/oigorbrito/finecore` | Candidate based on the provided FineCore repository link; confirm as the lasting canonical URL before updating metadata/docs. |
| User-facing host/product name | `FineCore` | Candidate; confirm casing and whether the CLI host is also branded FineCore. |
| CLI crate and executable | `finecore` | Candidate; changes command name, package identity, generated binary/test names, scripts, install/update flow, and user documentation. Breaking for existing CLI users unless compatibility alias is retained. |
| Rust library package prefix | `finecore-*` | Not approved. Renaming packages and Rust crate identifiers breaks downstream Cargo dependencies/imports; decide whether FineCore is a product rename, a fork namespace, or a separately published API. |
| Persistent state/config directory | Keep `.orca` and existing OS config paths initially | Preserve user data and compatibility. Any move needs an explicit migration/dual-read policy; do not rename by global replacement. |
| Environment variables | Keep `ORCA_*` names initially; optionally add FineCore aliases later | Public configuration contract. Define precedence, deprecation, and compatibility before changing. |
| Agent Plugin namespace | Keep `io.github.okikorg.orcacode` initially | Plugin ecosystem identifier; changing it can make installed plugins undiscoverable. Requires a compatibility/alias and migration policy. |
| MCP client identity / provider headers | Review individually; do not mass-replace | Values such as `orca-harness`, `orcacode`, and `ORCACODE_USER_AGENT` cross process/network boundaries. Confirm protocol/provider expectations before changing. |
| Release channel, tags, artifacts, installer URLs | Do not alter during identity-only code pass | `.github/workflows/release-orcacode.yml`, `ci/release.sh`, `crates/cli/src/update.rs`, and related assets connect to external services/secrets. Require an approved FineCore release destination and separate release plan. |
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
| `orcacode` / binary `orcacode` | `crates/cli/Cargo.toml` | `finecore` / binary `finecore` |

Affected surfaces include workspace and crate manifests, dependency keys, Rust `use` paths, Cargo.lock package records, examples, tests, `env!("CARGO_BIN_EXE_...")`, shell/CI/benchmark commands, docs and generated output. Keep source directory paths (for example `crates/harness-core`) unchanged unless a separate path-layout decision is approved.

### 2. Repository and public product presentation

Observed in root `Cargo.toml`, root `README.md`, crate READMEs, badges, install examples, product screenshots, diagram labels, issue/release links, and docs site links. Update only after confirming the FineCore canonical repository URL, product/CLI naming, and which docs/assets are current product presentation versus historical upstream evidence.

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

## Minimum transformation proposal

Pending decisions above, the safest initial transformation is **no broad source replacement**. After approval, a minimally scoped identity commit could update only confirmed repository metadata and current top-level product presentation. Package/executable/API names, runtime paths and variables, plugin/MCP/provider identifiers, and release infrastructure should be separate decision groups because they carry compatibility or operational consequences. Every changed path must be compared to the import commit and assigned a rationale; no behavior change is in scope.

## Validation checklist for any later identity commit

- Capture `git diff --name-status` and content diff against import commit `e036dd7`.
- Confirm every changed file maps to an approved decision above.
- Run targeted tests for changed APIs, config persistence, plugin/MCP integration, CLI usage, scripts, and benchmark plumbing.
- Run upstream-prescribed workspace tests, Clippy and formatting gates after baseline is established.
- Compare benchmark workload and stable metric IDs; do not claim performance results from changed or incomparable workloads.
- Verify `LICENSE`/`NOTICE` retained, no secrets or release side effects, and no changes to `main` or unrelated branches.
