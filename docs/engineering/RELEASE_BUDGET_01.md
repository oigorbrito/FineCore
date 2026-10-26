# FINECORE — RELEASE-BUDGET-01

**Status:** investigation complete; no policy change proposed or authorized. Snapshot date: 2026-10-26.

## Decision

**Do not establish a new numeric FineCore distribution budget yet.** The repository does not define a FineCore release target matrix, download/install/storage ceiling, offline-support contract, signing policy, or supported-OS minimums. The current 6.7 MB / 9 MB values are explicitly inherited Orca release constraints, not requirements derived from FineCore users. FineCore's own size-gate comment says it has not established a release baseline.

Keep every inherited gate unchanged and report its actual failures. Do not interpret 9,334,616 B as an approved budget, and do not replace the inherited limit with a different number on the basis of this report. The defensible decision is **no FineCore budget until product distribution requirements and target support are approved**.

## Reconciled repository and PR heads

Live GitHub refs queried on 2026-10-26:

| Ref | Current HEAD | State / relationship |
|---|---|---|
| `main` | `95e666924e672dba4749ab02b26d86782b0bf78a` | Current default branch; has no Rust `Cargo.toml` or Rust release workflow. |
| `integration/finecore-orca-main-review` | `2f980015a733ec449ca1ea8be51e97da08220fb1` | Draft PR #111 head, based on `main`; contains the candidate Rust CLI and inherited CI/release files. |
| PR #110 | `092ff1ff0fb654c814663a296318abc06266584f` | Open Draft, based on integration. Its recorded base SHA `cb143d3b…` predates the current integration HEAD; it is not the current integration snapshot. |
| PR #111 | `2f980015a733ec449ca1ea8be51e97da08220fb1` | Open Draft, based on current `main`. |
| PR #112 | `d2487e6aa046d77c15aed988ee335fa547a3c34d` | Open Draft, based on integration. |
| PR #113 | `45aaef0cf63ddf7e20816687331e9e11a858c172` | Open Draft, based on integration. |
| PR #114 | `388bb4a56c6bebf0831ac53b1ed8a3b124e11335` | Open Draft, based on integration. |
| PR #115 | `066229b4eb35f061e837197ebd105710fb42c80b` | Open Draft, based on integration; contains FOOTPRINT-01 source/evidence. |
| PR #116 | `952c64af92757141912192155413cae3b9f5346c` | Open Draft, stacked on PR #115. |
| PR #117 | `77b29206ce2391eb8856b0645cb5a6fd37eada73` | Open Draft, stacked on PR #116. |

All PRs #110–#117 remain Draft. No existing branch, PR, or `main` was changed for this investigation. The proposed report PR is based on the current integration branch and adds only report/evidence tooling.

This distinction matters: **the Rust CLI whose binary is being measured is still a Draft integration candidate, not the product currently on `main`.** FineCore's README states that FineCore has no official release or installer; the upstream Orcacode installation examples are expressly not FineCore releases.

## Policy, release flow, and Cargo audit

### What the size gate measures

`ci/check-binary-size.sh` uses `wc -c` on the uncompressed executable file and fails at or above the limit:

- paths containing `*-linux-musl/*`: exclusive `<9,000,000` bytes;
- every other path: exclusive `<6,700,000` bytes.

The script comment labels both limits upstream constraints and says FineCore has not established its own release baseline. `ci/README.md` describes the same limits. They are **not archive, installer, external-dependency, or total-installed-size limits**.

The candidate `.github/workflows/ci.yml` builds the native host release binary on `ubuntu-latest` and applies the default 6.7 MB gate. That is a native GNU/Linux CI build, not the inherited static-musl release target. The release workflow's size-gated rows are different (below).

### FineCore has no active release process

- The copied `.github/workflows/release-orcacode.yml` guards every release job with `github.repository == 'okikorg/orca-harness'`; it does not build or publish from `oigorbrito/finecore`.
- `ci/release.sh` exits with an upstream-only/no-FineCore-release-channel error before any `notes`, `prepare`, `tag`, `build`, or `publish` operation.
- The FineCore updater/background release check is disabled. No FineCore update feed, release destination, installer, or credentials are configured.
- The release workflow checks `SHA256SUMS` and the upstream release server re-hashes uploaded assets. The inspected workflow does not sign binaries, notarize macOS binaries, or produce a code-signing/attestation artifact. Checksums detect byte mismatch only when the checksum source is trusted; they are not a substitute for a signing/provenance policy.

