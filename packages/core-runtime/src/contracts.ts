/**
 * Stable boundary for the independent CORE runtime. No provider, UI or donor imports.
 * Tool effects are NOT replay-safe unless explicitly reconciled by the host.
 */
export type ToolCall = { id: string; name: string; input: unknown };
export type ToolResult = { id: string; name: string; ok: boolean; value?: unknown; error?: string };
export type ModelReply =
  | { kind: "final"; text: string }
  | { kind: "tools"; calls: ToolCall[] };
export type ModelInput = { prompt: string; history: readonly Event[]; signal: AbortSignal };
export interface Model { next(input: ModelInput): Promise<ModelReply> }
export interface Tool {
  readonly name: string;
  execute(input: unknown, signal: AbortSignal): Promise<unknown>;
}
export type Event =
  | { type: "run_started"; prompt: string }
  | { type: "model_result"; kind: ModelReply["kind"] }
  | { type: "tool_intent"; call: ToolCall }
  | { type: "tool_result"; result: ToolResult }
  | { type: "run_finished"; text: string }
  | { type: "run_failed"; reason: string };
export interface Journal {
  append(event: Event): Promise<void>;
  read(): Promise<readonly Event[]>;
}
export type RunResult =
  | { status: "completed"; answer: string; history: readonly Event[] }
  | { status: "failed" | "cancelled" | "uncertain"; reason: string; history: readonly Event[] };
