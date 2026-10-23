import {test} from "node:test";
import assert from "node:assert/strict";
import {CoreHost} from "../host.mjs";
import {inspectCapabilities} from "../contracts.mjs";
import {referenceEngine} from "../adapters/reference-engine.mjs";
import {memoryJournal} from "../adapters/memory-journal.mjs";

test("rejects missing engine and missing policy", () => {
  assert.throws(() => new CoreHost({}), /ENGINE_RUN_REQUIRED/);
  assert.throws(() => new CoreHost({engine:{run:async()=>{}}}), /POLICY_REQUIRED/);
});

test("runs without provider and journal captures event sequence", async () => {
  const journal=memoryJournal();
  const host=new CoreHost({engine:referenceEngine({model:{next:async()=>({kind:"final",text:"ok"})}}),authorize:async()=>false,journal});
  const result=await host.run({task:"hello"});
  assert.equal(result.status,"completed");
  assert.deepEqual(journal.snapshot().map(e=>e.type),["run_start","model_response","run_end"]);
  assert.deepEqual(journal.snapshot().map(e=>e.sequence),[1,2,3]);
});

test("does not claim capabilities absent in reference engine", () => {
  const host=new CoreHost({engine:referenceEngine({model:{next:async()=>({kind:"final",text:"ok"})}}),authorize:async()=>false});
  assert.deepEqual(host.capabilities,{run:true,steer:false,followUp:false,cancel:false,resume:false});
  assert.equal(inspectCapabilities({run:async()=>{}}).run,true);
});

test("rejects simultaneous runs of the same host", async () => {
  let release;
  const pending=new Promise(resolve=>{release=resolve});
  const host=new CoreHost({engine:{run:()=>pending},authorize:async()=>false});
  const first=host.run({task:"one"});
  await assert.rejects(host.run({task:"two"}),/RUN_ALREADY_ACTIVE/);
  release({status:"completed"});
  assert.equal((await first).status,"completed");
  assert.equal(host.busy,false);
});

test("releases lock even after engine failure", async () => {
  const host=new CoreHost({engine:{run:async()=>{throw Error("upstream failure")}},authorize:async()=>false});
  await assert.rejects(host.run({task:"one"}),/upstream failure/);
  assert.equal(host.busy,false);
});

test("policy remains fail-closed through adapter", async () => {
  let effects=0;
  const host=new CoreHost({
    engine:referenceEngine({model:{next:async()=>({kind:"tool",name:"edit"})},tools:{edit:async()=>effects++}}),
    authorize:async()=>undefined
  });
  assert.equal((await host.run({task:"try edit"})).status,"denied");
  assert.equal(effects,0);
});
