import { assertEngine, assertPolicy, assertJournal, inspectCapabilities } from "./contracts.mjs";

export class CoreHost {
  #active = false;
  constructor({engine, authorize, journal} = {}) {
    this.engine = assertEngine(engine);
    this.authorize = assertPolicy(authorize);
    this.journal = assertJournal(journal);
  }
  get capabilities() { return inspectCapabilities(this.engine); }
  get busy() { return this.#active; }

  async run({task, signal, onEvent = () => {}} = {}) {
    if (this.#active) throw new Error("RUN_ALREADY_ACTIVE");
    if (typeof task !== "string" || task.trim() === "") throw new TypeError("TASK_REQUIRED");
    if (typeof onEvent !== "function") throw new TypeError("EVENT_HANDLER_REQUIRED");
    this.#active = true;
    try {
      return await this.engine.run({task, signal, authorize: this.authorize, journal: this.journal, onEvent});
    } finally {
      this.#active = false;
    }
  }
}
