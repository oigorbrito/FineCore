import assert from "node:assert/strict";
import test from "node:test";
import { CoreRuntime, MemoryJournal } from "../src/runtime.ts";

test("final response yields auditable events", async () => {
  const journal = new MemoryJournal();
  const engine = new CoreRuntime({ next: async () => ({kind:"final", text:"ok"}) }, [], journal);
  const result = await engine.run("hello");
  assert.equal(result.status, "completed");
  assert.deepEqual(result.history.map(e=>e.type), ["run_started","model_result","run_finished"]);
});

test("tools execute once and record intent before effect", async () => {
  const journal = new MemoryJournal();
  let turns = 0, invocations = 0;
  const model = { next: async () => ++turns === 1 ? {kind:"tools",calls:[{id:"a",name:"read",input:{}}]} : {kind:"final",text:"done"} };
  const tool = {name:"read",execute:async()=>{invocations++;assert.equal((await journal.read()).at(-1).type,"tool_intent");return "data"}};
  const result = await new CoreRuntime(model,[tool],journal).run("task");
  assert.equal(result.status,"completed");
  assert.equal(invocations,1);
  assert.equal(result.history.filter(x=>x.type==="tool_result").length,1);
});

test("unreconciled intent blocks replay", async () => {
  const journal=new MemoryJournal();
  await journal.append({type:"run_started",prompt:"p"});
  await journal.append({type:"tool_intent",call:{id:"x",name:"write",input:{}}});
  let calls=0;
  const engine=new CoreRuntime({next:async()=>{calls++;return {kind:"final",text:"bad"}}},[],journal);
  const result=await engine.run("p");
  assert.equal(result.status,"uncertain");
  assert.equal(calls,0);
});

test("tool exception never claims safe rollback",async()=>{
  const journal=new MemoryJournal();let n=0;
  const engine=new CoreRuntime({next:async()=>++n===1?{kind:"tools",calls:[{id:"x",name:"edit",input:{}}]}:{kind:"final",text:"no"}},[{name:"edit",execute:async()=>{throw Error("disconnect")}}],journal);
  const result=await engine.run("p");
  assert.equal(result.status,"uncertain");
  assert.equal(result.history.at(-1).type,"tool_intent");
});

test("limits and invalid calls fail closed",async()=>{
  const a=await new CoreRuntime({next:async()=>({kind:"tools",calls:[]})},[],new MemoryJournal(),1).run("p");
  assert.equal(a.reason,"STEP_LIMIT");
  const b=await new CoreRuntime({next:async()=>({kind:"tools",calls:[{id:"same",name:"x",input:null},{id:"same",name:"x",input:null}]})},[],new MemoryJournal()).run("p");
  assert.equal(b.reason,"INVALID_TOOL_CALL");
});

test("abort before start causes no persistent effect",async()=>{
  const c=new AbortController();c.abort();
  const j=new MemoryJournal();
  const r=await new CoreRuntime({next:async()=>{throw Error("must not run")}},[],j).run("p",c.signal);
  assert.equal(r.status,"cancelled");
  assert.deepEqual(await j.read(),[]);
});