The upstream Orcacode workflow is a useful packaging precedent only. Its matrix declares five target triples:

| Upstream Orcacode target | Packaging | Upstream size gate |
|---|---|---|
| `aarch64-apple-darwin` | one `orcacode` in `.tar.gz` | `<6,700,000` |
| `x86_64-apple-darwin` | one `orcacode` in `.tar.gz` | none |
| `x86_64-unknown-linux-musl` | one `orcacode` in `.tar.gz` | `<9,000,000` |
| `aarch64-unknown-linux-musl` | one `orcacode` in `.tar.gz` | none |
| `x86_64-pc-windows-gnu` | one `orcacode.exe` in `.zip` | none |

All rows are disabled in the FineCore repository by the guard. This matrix is **not evidence that FineCore supports those targets**, nor that the unchecked rows meet a common size budget.

### Manifest and feature contract

The integration candidate is a ten-crate Rust workspace. `rust-toolchain.toml` says `channel = "stable"`; the root Cargo manifest has no `rust-version` minimum. The release profile is `lto = "fat"`, one codegen unit, `opt-level = "z"`, `panic = "abort"`, and `strip = true`. The CLI directly includes the harness, model-provider, tools, tool-extension, and extension crates; the CLI manifest has no product feature matrix for lean/full distributions. `rusqlite` is configured with `bundled`; HTTP uses Rustls. The release profile is already size-oriented and the measured binary includes the linked Rust dependencies.

This report does not reinterpret `--no-default-features` as a smaller product: the preserved FOOTPRINT-01 evidence found it had the same feature graph and identical binary hash. No feature is removed here.

## Platform evidence and limits

The available measurements are platform-specific snapshots, not a common cross-platform release matrix.

