# FOOTPRINT-01 controlled experiment results
- Preserved Actions artifact: https://github.com/oigorbrito/finecore/actions/runs/40351250854/artifacts/12678024189

- FineCore HEAD: `066229b4eb35f061e837197ebd105710fb42c80b`
- Canonical Orca: `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`
- Environment: `Linux-6.17.0-1022-azure-x86_64-with-glibc2.39` (x86_64)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1 (5f94df478 2026-08-27), clippy 0.1.99 (58cf8f9c35 2026-10-13)
- Run failure: The final report step raised FileNotFoundError for logs/logs/tool-version-rustc.log: the log reference already contained logs/ and the renderer added it again. All experiments completed and their evidence was uploaded.
- External lock graphs identical: `True`; release profiles identical at baseline: `True`
- Inherited size limit: `<6,700,000` bytes; unchanged. Provider calls: none.

## Variant results

Sizes are raw executable file bytes. Savings are relative to FineCore baseline; negative values mean growth. Repeat hash checks use a second empty target directory.

| Build | Configuration | Bytes | Delta B | Savings % | Compile s (first/repeat) | Repeat identical | Release tests | Bench smoke | MCP | Size gate | Compatibility | Decision |
|---|---|---:|---:|---:|---:|---|---|---|---:|---|---|---|
| finecore/baseline | baseline | 9334616 | 0 | 0.000 | 113.583/112.994 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Reference; keep unchanged |
| orca/baseline | baseline | 9342840 | 8224 | -0.088 | 114.222/115.408 | True | 0 | 0 | 20/20 | 1 | 2/2 valid | Reference; keep unchanged |
| finecore/thin-lto | release.lto=thin, bench.lto=thin | 10841752 | 1507136 | -16.146 | 92.346/91.009 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Experiment only; no promotion without cross-platform review |
| finecore/lto-off | release.lto=false, bench.lto=false | 11054328 | 1719712 | -18.423 | 85.808/84.886 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Experiment only; no promotion without cross-platform review |
| finecore/opt-level-s | release.opt_level=s, bench.opt_level=s | 9960504 | 625888 | -6.705 | 125.005/129.747 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Experiment only; no promotion without cross-platform review |
| finecore/codegen-units-16 | release.codegen_units=16, bench.codegen_units=16 | 9955032 | 620416 | -6.646 | 119.108/114.607 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Experiment only; no promotion without cross-platform review |
| finecore/panic-unwind | release.panic=unwind, bench.panic=unwind | 10452952 | 1118336 | -11.981 | 121.763/126.531 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Experiment only; no promotion without cross-platform review |
| finecore/unwind-tables-off | RUSTFLAGS=-C force-unwind-tables=no | 9334616 | 0 | 0.000 | 118.75/115.425 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | Diagnostic only; reject if unwind data/debuggability is lost |
| finecore/no-default-features | --no-default-features | 9334616 | 0 | 0.000 | 116.282/115.008 | True | 0 | 0 | 20/20 | 1 | 2/2 valid (2 invalid legacy probe(s) excluded) | No-op only if feature graph matches; otherwise reject |

## Section attribution: baseline ELF

| Section | FineCore bytes | Orca bytes | Interpretation |
|---|---:|---:|---|
| `.text` | 5133016 | 5139416 | linked executable code |
| `.rodata` | 1704613 | 1704613 | read-only literals/tables/assets |
| `.eh_frame_hdr` | 124156 | 124396 | unwind index |
| `.eh_frame` | 884744 | 886040 | unwind metadata |
| `.gcc_except_table` | 6688 | 6700 | language exception tables |
| `.rela.dyn` | 796104 | 796584 | dynamic relocations |
| `.rela.plt` | 1104 | 1104 | PLT relocations |
| `.data.rel.ro` | 648888 | 649176 | relocations followed by read-only protection |
| `.dynamic` | 464 | 464 | dynamic-link metadata |
| `.got` | 2272 | 2272 | global offset table |
| `.got.plt` | 392 | 392 | PLT global offset table |
| `.plt` | 752 | 752 | procedure linkage stubs |
| `.dynsym` | 4680 | 4680 | dynamic symbol table |
| `.dynstr` | 2626 | 2626 | dynamic symbol strings |
| `.data` | 18808 | 18808 | initialized writable data |
| `.bss` | 1584 | 1584 | zero-initialized memory; NOBITS, not file payload |

`size -A`, `readelf -SW/-d/-Ws`, and the stripped binary hashes are in `logs/`. ELF section totals are not identical to file bytes; NOBITS sections such as `.bss` occupy memory but not file payload. Debug/symbol audit builds are separately marked diagnostic and are never counted as candidates.

## Dependency and symbol attribution

