# FineCore Orca migration roadmap — proposal under review

## Purpose and constraints

Propose a complete, traceable migration based on the canonical Orca Harness upstream, applying only explicitly approved FineCore identity changes. This roadmap does not replace a rights review. Preserve required license and attribution notices. Do not represent upstream-derived material as independently authored.

**Authority note:** this roadmap is a proposal submitted for review, not the operational source of truth. Until formally accepted, the applicable root `AGENTS.md`, specialized migration skill, manifests, and validation record at the checked-out revision govern. Reconcile this proposal against those exact revisions and record paths, SHAs, authority, and unresolved conflicts before relying on it. The documented staging lineage is `migration/orca-faithful-import`; `bootstrap/finecore-independent` is the greenfield base. Any promotion to bootstrap is a separate decision. Public product identity is FineCore; preserve upstream code and compatibility contracts as closely as possible, retaining inherited identifiers where changing them could break compatibility.

### Pinned source and destination

- Upstream: `https://github.com/okikorg/orca-harness`
- Branch: `main`
- Historical source reference commit: `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01` (reconcile against current staging provenance before follow-up work)
- Historical source tree reference: `9c35d0dd0d7e0f226063f16dd80161e231628e2b` (reconcile against current staging provenance before follow-up work)
- Isolated comparison clone: `../orca-harness-reference`, local branch `reference/orca-harness-2a9fd39`, checked out at the pinned commit (do not merge its history into FineCore).
- Historical tree inventory: 1,284 recursive entries including directories; 1,101 blob/file paths. Do not treat these as current counts without checking manifests.
- Source date reported: 2026-10-22
- FineCore staging branch: `migration/orca-faithful-import` (verify current head and ancestry before execution)
- Bootstrap base: `bootstrap/finecore-independent` (greenfield; promotion target only by separate explicit decision)
- `main` and all other branches are out of scope and must remain unchanged.

The source README describes a Rust workspace and directs contributors to run `cargo test --workspace`, `cargo clippy --workspace --all-targets`, and `cargo fmt --all`. It identifies Apache-2.0 and points to `LICENSE` and `NOTICE`. The upstream contribution guide requests focused changes and human review of every `crates/harness-core` change. Follow the exact instructions present at the pinned revision; do not assume current `main` documentation matches it.

## Change policy

- Import the complete pinned upstream tree, including workspace members, tests, benchmarks, scripts, workflows, and documentation, unless an item cannot or should not be included for a concrete and documented reason. No discretionary feature selection.
- Make identity changes only from reviewed manifest rows. FineCore is the canonical public identity; preserve upstream code and compatibility contracts, including inherited technical identifiers, unless a specific change has evidence, compatibility coverage, and explicit review. Do not alter execution behavior, APIs, dependencies, build behavior, or performance characteristics as part of identity work.
- Keep upstream-derived content identifiable and retain required notices. Record source commit/tree and every intentional difference.
- Do not rewrite the published independent FineCore history or force-push. Preserve the existing branch. No push, publication, branch promotion, or destructive Git operation without explicit approval.
- Split remaining work into focused, reviewable PRs by phase or independently verifiable deliverable; do not put the whole migration into one monolithic PR. Keep commits coherent and defensible for each PR scope, without a fixed count or purposeless fragmentation. State dependencies and target the proper predecessor/staging base.

## Phases and gates

### 0. Preflight and preservation

1. Record `git status --short --branch`, remotes, refs, and current commit. Confirm the target branch and that there is no unrelated work to overwrite.
2. Verify the relevant FineCore commit and branch exist. The reference clone, source commit, tree hash, and inventory counts above are historical records; verify their availability and applicability to the current work before reuse or import, recording commands and outputs.
3. Inspect source instructions, `LICENSE`, `NOTICE`, dependency and workflow policies at the pinned commit. If rights, required notices, or scope are unclear, block only the affected import, redistribution, or transformation. Record the question and continue independent inventory, validation review, or documentation work.
4. Treat staging's `docs/upstream-import-manifest.csv`, `docs/identity-divergence.csv`, and `docs/FINECORE_IDENTITY_INVENTORY.md` as the source of truth. Reconcile proposed identity changes against them. Existing classified rows are evidence, not blanket approval for new transformations. Add or revise records only when a demonstrated gap or new change requires it; do not create parallel manifests. New identity changes require a scoped row, compatibility rationale/tests, and human review. Record missing decisions as pending.

**Gate:** no migration mutation until source revision, staging destination, provenance/notices, and applicable identity decisions are confirmed. Unrelated audit and documentation work may continue while a decision is pending.

### 1. Pristine upstream baseline

1. Run the upstream-prescribed checks in an isolated checkout at the exact pin, starting with `cargo test --workspace`, then `cargo clippy --workspace --all-targets` and `cargo fmt --all` (use `--check` where validation-only is intended).
2. Record Rust toolchain, OS/architecture, exact commands, exit codes, durations, warnings, unavailable checks, and pre-existing failures. Do not fix upstream defects in this phase.
3. Inventory benchmark scripts, fixtures, budgets, and published result methods as a separate, optional measurement track. Run only safe, reproducible benchmarks; identify those needing APIs, credentials, external services, or special hardware. Never embed secrets in commands, logs, manifests, or results.
4. If performance comparison is in scope, save baseline results separately from migrated results. Upstream README benchmark numbers are contextual claims, not measurements on this environment. Benchmark work does not gate migration/identity completion unless a performance claim is being made.

