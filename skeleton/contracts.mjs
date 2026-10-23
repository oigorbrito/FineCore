export function assertEngine(engine) {
  if (!engine || typeof engine.run !== "function") throw new TypeError("ENGINE_RUN_REQUIRED");
  return engine;
}

export function assertPolicy(authorize) {
  if (typeof authorize !== "function") throw new TypeError("POLICY_REQUIRED");
  return authorize;
}

export function assertJournal(journal) {
  if (journal !== undefined && (!journal || typeof journal.append !== "function")) {
    throw new TypeError("JOURNAL_APPEND_REQUIRED");
  }
  return journal;
}

// Capabilities must be measured separately; do not infer them from donor names.
export function inspectCapabilities(engine) {
  assertEngine(engine);
  return Object.freeze({
    run: true,
    steer: typeof engine.steer === "function",
    followUp: typeof engine.followUp === "function",
    cancel: typeof engine.cancel === "function",
    resume: typeof engine.resume === "function"
  });
}