`cargo-bloat` reports executable text by crate/function from an unstripped, debuginfo-enabled audit build. It does not apportion `.rodata`, relocation, or unwind bytes; LTO/inlining can blur crate attribution. Full outputs are linked below.

### finecore

- `logs/finecore-bloat-crates.log` (inside the Actions artifact)

```text
Error: warning: please specify `--format-version` flag explicitly to avoid compatibility problems
error: failed to download `bumpalo v3.20.3`

Caused by:
  attempting to make an HTTP request, but --offline was specified.
```
- `logs/finecore-bloat-top-functions.log` (inside the Actions artifact)

```text
Error: warning: please specify `--format-version` flag explicitly to avoid compatibility problems
error: failed to download `bumpalo v3.20.3`

Caused by:
  attempting to make an HTTP request, but --offline was specified.
```

### orca

- `logs/orca-bloat-crates.log` (inside the Actions artifact)

```text
Error: warning: please specify `--format-version` flag explicitly to avoid compatibility problems
error: failed to download `bumpalo v3.20.3`

Caused by:
  attempting to make an HTTP request, but --offline was specified.
```
- `logs/orca-bloat-top-functions.log` (inside the Actions artifact)

```text
Error: warning: please specify `--format-version` flag explicitly to avoid compatibility problems
error: failed to download `bumpalo v3.20.3`

Caused by:
  attempting to make an HTTP request, but --offline was specified.
```

## Paired startup, memory, and functional benchmark results

Startup is process launch plus `--version`, timed with `/usr/bin/time`; RSS is maximum child RSS in KiB. The file-operation tests validate outputs and accuracy, not agent/model behavior. Medians and 95% paired bootstrap intervals are exploratory; multiple comparisons are not corrected.

### Startup latency and memory

| Pair (left vs right) | Startup median % | 95% CI % | Median max-RSS delta KiB | Samples |
|---|---:|---:|---:|---:|
| baseline-finecore-vs-orca | -0.097 | [-0.782, 1.91] | -34.0 | 30 |
| finecore-thin-lto-vs-baseline | 5.356 | [3.25, 6.796] | 540.0 | 30 |
| finecore-lto-off-vs-baseline | 5.173 | [4.26, 6.626] | 604.0 | 30 |
| finecore-opt-level-s-vs-baseline | -1.401 | [-2.62, -0.391] | -540.0 | 30 |
| finecore-codegen-units-16-vs-baseline | 0.954 | [-0.386, 2.827] | 554.0 | 30 |
| finecore-panic-unwind-vs-baseline | 0.542 | [-0.991, 2.274] | 278.0 | 30 |
| finecore-unwind-tables-off-vs-baseline | 1.241 | [-0.494, 2.303] | -40.0 | 30 |
| finecore-no-default-features-vs-baseline | -0.09 | [-1.77, 1.779] | -34.0 | 30 |

### Core-tools paired workload timing

Negative time deltas mean the left-hand executable was faster in the paired samples. These are local microbenchmarks, not overall agent performance.

