# Node template dependency/security review

**Review snapshot:** 2026-10-26  
**Branch base:** `integration/finecore-orca-main-review` at `2f980015a733ec449ca1ea8be51e97da08220fb1`  
**Scope:** generated TypeScript MCP-plugin template and its scaffold assertions only. No Rust runtime dependency, `main`, PR #110/#111, or upstream Orca repository was changed. No model/provider calls were made.

## Upstream Orca PRs #66–#72

All seven PRs were open against Orca `main` when reviewed. Their reviewed heads were: #66 `8115d4911422388aff5b8d6a06ddcf1f47be4d8f`, #67 `dec650564af29d11e87baa5e2553f408473ede71`, #68 `4f1e3571129700d2c87d07068cdffb1b38b3cf50`, #69 `f501a20ef0f425c93a8a2d9b0ff28845cad1f8a3`, #70 `d8148425fa47c8f3e8286f6e66f98f8ec5d15b3d`, #71 `a27474d18236132cc90b3f0d39dea79fed33a0c6`, and #72 `9de77fefd5b86ab15744f17a2345cfb9bc9d3f51`. They modify only the template lockfile, plus `package.json` in #67 and #72; no application source. The template `package.json` and lockfile at the reviewed PR base have the same SHA-256 as the files in this FineCore integration base (`6d971ff3…` and `437650f1…` respectively), so each patch was assessed against the imported template rather than a stale or differently edited copy.

