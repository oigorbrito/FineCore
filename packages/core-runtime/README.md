# CORE runtime foundation — experimental

A minimal TypeScript execution kernel, **not** a complete coding agent, donor transplant, benchmark victory or released API.

## Run

Node.js 22.10+:

```sh
cd packages/core-runtime
npm test
```

No external dependencies, model calls, shell, or writes to a user workspace. Models and tools are injected. Intent is journaled before a tool runs; uncertain effects **block replay** rather than being silently retried. MemoryJournal is volatile and intentionally insufficient for crash recovery.

## Current boundaries

- One-shot sessions; no resume after successful or failed previous run.
- Sequential tools only; unknown tools return a failed result.
- No shell, filesystem write, sandbox, provider adapter, streaming, steering, follow-up or concurrency.
- Exceptions after intent produce `uncertain`. Cancellation during a pending tool is also uncertain.
- A persistence adapter needs durability/ordering guarantees and explicit reconciliation before M3.
- JavaScript `node --experimental-strip-types` is a test runner convenience, not a production build strategy.

Do not mark M1–M5 accepted from these unit tests. Do not merge without review. Follow `CORE-PERFORMANCE-CONTRACT-001.md` and `docs/research/CORE-OPT-EXTERNAL-FIRST.md` in the repository root.
