# FOOTPRINT-ATTRIBUTION-02 — diagnostic size attribution

## Scope, ancestry, and safety

This is an isolated follow-up based on PR #116's head, itself stacked on PR #115. It does not update either PR branch, `main`, PRs #110–#116, the release profile, feature set, gates, or the 6.7 MB limit. FineCore source/manifest files must be byte-for-byte Git-blob-identical to FOOTPRINT-01 source head `066229b4eb35f061e837197ebd105710fb42c80b`; the donor must be canonical Orca commit `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`.

The investigation starts with the preserved FOOTPRINT-01 artifact `12678024189`. It will not rerun the nine baseline/variant measurements, functional tests, startup/RSS trials, or paired benchmarks. The only additional compilations allowed are one equivalent **diagnostic release build** for FineCore and one for Orca, with symbols and level-1 debug info enabled so crate/function attribution can be recovered. The inherited release settings (fat LTO, `opt-level=z`, one codegen unit, `panic=abort`) remain in force; diagnostic `strip=false` and `debug=1` are not candidate settings and their executable sizes are not shipping-size measurements.

## Existing evidence examined first

The preserved stripped FineCore ELF is 9,334,616 bytes; Orca is 9,342,840 bytes. Their `size -A`/`readelf -SW` sections show the largest file-backed regions:

| FineCore section | Bytes | Orca bytes | What the section establishes |
|---|---:|---:|---|
| `.text` | 5,133,016 | 5,139,416 | Executable code; crate attribution needs symbols/link data |
| `.rodata` | 1,704,613 | 1,704,613 | Read-only constants/tables/assets; same total for both |
| `.eh_frame` | 884,744 | 886,040 | Unwind/frame metadata, not ordinary code |
| `.rela.dyn` | 796,104 | 796,584 | Relocation records; not directly attributable to one crate |
| `.data.rel.ro` | 648,888 | 649,176 | Relocated read-only objects/vtables/constant pointers |
| `.eh_frame_hdr` | 124,156 | 124,396 | Index for unwind/frame data |
| `.gcc_except_table` | 6,688 | 6,700 | Language exception tables |

Other sections are much smaller. `.bss` and `.tbss` are NOBITS memory, not executable file payload. The shipped binary dynamically requires only `libgcc_s.so.1`, `libm.so.6`, and `libc.so.6`; the dependency code and tables discussed below are linked into the executable. The stripped binary's `.dynsym` has 195 entries, but its static `.symtab` and debug sections are absent, so shipped-image symbols alone cannot recover Rust crate ownership.

The existing diagnostic `nm -S --size-sort --demangle` output has 16,148 nonzero symbol rows per image. Its largest named rows include the `ring` `ecp_nistz256_precomputed` read-only table (151,552 B), `regex_automata::meta::strategy::new` (80,438 B), the `html_escape` named-entity table (68,000 B), FineCore's `tui::render::shell::live::live_region` (62,280 B), FineCore's async worker state machine (48,445 B), `html_to_markdown_rs::converter::main::walk_node` (42,250 B), FineCore's interactive runtime (39,120 B), `rustls` handshake parsing (32,616 B), and bundled SQLite `sqlite3VdbeExec` (28,311 B). These are individual diagnostic symbol sizes—not removable feature savings or summed crate totals.

FineCore's `.text` is 6,400 B smaller than Orca's; `.rodata` is identical. The other major section differences are also small. This is consistent with the 8,224 B whole-file difference, not a multi-megabyte donor-specific opportunity.

## Symbol and crate attribution limits

- LTO can inline across crate boundaries, internalize functions, merge identical code, and specialize generics. A demangled namespace is a useful symbol label, not proof that all bytes were emitted by that dependency or can be reclaimed by removing it.
- `nm` symbol sizes can include aliases, thunks, monomorphized closures, or ranges whose attribution differs from Cargo package ownership. Do not sum symbol sizes as crate totals or subtract them from the stripped binary.
- The previous `cargo-bloat` attempts failed because the offline audit cache did not contain locked `bumpalo v3.20.3`. Those failed outputs are retained; they are not treated as zero contribution.
- `cargo-bloat --crates` measures linked text, not `.rodata`, relocations, unwind records, or all linker metadata. The section table and named data symbols must be interpreted separately.
- Diagnostic `strip=false`/`debug=1` outputs contain debug/symbol data absent from the shipped executable. Only loadable/code contribution and section names inform the shipped image; diagnostic file size is never a candidate size.

## Dependency preflight and minimal diagnostic run

Before any `cargo-bloat` command, the runner:

1. Confirms the exact FineCore source ancestry, unchanged Cargo manifests/lockfile, canonical Orca SHA, matching release-profile contract, toolchain file, and 281-package external lock graph from FOOTPRINT-01.
2. Runs `cargo fetch --locked` separately for both checkouts while network access is enabled. No lockfile may change.
3. Verifies every registry archive against the checksum in its lockfile, including `bumpalo 3.20.3` with SHA-256 `72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649`. If any package is missing/mismatched, the runner refuses to invoke `cargo-bloat`.
4. Installs pinned `cargo-bloat 0.12.1`, then switches project builds and analysis to Cargo offline mode.
5. Builds each CLI once with the diagnostic profile, runs `cargo-bloat --crates` and top-100 function analysis, and captures `nm`, `readelf`, `size`, hashes, timings, and all commands. It runs no tests or benchmarks.

The workflow is `.github/workflows/footprint-attribution-02.yml`; raw commands and the complete checksum manifest are uploaded even on failure. `scripts/footprint_attribution_02.py` is the reproducible runner. No product source or feature is removed.

## Decision rule

The final report will rank only contributions demonstrated by the paired diagnostic outputs, separate text from data/metadata, and attach explicit uncertainty. Reduction estimates will be labeled as theoretical upper bounds or source-level hypotheses, not forecast savings. A material architectural opportunity requires a coherent, functionality-preserving removal/replacement candidate supported by the measured sections and crate attribution; the report will not promise that any candidate reaches 6.7 MB. All inherited size, source-size, and Clippy gates remain unchanged and visible.
