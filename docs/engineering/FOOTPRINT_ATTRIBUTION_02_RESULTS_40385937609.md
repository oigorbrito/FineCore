# FOOTPRINT-ATTRIBUTION-02 — linked-code and data attribution results

**Status:** diagnostic investigation complete; no size optimization or product change proposed.

- Workflow: [run 40385937609](https://github.com/oigorbrito/finecore/actions/runs/40385937609)
- Raw artifact: [footprint-attribution-02-40385937609, artifact 12680307162](https://github.com/oigorbrito/finecore/actions/runs/40385937609/artifacts/12680307162)
- Report source: FineCore experiment head `37eb8d86c10923edeebbc983d06b5a478b014bbd`; canonical Orca `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`
- Durable selected evidence: [`footprint-attribution-02/evidence/`](footprint-attribution-02/evidence/)

## Conclusion

The evidence identifies several sizable code and data areas, but **does not establish a safe, functionality-preserving change that reaches the inherited exclusive 6,700,000-byte limit**. No single named FineCore code contribution is close to the 2,634,616-byte baseline gap. Reaching the limit appears to require a multi-subsystem packaging/architecture change and/or a substantial reduction of currently unattributed data/metadata. The estimates below are deliberately optimistic upper bounds or investigation hypotheses—not forecasts.

A useful architectural direction to investigate later is splitting selected payloads (for example, syntax/theme assets, HTML conversion, or optional integrations) into separately packaged components while preserving them in the overall distribution. That could reduce the *main executable*, but this run cannot quantify the savings or show that the result would meet the gate. Moving assets out of the executable does not by itself reduce total installed bytes. **No target size, feature removal, gate change, or profile change is approved.**

## Reproducibility and scope

The pinned runner built FineCore and canonical Orca once each with the inherited release settings (`lto=fat`, `opt-level=z`, one codegen unit, `panic=abort`) and diagnostic-only `strip=false`, `debug=1`. It verified the identical 281-package external lock graph and all 281 registry archive checksums in each checkout before invoking `cargo-bloat`; `bumpalo 3.20.3` was downloaded and verified against SHA-256 `72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649`. Toolchain was Rust/Cargo 1.99.1 and `cargo-bloat` 0.12.1 on x86-64 Linux.

All 18 recorded runner commands exited zero. Both diagnostic builds, both `nm`/`readelf`/`size` captures, and all four `cargo-bloat` analyses completed. The runner performed **no Rust tests, benchmark, startup/RSS trial, or FOOTPRINT-01 variant rerun**. The builds emitted three existing unused/dead-code warnings in `clipboard_image.rs`; they did not fail. The complete summary and checksum manifest are tracked under the evidence directory; full symbol dumps and every command log are in the workflow artifact.

Diagnostic executable sizes were 56,769,048 B (FineCore, SHA-256 `65bbe6be9f28c718ee58b036eeff4813d9e425f639b64736c7dbb311673b93aa`) and 56,875,240 B (Orca, SHA-256 `c10cb6440627dcd2beff4b1e90ad8b93608de4c1663fc7b0e88f0652f97ed4c0`). These include debug and symbol sections and are **not shipping sizes**. The debug sections alone account for tens of megabytes. `cargo-bloat` likewise reports a 54.1 MiB diagnostic file; its useful output here is the approximate linked `.text` attribution, not that file size.

## Section evidence

The stripped shipping-image measurements below are from the preserved FOOTPRINT-01 run 40351250854. The additional diagnostic `readelf` run retained symbols/debug information, so its section values are shown separately rather than substituted for shipping measurements.

| Shipping section | FineCore bytes | Orca bytes | Notes |
|---|---:|---:|---|
| `.text` | 5,133,016 | 5,139,416 | Executable code |
| `.rodata` | 1,704,613 | 1,704,613 | Read-only constants/tables/assets |
| `.eh_frame` | 884,744 | 886,040 | Frame/unwind metadata |
| `.rela.dyn` | 796,104 | 796,584 | Dynamic relocation records |
| `.data.rel.ro` | 648,888 | 649,176 | Relocated read-only objects/pointers |
| `.eh_frame_hdr` | 124,156 | 124,396 | Frame index |
| `.gcc_except_table` | 6,688 | 6,700 | Exception metadata |
| **Stripped file** | **9,334,616** | **9,342,840** | FineCore is 8,224 B smaller |

Shipping `.text` plus `.rodata` is 6,837,629 B—already 137,629 B above the exclusive 6,700,000-byte limit, even in the impossible lower-bound scenario where every other file section disappeared. The actual baseline is 2,634,616 B over the limit.

The diagnostic FineCore ELF measured `.text` 5,171,416 B, `.rodata` 1,704,613 B, `.eh_frame` 857,380 B, `.rela.dyn` 795,360 B, and `.data.rel.ro` 648,408 B. Orca's corresponding diagnostic `.text` was 5,176,920 B and `.rodata` was identical. Diagnostic `.text` differs from the stripped shipping image, so `cargo-bloat` values must not be treated as exact shipping-byte deltas.

## Largest measured `.text` contributors

`cargo-bloat --crates` ranks linked text and explicitly warns: “numbers above are a result of guesswork. They are not 100% correct and never will be.” The first 20 FineCore rows are below; the remaining 104 crates aggregate to 590.0 KiB. These are approximate attribution buckets, not independently removable units, and they say nothing about `.rodata` or relocation/unwind sections.

| Rank | FineCore bucket | Approx. `.text` | Share of `.text` |
|---:|---|---:|---:|
| 1 | `[Unknown]` | 758.8 KiB | 15.0% |
| 2 | `finecore` | 645.9 KiB | 12.8% |
| 3 | `std` | 601.7 KiB | 11.9% |
| 4 | `orca_harness_tools` | 339.0 KiB | 6.7% |
| 5 | `regex_automata` | 303.5 KiB | 6.0% |
| 6 | `html_to_markdown_rs` | 290.1 KiB | 5.7% |
| 7 | `rustls` | 245.3 KiB | 4.9% |
| 8 | `ring` | 181.6 KiB | 3.6% |
| 9 | `orca_harness_tool_extensions` | 181.2 KiB | 3.6% |
| 10 | `orca_harness_model_providers` | 176.2 KiB | 3.5% |
| 11 | `reqwest` | 118.4 KiB | 2.3% |
| 12 | `aho_corasick` | 114.3 KiB | 2.3% |
| 13 | `regex_syntax` | 97.7 KiB | 1.9% |
| 14 | `orca_harness_extensions` | 92.0 KiB | 1.8% |
| 15 | `tokio` | 70.4 KiB | 1.4% |
| 16 | `orca_harness_dag` | 62.6 KiB | 1.2% |
| 17 | `syntect` | 46.7 KiB | 0.9% |
| 18 | `orca_harness_core` | 46.3 KiB | 0.9% |
| 19 | `libsqlite3_sys` | 42.5 KiB | 0.8% |
| 20 | `serde_json` | 39.0 KiB | 0.8% |

The largest known bucket, the FineCore executable crate itself, is only about 661,402 B of attributed text—25.1% of the 2,634,616-byte gap, even under the unrealistic assumption that every byte could be removed. The `html_to_markdown_rs` text bucket is about 297,062 B (11.3% of the gap). `std` is runtime foundation, not a stand-alone removable feature. `[Unknown]` is the largest single bucket but is not a crate or a safe reduction estimate.

The FineCore/Orca rankings are closely matched: the executable crate is 645.9/649.8 KiB; tools 339.0/339.0 KiB; HTML conversion 290.1/292.6 KiB; tool extensions 181.2/180.9 KiB; model providers 176.2/158.8 KiB. This agrees with the small 8,224-byte stripped-image difference; the migration has not introduced a multi-megabyte code-size increase.

The largest `cargo-bloat --functions` observations also show where some code resides: `regex_automata::meta::strategy::new` 78.6 KiB; FineCore's `live_region` 60.8 KiB; FineCore worker state machine 47.3 KiB; `html_to_markdown_rs::converter::main::walk_node` 41.3 KiB; FineCore interactive runtime 38.2 KiB; Rustls handshake parsing 31.9 KiB; `[Unknown] sqlite3VdbeExec` 27.6 KiB; and HTML5ever tree-builder code 26.1 KiB. Individual symbol sizes are not estimates of the savings from removing a dependency.

## Named data and attribution gaps

The `nm` and `readelf` evidence provides direct named examples:

| Symbol | Bytes | Section/association | Interpretation |
|---|---:|---|---|
| `ecp_nistz256_precomputed` | 151,552 | `.rodata`, `ring` | P-256 precomputation table |
| `ring_core…k25519Precomp` | 24,576 | `.rodata`, `ring` | Curve precomputation table |
| `unicode_linebreak::BREAK_PROP_TRIE_DATA` | 12,996 | `.rodata`, Unicode line breaking | Named Unicode property data |
| `unicode_linebreak::BREAK_PROP_TRIE_INDEX` | 5,688 | `.rodata`, Unicode line breaking | Index for that property table |
| `html_escape…NAMED_ENTITIES` | 68,000 | `.data.rel.ro`, HTML entity table | Named entity mapping data |
| `sqlite3VdbeExec` | 28,311 | `.text`, SQLite C symbol; `cargo-bloat` labels it `[Unknown]` | SQLite virtual-machine execution code |

Using diagnostic symbol address ranges clipped to the matching `readelf` section and unioned to avoid double-counting overlaps, named `r/R` ranges cover 277,489 of 1,704,613 `.rodata` bytes; named `d/D` ranges cover 82,376 of 648,408 `.data.rel.ro` bytes. This is **symbol-name coverage**, not crate ownership. Most read-only and relocated-read-only bytes have no named data symbol in the captured output. `cargo-bloat` does not attribute these data sections, so this run cannot honestly assign the remaining roughly 1.43 MB of `.rodata` or 566 KB of `.data.rel.ro` to crates.

`[Unknown]` includes visible SQLite functions (`sqlite3VdbeExec`, `sqlite3Select`, `sqlite3Pragma`, `sqlite3Parser`) but must not be equated with SQLite as a whole. The diagnostic `nm` output has 1,074 `sqlite3*` text-symbol rows; the union of their named ranges inside `.text` is about 354 KB. That is an address-range observation, not a complete SQLite package measurement. `cargo-bloat` attributes only 42.5 KiB to `libsqlite3_sys`, demonstrating why C/native code can be under-accounted in its crate table.

## Ranked reduction hypotheses (not approved changes)

| Priority to investigate | Evidence and optimistic size bound | Main tradeoff / uncertainty |
|---|---|---|
| 1. Separate syntax/theme payload from the executable | `syntect` contributes 46.7 KiB of reported text; `crates/cli/Cargo.toml` explicitly enables `default-syntaxes`, `default-themes`, and `regex-fancy`. | Syntax/theme payload is not named/quantified in the section evidence. A separately packaged asset set could preserve functionality but does not reduce total distribution size; custom subsets could change language/theme compatibility. |
| 2. Isolate HTML conversion/web-fetch payload | `html_to_markdown_rs` contributes 290.1 KiB of reported text; the dependency is in tool extensions and backs HTML-to-Markdown conversion. | At most 290.1 KiB of *reported direct text* is identified, before shared dependencies/data. Removing it from the main executable would remove web-fetch conversion unless delivered as a separately available component. It cannot close the gap alone. |
| 3. Revisit bundled SQLite deployment | `rusqlite` is configured with `features = ["bundled"]`; named SQLite text ranges cover about 354 KB in the diagnostic `.text`. | System/dynamic SQLite could move code out of this executable, but introduces an OS library/runtime and cross-platform packaging/security contract. The 354 KB is not a guaranteed saving and is far below the gap. |
| 4. Audit regex/syntax-engine feature breadth | `regex_automata` 303.5 KiB + `aho_corasick` 114.3 KiB + `regex_syntax` 97.7 KiB + `syntect` 46.7 KiB = 562.2 KiB of reported text buckets. | This is only an arithmetic sum of approximate text attribution; shared code and unmeasured embedded data prevent treating it as a saving. Removing features can affect syntax highlighting, matching, and UI behavior. |
| 5. Audit TLS/crypto backend footprint | `rustls` is 245.3 KiB and `ring` 181.6 KiB of reported text; the two named `ring` tables above total 176,128 B. | TLS, certificate validation, and crypto remain required. Backend or algorithm changes need security and supported-platform validation; no compatible reduction was tested. |
| 6. Examine relocations/unwind metadata with linker-level evidence | Shipping `.rela.dyn` is 796,104 B, `.eh_frame` 884,744 B, `.eh_frame_hdr` 124,156 B, and `.data.rel.ro` 648,888 B. | These are structural sections, not simply removable padding. Fat LTO and `panic=abort` are already in the release profile; FOOTPRINT-01's no-unwind-tables diagnostic did not reduce the binary. No safe metadata saving is established. |

No single measured component closes the gap. Even the 562.2 KiB regex/syntax text-bucket sum is about 21% of it, before uncertainty and functionality loss. Multiple such subsystems might justify a separately packaged architecture, but this attribution does not quantify a safe combined saving, establish parity, or identify a route to `<6,700,000` B.

## LTO, inlining, and stripping limits

- The shipping build already uses fat LTO, `opt-level=z`, one codegen unit, `panic=abort`, and stripping. ThinLTO, LTO-off, size-opt, and codegen-unit variants from FOOTPRINT-01 did not reduce size; none is proposed here.
- LTO can inline across crates, specialize generics, internalize symbols, and merge identical code. A demangled symbol namespace or `cargo-bloat` crate row is evidence for investigation, not a byte-perfect ownership map.
- Stripping removes symbol/debug metadata; it does not remove the linked code, tables, relocations, or unwind records discussed here. The diagnostic file-size difference is dominated by debug/symbol data and is irrelevant to the inherited shipping-size gate.
- The existing `--no-default-features` control did not change the linked binary in FOOTPRINT-01. No feature was disabled in this investigation.

## Gate and review status

The inherited binary limit remains exclusive `<6,700,000` bytes. FineCore baseline remains 9,334,616 B; all inherited source-size, strict Clippy, and binary-size failures remain visible and unwaived. This report changes no product code, release profile, feature set, gate, benchmark, or release status. PR #117 remains Draft for review; no merge or release is authorized by these findings.