| Platform / evidence | Exact scope | FineCore executable | Orca executable | Gate outcome |
|---|---|---:|---:|---|
| Native x86-64 GNU/Linux, Ubuntu/glibc | FOOTPRINT-01 run [40351250854](https://github.com/oigorbrito/finecore/actions/runs/40351250854), FineCore `066229b4…`, Orca `2a9fd399…`; same Linux runner, toolchain 1.99.1, 281-package external graph, and release profile | 9,334,616 B | 9,342,840 B | Both FAIL `<6,700,000`; FineCore is 2,634,616 B over. |
| Windows x86_64-pc-windows-msvc | Paired Windows baseline at integration HEAD `2f980015…`, before the later MCP source transplant; Rust/Cargo 1.99.1, same Windows run/profile for both | 7,861,248 B | 7,868,416 B | Both FAIL `<6,700,000`. FineCore is 1,161,248 B over. |
| macOS | Integration run [40260695414](https://github.com/oigorbrito/finecore/actions/runs/40260695414), head `211bc16…`, `macos-15`; `cargo check --workspace --all-targets` and `cargo test -p finecore` passed | No release executable measured | No paired release executable measured | No binary-size result. This was compile/test coverage, not release packaging. |
| Linux x86_64 musl, Linux aarch64 musl, macOS release targets, Windows GNU | Only appear in the guarded upstream Orcacode release matrix | Not measured for FineCore | Not measured for this FineCore comparison | No FineCore release result. |

The Linux pair is the current FOOTPRINT-01 release-profile baseline, and both builds had repeat-build hashes recorded. The Windows pair is real and paired, but it is an older integration snapshot and MSVC target; it must not be substituted for a current FineCore Windows-GNU release build. The Windows baseline's full-workspace validation also fails on existing Unix-only `std::os::unix::fs::symlink` code. No Windows support promise follows from the successful single CLI build.

The macOS workflow is useful evidence of compile/test feasibility at its recorded head, but it does not answer release size, archive compression, minimum macOS version, signing, notarization, or installed footprint. Linux and Windows byte counts are not cross-platform comparisons: they differ in executable format, target, and source snapshot.

## Executable, archive, dependencies, installer, installed size

FOOTPRINT-01's Linux raw executables were recovered from its preserved artifact and hash-verified. The Windows PE input hashes/sizes are recorded in the Windows baseline JSON; the local PE files were used for the archive probe but were not uploaded as a GitHub Actions artifact. The report's reproducible helper validates each input SHA-256 before producing a normalized one-file archive. No product build was run for this report.

| Snapshot | Raw FineCore binary | One-file reference archive | Raw Orca binary | One-file reference archive |
|---|---:|---:|---:|---:|
| Linux GNU | 9,334,616 B | `.tar.gz` 4,262,983 B (45.67%) | 9,342,840 B | `.tar.gz` 4,266,210 B (45.66%) |
| Windows MSVC baseline | 7,861,248 B | `.zip` 3,921,804 B (49.89%) | 7,868,416 B | `.zip` 3,924,632 B (49.88%) |

Thus, for the measured Linux snapshot, the inherited **raw executable** gate fails at 9,334,616 B even though this illustrative compressed container is 4,262,983 B. The archive does not satisfy or override the raw-size gate; it measures a different layer. These compressed sizes are **controlled reference containers, not published FineCore assets**. They package only the already-built executable, at the root, with normalized metadata and compression level 6. The Linux raw input is GNU/glibc, not the upstream musl release target. The Windows input is MSVC, not the upstream Windows-GNU target. The Windows executable is retained locally and its hash is recorded, but an external reviewer needs that exact executable (or a reproducible same-hash build) to replay the ZIP calculation. Do not compare these reference archives with upstream Orcacode release downloads or treat them as a FineCore download promise.

| Distribution layer | Evidence / current status |
|---|---|
| Executable | Measured for the two snapshots above. This is the only dimension the inherited shell gate directly measures. |
| Compressed package | Reference one-file containers measured as above; no official FineCore archive format or assets exist. |
| Installer | Not measured / not applicable to a FineCore release: there is no FineCore installer. README install commands and scripts point to upstream Orcacode. |
| External OS dependencies | The GNU/Linux executable's ELF `NEEDED` entries are `libgcc_s.so.1`, `libm.so.6`, and `libc.so.6`; these are not inside the binary or reference tar and their installed bytes vary by distribution. Rust crates are otherwise linked into the executable. No comparable Windows or macOS dependency inventory is available. The upstream musl target is intended to be static, but no FineCore musl executable was measured. |
| External tools/services | Not in the binary/archive: user-selected model providers, local model servers, MCP servers, and feature-specific tools/runtimes such as Python/Bun/Docker. The README says those runtimes are not required merely to launch the native CLI; that does not mean every optional tool works without its external runtime. |
| Total installed footprint | Not measured. A one-file archive expands to approximately the executable file size, plus target OS libraries and any user-installed tools. Config, state, caches, optional assets, installer residue, and per-user data have no bounded measured install profile. |

The upstream release notes describe a single executable at the archive root, SHA256SUMS, and an installer that writes one binary under `~/.local/bin`. This is an upstream deployment design, not a FineCore requirement or measured FineCore installed-size contract. A checksum or compressed archive can reduce transfer bytes; it cannot make the extracted executable smaller or establish a disk quota.

## Product needs established by repository evidence

**Verified:** the candidate is a native Rust terminal coding agent; it supports local and remote model configurations and optional tools/extensions. FineCore's README expressly says there is no official release/installer yet. The identity inventory says there is no FineCore release channel and the upstream update/release paths are disabled. The Rust workspace and CLI are still proposed by Draft PR #111, not present on `main`.

**Not specified:** supported/release target tuples and minimum OS versions; an MSRV; target download limits; install/storage ceilings; offline-install or air-gapped requirements; whether all optional tools/assets must ship together; update/rollback expectations; signing/notarization/attestation requirements; release support lifetime; or allowed size regression. No user or product specification in the audited repository supplies a number for any of these. Therefore this report does not invent a byte budget.

## Packaging alternatives and trade-offs

1. **One executable per target, one compressed archive.** Closest to upstream precedent and simplest to install/offline once downloaded. The archive is not the installed size. GNU/Linux builds still depend on host system libraries; static-musl can reduce that dependency but changes the executable footprint and requires target-specific validation.
2. **Target-specific installer plus complete payload.** Can verify checksums/signatures, select a target, install to a standard location, and manage upgrades/rollback. Its measured budget must include the installer, downloaded package, extracted files, and any retained old version/cache. FineCore currently has no installer or release channel.
3. **System/dynamic libraries or external helper services.** May shrink the main executable, but transfers size to OS prerequisites or separately versioned packages and changes offline, portability, security-update, and support responsibilities. No target matrix or dependency policy currently authorizes this trade.
4. **Separate optional feature/assets bundles.** Could shrink a core executable, but would not preserve “no functionality loss” unless the full installation also obtains every required component, works offline when required, and keeps component versions compatible. Total installed size must count all bundles; only the main executable might become smaller.

No option is selected. The fact that the reference compressed archive is smaller does not establish a product requirement for a small installer or a viable new binary budget.

## Objective budget process (no numeric threshold selected)

Before adopting a FineCore budget, product/release owners should record:

1. Supported target triples, minimum OS/libc versions, and which are release-supported versus merely compile-tested.
2. Per-target delivery format and whether external OS libraries, runtime tools, assets, or helpers are bundled.
3. User-derived ceilings, if any, for download bytes, extracted install bytes, disk/cache use, or offline media—not a number copied from Orca.
4. Update/rollback, offline installation, signing/notarization, checksum/attestation, and security-support requirements.

Then measure each approved target independently from a locked source/toolchain/profile with two clean builds and hashes. Record at least: raw executable bytes; compressed published asset bytes and checksum; installer bytes; installed file-set bytes (excluding user data, with exclusions enumerated); external dependency names/versions and whether bundled; and a clean offline-install/launch smoke test. A target's budget and regression policy must reference the specific user-derived constraint and packaging layer it protects. Missing/untested target data should remain “not measured,” never be inferred from Linux, Windows, or Orca.

Until that decision exists, keep the inherited 6.7 MB / 9 MB checks unchanged and visibly FAIL where they currently fail. Do not call the current 9.33 MB build a budget, and do not set a percent-regression allowance without an owner-approved requirement.

## Reproduction, validations, and non-actions

- Archived Linux binaries and all original FOOTPRINT-01 measurements: [run 40351250854](https://github.com/oigorbrito/finecore/actions/runs/40351250854), [artifact 12678024189](https://github.com/oigorbrito/finecore/actions/runs/40351250854/artifacts/12678024189). That run completed measurements but its final renderer failed on a duplicate `logs/` prefix; the report/evidence were recovered from the artifact without rebuilding.
- Windows paired sizes, toolchain, profile, lock-graph, hashes, and gate results: [`evidence/windows-baseline.json`](release-budget-01/evidence/windows-baseline.json), copied from PR #114's [`inherited-gates-01-windows-baseline.json`](https://github.com/oigorbrito/finecore/blob/investigate/inherited-gates-01/docs/validation-logs/inherited-gates-01-windows-baseline.json).
- Archive calculations and input checksums: [`evidence/release-budget-01-evidence.json`](release-budget-01/evidence/release-budget-01-evidence.json). Run `python scripts/release_budget_01.py <manifest.json>` with paths to the preserved inputs and the recorded hashes; the helper makes no product build or file publication.
- Validation: four focused Python tests, Ruff, `py_compile`, and staged `git diff --check` all pass. No Rust build, platform build, benchmark, provider call, model inference, release action, or policy/gate change was performed for this report.
- Candidate Linux gates remain red: binary-size limit, 14 source-size violations, and 13 strict Clippy diagnostics as recorded by FOOTPRINT-01. This report does not waive, suppress, or alter them.

**Recommendation:** approve no numeric FineCore size budget in this investigation. Keep the inherited checks unchanged and failing, resolve the product target/distribution requirements first, then run release measurements only for the explicitly supported target matrix. This Draft report requests no merge, release, or force-push.