| PR | Upstream proposal | Assessment and FineCore result |
|---|---|---|
| [#66](https://github.com/okikorg/orca-harness/pull/66) | `source-map-js` 1.2.1 → 1.2.2 | Adopted. Fixes indexed source-map denial of service (GHSA-68fv-2mgg-jv7q / CVE-2026-93749). It is in the build/test dependency graph, not the example server's direct imports. |
| [#67](https://github.com/okikorg/orca-harness/pull/67) | MCP SDK 1.30.0 → 1.31.0 | The direction is correct, but 1.31.0 is still below the npm advisory fix threshold 1.32.0 for several SDK issues. FineCore uses `^1.32.0`, lock resolves 1.32.2. This covers the SDK advisories including GHSA-22jm-h49p-29qw, GHSA-6prh-2h8m-c8cw, GHSA-46jr-4wx8-v453, and GHSA-rvq5-wwqv-78pq; the earlier credential issue GHSA-6qxp-vccf-f47h is also fixed. |
| [#68](https://github.com/okikorg/orca-harness/pull/68) | `fast-uri` 3.1.5 → 3.1.9 | Adopted; lock resolves 3.1.9. Fixes URI normalization/host-confusion and SSRF advisories (including GHSA-5jgf-p345-68v8, GHSA-f65p-4m7j-42xc, GHSA-fph4-wmhf-6fwf, GHSA-jqff-g426-hqxp, and later 3.1.9 advisories). |
| [#69](https://github.com/okikorg/orca-harness/pull/69) | `proxy-addr` 2.0.7 → 2.0.8 | Adopted; lock resolves 2.0.8. Fixes critical trusted-proxy IP spoofing (GHSA-jqcg-44mw-7w3h / CVE-2026-90711). |
| [#70](https://github.com/okikorg/orca-harness/pull/70) | `ip-address` 10.5.0 → 10.7.4 | Adopted; npm's current compatible resolution is 10.7.5. Addresses IPv6 classification, subnet-comparison, and parsing denial-of-service advisories, including GHSA-rpw4-54j3-4h4q, GHSA-j6r3-76f7-8jcv, GHSA-h3mg-xc3c-68pw, and GHSA-rxp7-6hxx-x8q5. |
| [#71](https://github.com/okikorg/orca-harness/pull/71) | Hono 4.13.2 → 4.13.13 | Adopted; lock resolves 4.13.14. Includes the `serveStatic` path-decoding fix (GHSA-5r4p-p66f-jhc7) and earlier Hono fixes. The generated example uses stdio, but the SDK dependency graph also contains HTTP transports. |
| [#72](https://github.com/okikorg/orca-harness/pull/72) | Vitest 2.1.9 → 5.0.5; remove `tinypool` | Security intent adopted, version not copied verbatim. Vitest 5.0.5 requires Node `^22.12.0 || ^24.0.0 || >=26.0.0` and its lock resolves Vite 8, whose engine excludes Node 20 below 20.19. That conflicts with this template's declared Node `>=20`. FineCore instead pins Vitest 4.1.12 and Vite 6.4.4, which retain support for maintained Node 20/22+ LTS lines and clear the current audit. The brittle scaffold test was updated to assert the direct locked packages and their engines rather than removed Vitest/Vite nested paths. |

The remaining compatible audit fixes are also included: Zod is declared `^4.6.6` (locked 4.6.7; fixes GHSA-mrhx-c7xc-m3w5), and the lockfile refresh updates vulnerable transitives such as `body-parser` 2.3.0→2.4.0, `qs` 6.15.3→6.16.1, PostCSS 8.5.6→8.5.31, and Rollup 4.45.1→4.65.0. Old nested Vite/esbuild dependencies and `tinypool` are removed by the selected Vitest graph. The lockfile now records dev-only packages as dev-only, so production-only audit results do not misclassify build/test tooling.

**Engine contract:** Vitest 4.1.12 declares `^20.0.0 || ^22.0.0 || >=24.0.0`; Vite 6.4.4 declares `^18.0.0 || ^20.0.0 || >=22.0.0`. Intersecting these with the template's prior Node `>=20` policy yields the smallest union `^20.0.0 || ^22.0.0 || >=24.0.0`: Node 20.x, 22.x, and 24+. This intentionally excludes Node 21/23, which the test toolchain does not declare. The same expression is now in `package.json`, the lockfile root, the scaffold assertions, and the generated README. A transitive, optional Rollup platform binary has a narrower engine range and is skipped where incompatible; strict installs and the full test/build/audit suite are validated on the matrix below.

**Technical compatibility is not security support.** The matrix demonstrates that this template can run on Node 20. The official [Node.js release schedule](https://github.com/nodejs/Release/blob/main/schedule.json) lists v20's end-of-life date as **2026-04-30**, not March 2026 as stated in the request. As of this review (2026-10-26), Node 20 is EOL and receives no official security fixes; its continued presence in the engine range is a legacy compatibility allowance only. Prefer supported Node 22, 24, or 26 for production.

## Audit evidence

The baseline and fixed JSON reports are committed under [`npm-audit/`](npm-audit/). They were collected with Node v26.10.0 and npm 11.19.1 on 2026-10-26.

- Before: `npm audit` reported **17 total** (3 critical, 9 high, 5 moderate). `npm audit --omit=dev` reported 11. The original lock omitted `dev: true` classification for development-only transitive packages; this explains why some tooling findings appeared in the production-only result.
- After: both full and production-only `npm audit` report **0 vulnerabilities**.
- Main baseline findings included critical `proxy-addr`/`tinypool`/Vitest issues; high SDK, `fast-uri`, `body-parser`, `esbuild`, `postcss`, `Rollup`, `source-map-js`, Vite, and Zod issues; and Hono/IP-address/`qs` advisories. Raw npm output retains the advisory IDs, affected ranges, and fix availability.

## Validation

A real scaffold was generated by the branch's FineCore CLI into a disposable directory. No project-specific code was substituted beyond the CLI's normal name rendering.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test -p finecore --test plugin_cli` on Windows | 4/6 pass; the scaffold/lockfile test passes. Two unrelated existing Windows fixture/path tests fail (details below). |
| Same `plugin_cli` suite at the unchanged parent commit `2f980015…` | The same 4/6 pass and the same two failures, reproduced verbatim; therefore not regressions from this change. |
| Hosted CI, manually dispatched run [40269413752](https://github.com/oigorbrito/finecore/actions/runs/40269413752) | `fmt` passed; workspace tests, benchmark reporting tests, and bench smoke passed. Source-size failed on 15 inherited files; Clippy was skipped after that failure. Release binary size failed at 9,313,752 bytes (limit `<6,700,000`). Overall workflow: FAIL. |
| Generated scaffold `npm ci` | PASS; npm emitted its install-script allow-list advisory for esbuild, but installation completed. |
| Generated scaffold `npm test` | PASS, 1 test / 1 passed (Vitest 4.1.12). |
| Generated scaffold `npm run build` | PASS; produced `dist/server.mjs`. |
| Generated scaffold `npx tsc --noEmit` | PASS. |
| `finecore plugin validate .` and `finecore plugin test .` | PASS; stdio `echo` tool was discovered and handshaken. |
| Full and production-only `npm audit` on generated scaffold | PASS; 0 vulnerabilities in both. |

The two baseline-reproduced Windows failures are: (1) shell fixture paths are mangled when a hook script is launched through `sh`, causing the two fixture MCP servers to close; and (2) an uninstall test's path-string assertion differs on Windows. They are present at the parent commit and are outside this dependency/template change. Existing unrelated Rust unused/dead-code warnings also remain. The normal `pull_request` CI trigger is restricted to base `main`, so this PR targeting the integration branch had no automatic PR checks; the workflow was manually dispatched on this head (run 40269413752). Its source-size and release-binary-size failures match the previously recorded inherited migration gates; Clippy was skipped after source-size failed. The passing workspace test job is Linux CI evidence, distinct from the reproduced Windows fixture failures above.

## NODE-CONTRACT-112 follow-up

This follow-up audits original PR #112 head `080ddb9b0ddd8d00ffb1abc743d024fb08fd45bb`. The compatibility matrix run [40272555471](https://github.com/oigorbrito/finecore/actions/runs/40272555471) was green on derivative commit `364542b64ab1f89bda85f2410a169426a171b7a9`, but it predated the corrected root engine declaration. It is useful as a dependency-graph comparison, not the final proof for this contract. The run's exact tool versions were:

| Runner | Node | npm | Strict install/test/build/TypeScript/full+prod audit |
|---|---:|---:|---|
| Ubuntu 24.04 | 20.20.2 | 10.8.2 | PASS |
| Ubuntu 24.04 | 22.23.4 | 10.9.9 | PASS |
| Ubuntu 24.04 | 24.22.1 | 11.19.1 | PASS |
| Ubuntu 24.04 | 26.12.1 | 11.20.0 | PASS |
| Windows 2022 | 22.23.4 | 10.9.9 | PASS |

That derivative added only the matrix workflow; the dependency graph was identical to the original PR head. This follow-up changes the root `engines.node` in both manifest and lock to `^20.0.0 || ^22.0.0 || >=24.0.0`, updates the generated README and scaffold assertions, and adds `.github/workflows/finecore-node-template-compat-112.yml` so the exact contract is checked on this PR for those five runner/Node combinations. The workflow runs `npm ci --engine-strict`, Vitest, build, TypeScript, and both npm audits. It verifies generated manifest/lock parity before installing.

After this change, the scaffold integration test and `cargo fmt --all -- --check` pass locally. A fresh CLI-generated Node 26.10.0 scaffold passed strict install, Vitest (1/1), build, TypeScript, `finecore plugin validate`, `finecore plugin test`, and full/production npm audits (both zero). The updated audit JSON is retained in `npm-audit/contract-node26-*.json`. The five-runner matrix for the corrected engine contract is recorded in the hosted workflow linked from PR #112; results are to be read from that run, not inferred from the earlier 40272555471 run.

The dependency lock graph and resolved package versions are unchanged from original PR head `080ddb9`; no runtime or security behavior changed. The only shipped contract change narrows the declared Node range to versions admitted by the required test tools. Node 20 remains technically supported by the template and passed the strict matrix, but it is EOL and this is not a security-support claim. The official schedule records 2026-04-30; the request's March date does not match that source.

## Review boundary

This change is ready for review as a Draft dependency/security update, not as an assertion that all repository gates pass. It changes only the TypeScript template manifest/lockfile, generated README, scaffold assertions, a focused Node compatibility workflow, and this evidence. It does not fix inherited Rust gates, release anything, merge anything, or alter PR #110/#111 or upstream Orca.