| Pair | Scenario | Metric | Median delta % | 95% CI % | Pairs |
|---|---|---|---:|---:|---:|
| finecore-baseline-vs-orca | append_files | wall_seconds | 0.688 | [-1.376, 2.149] | 20 |
| finecore-baseline-vs-orca | batch_files | wall_seconds | -0.327 | [-3.063, 3.254] | 20 |
| finecore-baseline-vs-orca | distinct | wall_seconds | 0.533 | [-3.227, 3.982] | 20 |
| finecore-baseline-vs-orca | distinct | single_average_seconds | 1.18 | [-2.861, 3.608] | 20 |
| finecore-baseline-vs-orca | guarded_distinct | wall_seconds | 0.365 | [-4.295, 4.527] | 20 |
| finecore-baseline-vs-orca | guarded_distinct | single_average_seconds | 1.094 | [-1.081, 3.066] | 20 |
| finecore-baseline-vs-orca | repeated_file | wall_seconds | 1.502 | [-2.379, 3.962] | 20 |
| finecore-thin-lto-vs-baseline | append_files | wall_seconds | 0.454 | [-1.702, 1.955] | 20 |
| finecore-thin-lto-vs-baseline | batch_files | wall_seconds | 6.334 | [2.753, 8.731] | 20 |
| finecore-thin-lto-vs-baseline | distinct | wall_seconds | 2.716 | [1.322, 6.947] | 20 |
| finecore-thin-lto-vs-baseline | distinct | single_average_seconds | 5.146 | [3.177, 8.69] | 20 |
| finecore-thin-lto-vs-baseline | guarded_distinct | wall_seconds | 0.066 | [-3.778, 9.457] | 20 |
| finecore-thin-lto-vs-baseline | guarded_distinct | single_average_seconds | 6.949 | [4.755, 9.055] | 20 |
| finecore-thin-lto-vs-baseline | repeated_file | wall_seconds | 26.327 | [23.477, 28.634] | 20 |
| finecore-lto-off-vs-baseline | append_files | wall_seconds | 1.571 | [-0.39, 3.702] | 20 |
| finecore-lto-off-vs-baseline | batch_files | wall_seconds | 1.84 | [-2.277, 8.445] | 20 |
| finecore-lto-off-vs-baseline | distinct | wall_seconds | -0.34 | [-5.103, 2.586] | 20 |
| finecore-lto-off-vs-baseline | distinct | single_average_seconds | 1.37 | [-2.522, 2.75] | 20 |
| finecore-lto-off-vs-baseline | guarded_distinct | wall_seconds | 4.325 | [2.845, 10.011] | 20 |
| finecore-lto-off-vs-baseline | guarded_distinct | single_average_seconds | 1.973 | [-0.948, 3.687] | 20 |
| finecore-lto-off-vs-baseline | repeated_file | wall_seconds | -2.725 | [-5.174, 1.526] | 20 |
| finecore-opt-level-s-vs-baseline | append_files | wall_seconds | 1.166 | [-0.052, 2.211] | 20 |
| finecore-opt-level-s-vs-baseline | batch_files | wall_seconds | 4.072 | [0.086, 6.393] | 20 |
| finecore-opt-level-s-vs-baseline | distinct | wall_seconds | 2.877 | [-1.482, 6.33] | 20 |
| finecore-opt-level-s-vs-baseline | distinct | single_average_seconds | 4.77 | [0.439, 7.29] | 20 |
| finecore-opt-level-s-vs-baseline | guarded_distinct | wall_seconds | 1.272 | [-1.795, 6.335] | 20 |
| finecore-opt-level-s-vs-baseline | guarded_distinct | single_average_seconds | 3.917 | [1.785, 8.441] | 20 |
| finecore-opt-level-s-vs-baseline | repeated_file | wall_seconds | 20.918 | [16.257, 22.757] | 20 |
| finecore-codegen-units-16-vs-baseline | append_files | wall_seconds | 1.007 | [-1.848, 2.784] | 20 |
| finecore-codegen-units-16-vs-baseline | batch_files | wall_seconds | 3.396 | [1.829, 7.902] | 20 |
| finecore-codegen-units-16-vs-baseline | distinct | wall_seconds | 2.709 | [-1.353, 6.307] | 20 |
| finecore-codegen-units-16-vs-baseline | distinct | single_average_seconds | 6.481 | [1.795, 10.667] | 20 |
| finecore-codegen-units-16-vs-baseline | guarded_distinct | wall_seconds | 8.273 | [1.375, 11.516] | 20 |
| finecore-codegen-units-16-vs-baseline | guarded_distinct | single_average_seconds | 4.656 | [1.938, 10.405] | 20 |
| finecore-codegen-units-16-vs-baseline | repeated_file | wall_seconds | 25.206 | [21.615, 27.353] | 20 |
| finecore-panic-unwind-vs-baseline | append_files | wall_seconds | 2.049 | [-2.355, 3.449] | 20 |
| finecore-panic-unwind-vs-baseline | batch_files | wall_seconds | 0.192 | [-3.39, 1.363] | 20 |
| finecore-panic-unwind-vs-baseline | distinct | wall_seconds | -0.375 | [-2.819, 4.087] | 20 |
| finecore-panic-unwind-vs-baseline | distinct | single_average_seconds | 1.698 | [-0.828, 4.13] | 20 |
| finecore-panic-unwind-vs-baseline | guarded_distinct | wall_seconds | -0.647 | [-2.532, 5.863] | 20 |
| finecore-panic-unwind-vs-baseline | guarded_distinct | single_average_seconds | 0.336 | [-3.601, 2.725] | 20 |
| finecore-panic-unwind-vs-baseline | repeated_file | wall_seconds | 5.176 | [-5.74, 7.575] | 20 |
| finecore-unwind-tables-off-vs-baseline | append_files | wall_seconds | -0.514 | [-1.889, 1.784] | 20 |
| finecore-unwind-tables-off-vs-baseline | batch_files | wall_seconds | -2.986 | [-5.466, 1.412] | 20 |
| finecore-unwind-tables-off-vs-baseline | distinct | wall_seconds | -0.616 | [-5.166, 2.762] | 20 |
| finecore-unwind-tables-off-vs-baseline | distinct | single_average_seconds | 0.242 | [-2.029, 1.337] | 20 |
| finecore-unwind-tables-off-vs-baseline | guarded_distinct | wall_seconds | -2.406 | [-5.989, 1.091] | 20 |
| finecore-unwind-tables-off-vs-baseline | guarded_distinct | single_average_seconds | -2.175 | [-4.226, 2.977] | 20 |
| finecore-unwind-tables-off-vs-baseline | repeated_file | wall_seconds | -0.469 | [-6.097, 1.125] | 20 |
| finecore-no-default-features-vs-baseline | append_files | wall_seconds | 0.05 | [-1.198, 1.834] | 20 |
| finecore-no-default-features-vs-baseline | batch_files | wall_seconds | 0.052 | [-3.01, 1.679] | 20 |
| finecore-no-default-features-vs-baseline | distinct | wall_seconds | -0.582 | [-2.626, 2.273] | 20 |
| finecore-no-default-features-vs-baseline | distinct | single_average_seconds | 0.4 | [-0.907, 5.246] | 20 |
| finecore-no-default-features-vs-baseline | guarded_distinct | wall_seconds | 4.991 | [-2.322, 8.759] | 20 |
| finecore-no-default-features-vs-baseline | guarded_distinct | single_average_seconds | -1.382 | [-2.686, 0.809] | 20 |
| finecore-no-default-features-vs-baseline | repeated_file | wall_seconds | 0.736 | [-2.253, 8.516] | 20 |

