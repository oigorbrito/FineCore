import type { Event, Journal, Model, RunResult, Tool, ToolCall, ToolResult } from "./contracts.ts";

export class MemoryJournal implements Journal {
  private readonly events: Event[] = [];
  async append(event: Event): Promise<void> { this.events.push(structuredClone(event)); }
  async read(): Promise<readonly Event[]> { return structuredClone(this.events); }
}

export class CoreRuntime {
  private readonly tools: Map<string, Tool>;
  constructor(
    private readonly model: Model,
    tools: readonly Tool[],
    private readonly journal: Journal,
    private readonly maxSteps = 24,
  ) {
    if (!Number.isSafeInteger(maxSteps) || maxSteps < 1) throw new Error("INVALID_STEP_LIMIT");
    this.tools = new Map();
    for (const tool of tools) {
      if (this.tools.has(tool.name)) throw new Error("DUPLICATE_TOOL");
      this.tools.set(tool.name, tool);
    }
  }

  async run(prompt: string, signal: AbortSignal = new AbortController().signal): Promise<RunResult> {
    let events: readonly Event[];
    try { events = await this.journal.read(); }
    catch { return { status: "uncertain", reason: "JOURNAL_READ_FAILED", history: [] }; }
    // A persisted intent without a corresponding outcome is an uncertain side effect.
    const pending = new Set<string>();
    for (const event of events) {
      if (event.type === "tool_intent") pending.add(event.call.id);
      if (event.type === "tool_result") pending.delete(event.result.id);
    }
    if (pending.size) return { status: "uncertain", reason: "UNRECONCILED_TOOL_INTENT", history: events };
    if (events.length) return { status: "failed", reason: "SESSION_ALREADY_USED", history: events };
    const add = async (event: Event) => this.journal.append(event);
    const snapshot = async (): Promise<readonly Event[]> => {
      try { return await this.journal.read(); } catch { return []; }
    };
    const usedIds = new Set<string>();
    const finish = async (status: "failed" | "cancelled", reason: string): Promise<RunResult> => {
      await add({ type: "run_failed", reason });
      return { status, reason, history: await this.journal.read() };
    };
    if (signal.aborted) return { status: "cancelled", reason: "ABORTED", history: events };
    try {
      await add({ type: "run_started", prompt });
      for (let step = 0; step < this.maxSteps; step++) {
        if (signal.aborted) return finish("cancelled", "ABORTED");
        const reply = await this.model.next({ prompt, history: await this.journal.read(), signal });
        if (signal.aborted) return finish("cancelled", "ABORTED");
        await add({ type: "model_result", kind: reply.kind });
        if (reply.kind === "final") {
          await add({ type: "run_finished", text: reply.text });
          return { status: "completed", answer: reply.text, history: await this.journal.read() };
        }
        const seen = new Set<string>();
        for (const call of reply.calls) {
          if (!call.id || seen.has(call.id) || usedIds.has(call.id) || !call.name) return finish("failed", "INVALID_TOOL_CALL");
          seen.add(call.id);
          usedIds.add(call.id);
        }
        // Execute serially; write each intent only immediately before its effect.
        for (const call of reply.calls) {
          if (signal.aborted) return { status: "uncertain", reason: "ABORTED_WITH_PENDING_INTENT", history: await this.journal.read() };
          await add({ type: "tool_intent", call });
          const tool = this.tools.get(call.name);
          let result: ToolResult;
          if (!tool) result = { id: call.id, name: call.name, ok: false, error: "UNKNOWN_TOOL" };
          else {
            try { result = { id: call.id, name: call.name, ok: true, value: await tool.execute(call.input, signal) }; }
            catch (err) {
              // An exception does not prove the tool had no external effect.
              return { status: "uncertain", reason: "TOOL_EFFECT_UNKNOWN", history: await this.journal.read() };
            }
          }
          await add({ type: "tool_result", result });
        }
      }
      return finish("failed", "STEP_LIMIT");
    } catch (_err) {
      return { status: "uncertain", reason: "MODEL_OR_JOURNAL_FAILURE", history: await snapshot() };
    }
  }
}