**Gate:** for each required test/check, either applicable evidence is recorded or the check is explicitly `BLOCKED`/`NOT RUN` with evidence and a resolution condition. Before reusing staging logs, verify source/destination SHAs, toolchain, platform, and scope match the gate being assessed; repeat only uncovered or stale checks. A claim that a check passed requires successful, applicable evidence. Missing evidence blocks only claims and actions that depend on it, not independent source inventory, audit, or documentation work. Benchmarks remain a separate optional track unless performance claims are in scope.

### 2. Complete tree import

1. The canonical import is already recorded on `migration/orca-faithful-import`; do not repeat it into bootstrap. For any approved follow-up import, work on an isolated descendant of staging, without merging unrelated Git history. Preserve ancestry and do not alter other branches.
2. Compare tracked paths and file contents with the pinned upstream tree before identity edits. Verify the existing staging manifest covers source path, destination path, source blob, destination blob, and status (identical/added/missing/modified); update it only for a demonstrated gap or new change, preserving its history and evidence.
3. Resolve omissions and unexpected differences. Generated files, vendored dependencies, assets, hidden files, and workflow files are included or explicitly dispositioned; “not needed” alone is not sufficient justification.

**Gate:** all upstream paths are accounted for and the applicable import diff is reviewed before declaring import completeness or making import-dependent changes. Existing staging manifests may satisfy this gate only when their source pin, destination commit, and blob evidence match; other independent audit and documentation work may continue.

### 3. Minimal identity layer

1. Apply only the approved identity manifest. Prefer narrow, deterministic transformations and retain upstream notices.
2. Avoid blind global replacements: audit Rust crate names, workspace references, CLI names, environment variables, config paths, URLs, docs, fixtures, test snapshots, CI, and release scripts individually.
3. Produce a path-level and content-level divergence report against the pinned upstream tree. Every difference has a category (identity-only, generated consequence, required repository integration, or approved exception), rationale, and reviewer.
4. Verify there are no accidental behavior, API, dependency, feature, test, or benchmark changes. Record an unexplained difference, its evidence, owner or resolution condition, and block dependent actions and conclusions while continuing independent work.

**Gate:** all diffs are explained and the identity-only scope is confirmed by review.

### 4. Functional and operational validation

1. Run the full upstream-prescribed checks on the migrated tree: `cargo test --workspace`, `cargo clippy --workspace --all-targets`, and `cargo fmt --all -- --check`; record toolchain and results.
2. Run relevant CI workflows or equivalent local checks. Document platform-specific checks that cannot be run. Check package/build artifacts, CLI identity, docs and workflows for stale upstream product naming without changing upstream behavior.
3. Compare test outcomes with the pristine baseline. Classify every failure as baseline, migration-introduced, environment-specific, or unresolved; do not suppress failures to obtain a pass.
4. If a performance comparison or claim is in scope, run the same safe benchmark commands and workloads as baseline, with controlled conditions and repeated runs. Report distributions and resource use; distinguish measurement noise from regressions. Do not optimize in this phase.

**Gate:** no unexplained identity-related regression. Classify each red check as inherited, migration-introduced, environment-specific, flaky/order-dependent, or unresolved, using comparable source/FineCore evidence. Historical inherited red gates remain red until independently fixed and revalidated on the applicable current revision. Human acceptance may record a risk or follow-up decision, but does not convert a red gate to pass or satisfy a required passing gate. A flaky or environment-specific result is not automatically inherited. Platform-specific gaps remain explicit. Benchmark gaps do not block unrelated gates, and no benchmark/performance conclusion may be claimed while they remain open.

### 5. Audit, handoff, and later optimization

1. Review path inventory, divergence report, notices, dependency/lockfile changes, secrets, generated outputs, and Git status. Confirm `main` and other refs are unchanged.
2. Report migration status using distinct outcomes:
   - `MIGRATION_COMPLETE`: provenance is reconciled; all import paths and identity divergences are accounted for and reviewed; every required functional check has applicable passing evidence. Any non-required platform/external gaps have explicit status and resolution conditions.
   - `MIGRATION_COMPLETE_WITH_INHERITED_RED_GATES`: use only if the applicable accepted staging policy explicitly defines this outcome and its conditions are met. This roadmap does not waive, downgrade, or accept a red gate. Under the current identity policy, inherited red gates remain open until fixed and revalidated.
   - `MIGRATION_PENDING`: required evidence is missing, divergences remain unexplained, a migration-introduced or unresolved regression remains, or any required gate remains red or unverified.

   Neither completion status implies promotion readiness. Promotion requires the separate staging policy's gates and explicit approval. Benchmarks are required only for benchmark/performance claims, not to close unrelated migration gates. Prepare a report with revisions, manifests, validation, exceptions, benchmark status (`BLOCKED`/`NOT RUN` where applicable), and known gaps.
3. Keep performance optimization as a later, separately approved phase. For each proposed optimization, add a hypothesis, workload, correctness tests, benchmark plan, threshold, and rollback. Re-run affected tests and benchmarks; reject changes without reproducible benefit or with unacceptable regressions.
4. Scoped commits and Draft PRs are the review/delivery mechanism for approved work, split by phase or independently verifiable deliverable. Merging a PR, promoting staging to bootstrap, releasing/publishing artifacts, rewriting history, or changing protected refs are separate actions requiring their own explicit approval. Remote `main` is out of scope unless a separately approved plan explicitly says otherwise.

## Evidence artifacts

Keep the following reviewable in the repository or migration record, without secrets:

- pinned source and destination metadata;
- source/destination path-and-blob manifest;
- approved identity-change manifest;
- baseline and post-migration test/check logs;
- benchmark command/configuration/results and environment metadata when a performance comparison or claim is in scope; otherwise record the benchmark track as not in scope;
- divergence report and exception approvals;
- final branch/ref preservation check.
