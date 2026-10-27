# FineCore — Pi upstream evolution and evidence-first performance program

**Source:** [earendil-works/pi](https://github.com/earendil-works/pi) (canonical); **destination:** [oigorbrito/finecore](https://github.com/oigorbrito/finecore). **Established:** 2026-10-26. No automatic code import or merge.

## Activation and scope

- This tracker is intentionally separate from the existing Orca tracker at `17 */6 * * *` UTC. Pi is sampled at `37 */6 * * *` UTC, with manual dispatch.
- Pi has **7,488 issues and 3,546 PRs** in its historical corpus as observed on 2026-10-26. The Orca tracker is not directly usable: its 30-page/3,000-record safety cap would fail and bulk mirroring would pollute the downstream issue tracker.
- Pi monitoring samples *all currently open PRs*, PRs and issues from a **14-day GitHub Search window**, the default-branch HEAD, and releases. Search completeness is explicitly checked and fails closed when GitHub returns `incomplete_results`, duplicates, a moving total count, or over **900 results per stream**. GitHub's ordinary PR list does not have reliable `updated_at` ordering; the tracker therefore never uses that order to terminate the open-PR inventory. The open-PR list contains exact head SHAs; recently updated closed PRs returned by Search have status/updated-time but no inferred head SHA. This is an intentionally **bounded incremental watch**, not a full historical clone: outages beyond the lookback require a separate backfill.
- Each run scans Pi using GET only, then records **at most one** deduplicated downstream issue with stable marker `finecore-pi-monitor` and digest, and a JSON Actions artifact (90-day retention) with all observed record identities, observed PR head SHAs where available, release and triage candidates.
- An unmodified snapshot is an idempotent no-op. An existing issue's downstream human notes *after* the monitor's managed marker survive an update. It does not close PRs, synchronize patches, update branches, create merges, open upstream issues or run external models.
- Only the `track` job (schedule/dispatch/push to main after manual merge) gets `issues:write`; validation on pull requests uses `contents:read` only. No application/PAT or upstream write permissions are requested.
- **GitHub scheduled workflows only operate from the default branch.** A passing feature-branch or Draft PR check does not imply the repository tracker is activated. Activation gate: manual review + merge into `main`, first successful track run with a FineCore issue marker, followed by replay showing `unchanged`. A separate ChatGPT recurring change-watch can operate before repository integration and should not be confused with GitHub Actions.
- If Actions are delayed/disabled or an API scan exceeds the page budget, report failure and preserve uncertainty. Never label an incomplete scan successful.

## What is tracked and what is *not* a gain

The watcher tags recent PR titles heuristically into `auth`, `model_provider`, `agent`, `performance`, `security` and `tui`. This triage is **not evidence that the PR is correct, relevant to FineCore or better-performing**. An upstream merged PR may be redundant with FineCode or incompatible with the Orca/Rust runtime. A skipped/completed CI check is not an agent-effectiveness benchmark.

On 2026-10-26, the source HEAD observed was `6c384aa30f619d7b28963f2bab24d8dacbe73cb6`, and FineCore `main` was `95e666924e672dba4749ab02b26d86782b0bf78a`. That FineCore `main` is a standalone independent tracker/kernel tree **without the full Orca integration** (still Draft #111). Thus **end-to-end FineCore vs Pi effectiveness cannot be demonstrated from the present `main`**.

### Benchmark campaign: required before claiming FineCore outperforms Pi

1. **Freeze revisions and intended product surfaces.** Pi canonical SHA, FineCore integrated candidate SHA, tool manifests, system prompts, provider transport, benchmark task revisions, dependencies and OS/container hashes. Explicitly name missing features or tools; do not treat feature absence as an efficiency improvement.
2. **Same model, same model version, equivalent inference routing and token accounting.** Match tool-availability policy, retry ceilings, context windows, safety permissions, temperature/seed where supported. Record provider and server-side cache settings. If the model is nondeterministic, repeat paired trials with randomized AB/BA order and a monitored idle resource gate. Isolate shared Ollama/GPU workloads.
3. **Use a task workload with verified answer quality.** Real repository change tasks, held-out coding problems, and repeatable tool-workflows with a separate, trusted test/human review oracle. Check solution correctness and regressions, not only agent stop state or shell command speed. Record tasks excluded and why.
4. **Primary metrics:** verified task success (paired discordances, confidence intervals), total tokens, USD equivalent per correct task (or measured local compute resource cost), wall-clock latency for correct solutions, elapsed busy compute time and model/tool call counts. Secondary: memory peak, CPU, tool failures, user interventions, p95 latency, overhead at equivalent quality. Include all failures, timeouts and censored trials.
5. **Statistics:** analyze paired per-task outcomes, block by task/model/runner, report uncertainty (e.g. bootstrap over tasks) and perform sensitivity analysis for outliers/failed runs. Never declare a win based on one model/task, microbenchmark, synthetic fault injection, or unadjusted wall times with resource contention.
6. **Adoption gate:** show a material improvement at non-inferior correctness, with justified migration cost and no new security, data-integrity or compatibility regressions. An improvement in Rust startup or CLI size is not by itself a win in end-to-end programming quality. Review PRs and merge manually.

### Current hypotheses (not experimental conclusions)

- Rust FineCore may lower native process overhead, dependency surface or idle resource use against the TypeScript Pi CLI, but external model inference and tool execution can dominate end-to-end latency.
- Pi's newer provider/OAuth/model support can be adopted via demonstrated compatibility. Existing FineCode already includes many identical source files, so avoid duplicating that work or assuming newer automatically means better.
- A lightweight core can outperform a larger agent on **specific workloads** if correctness is maintained; broad superiority is unproven without paired effectiveness data.
- Authentication fidelity is independent of speed. The staged Codex browser PR #122 has offline Windows/Linux contract validation, but does not yet constitute Pi-wide login parity.

## Acceptance criteria for the monitor PR

Offline regression tests verify JSON marker safety, failed pagination, open-old PR inclusion, malformed state rejection, upstream read-only enforcement, single issue creation and idempotent replay. Review the Actions artifact and target issue after deployment; observe two consecutive successful tracking runs. No automatic PR creation, patch cherry-picking, merge, release or cost-incurring API calls.

## Initial 2026-10-26 triage for human review

Pi [#11435](https://github.com/earendil-works/pi/pull/11435) (updated undici security dependency; merged) and [#11434](https://github.com/earendil-works/pi/pull/11434) (protocol-specific OpenCode endpoints; open) are **compatibility/security hypotheses**. Pi [#11440](https://github.com/earendil-works/pi/pull/11440) (tool-search ranking; merged) is a **potential functional quality** candidate; Pi [#11438](https://github.com/earendil-works/pi/pull/11438) (CI parallelization; merged) is a **build-efficiency hypothesis**, not an agent-runtime gain. Their patches, versions, cross-repository relevance and benefits require separate verification. Do not infer these apply to FineCore Rust or that performance improved merely from titles.


## First read-only GitHub Actions probe (2026-10-27 UTC)

- [Run 40427121607](https://github.com/oigorbrito/finecore/actions/runs/40427121607) completed the offline contract suite and live GET-only scan; **no FineCore issue was written**. The resulting Actions artifact is [finecore-pi-upstream-readonly-40427121607](https://github.com/oigorbrito/finecore/actions/runs/40427121607/artifacts/12700910620), ZIP SHA-256 `d7e6fa1f1467b711b28d937df898b80848a85c91d241ff74d948e5bef173650f`. It stores full collected identity lists and topic heuristics, not model usage or measured speed.
- Observed Pi HEAD `6c384aa30f619d7b28963f2bab24d8dacbe73cb6`; **89 open PRs**, **169 updated PRs**, **645 updated issues** in the bounded collection, **275 releases**. Counts vary during subsequent GitHub updates.
- The initial HTTP probe [40426630443](https://github.com/oigorbrito/finecore/actions/runs/40426630443) identified unreliable `updated_at` ordering in the ordinary GitHub listing. This was **a real integration finding**: the implementation switched its updated PR and issue inventories to the Search API with explicit total-count/completeness checks. Earlier failed runs are diagnostic evidence, not accepted inventories.
- Remaining gate before calling the GitHub monitor **active**: manual review/merge of this Draft into default branch, followed by a real non-dry-run tracker execution and a second idempotent replay. Until then, only the separate scheduled ChatGPT monitoring is activated.