## Preserved nonzero outcomes

These exit codes are retained verbatim; inherited CI failures and diagnostic tooling failures are not waived.

| Command | Exit |
|---|---:|
| `baseline-finecore-source-size` | 1 |
| `baseline-finecore-clippy` | 101 |
| `baseline-orca-source-size` | 1 |
| `baseline-orca-clippy` | 101 |
| `finecore-baseline-binary-size-gate` | 1 |
| `finecore-baseline-compat-2` | 1 |
| `finecore-baseline-compat-3` | 1 |
| `orca-baseline-binary-size-gate` | 1 |
| `finecore-thin-lto-binary-size-gate` | 1 |
| `finecore-thin-lto-compat-2` | 1 |
| `finecore-thin-lto-compat-3` | 1 |
| `finecore-lto-off-binary-size-gate` | 1 |
| `finecore-lto-off-compat-2` | 1 |
| `finecore-lto-off-compat-3` | 1 |
| `finecore-opt-level-s-binary-size-gate` | 1 |
| `finecore-opt-level-s-compat-2` | 1 |
| `finecore-opt-level-s-compat-3` | 1 |
| `finecore-codegen-units-16-binary-size-gate` | 1 |
| `finecore-codegen-units-16-compat-2` | 1 |
| `finecore-codegen-units-16-compat-3` | 1 |
| `finecore-panic-unwind-binary-size-gate` | 1 |
| `finecore-panic-unwind-compat-2` | 1 |
| `finecore-panic-unwind-compat-3` | 1 |
| `finecore-unwind-tables-off-binary-size-gate` | 1 |
| `finecore-unwind-tables-off-compat-2` | 1 |
| `finecore-unwind-tables-off-compat-3` | 1 |
| `finecore-no-default-features-binary-size-gate` | 1 |
| `finecore-no-default-features-compat-2` | 1 |
| `finecore-no-default-features-compat-3` | 1 |
| `finecore-bloat-crates` | 1 |
| `finecore-bloat-top-functions` | 1 |
| `orca-bloat-crates` | 1 |
| `orca-bloat-top-functions` | 1 |

## Feature graph and interpretation

Smallest eligible, non-diagnostic build that passed valid Linux functional checks: `baseline` at 9,334,616 bytes; it remains 2,634,616 bytes above the inherited cap. This is not cross-platform approval.
- FineCore baseline gap to the unchanged cap: 2634616 bytes.
- `cargo tree -e features` identical with/without `--no-default-features`: `True` (hashes and raw output in JSON/logs).
- Baseline source-size, strict Clippy, and 6.7 MB binary gates are reported with their real exit codes; no threshold, lint, source-size rule, or feature set is silently changed.
- All variants are Linux-only measurements. No variant is portable/production-approved until the exact candidate is built and tested on every supported target.
- No profile variant was combined after the fact. A smaller but diagnostic-only build is not a safe minimum.
- The original `plugin --help` and `plugin validate --help` checks were invalid for this custom CLI parser; their nonzero outcomes remain in the raw records but are excluded from functional eligibility. Valid `--version`, top-level `--help`, and `plugin list` checks are used in subsequent runs.
- cargo-bloat attribution failed because the offline audit build lacked cached `bumpalo v3.20.3`; section/symbol data remains available and the attribution failure is preserved.
- Complete raw records: `footprint-01-results.json`; per-command stdout/stderr in `logs/`; copied executables and test harnesses in `binaries/`.
