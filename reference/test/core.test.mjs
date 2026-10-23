import {test} from "node:test";
import assert from "node:assert/strict";
import {Core} from "./core.mjs";
const scripted=(...items)=>({next:async()=>items.shift()});
const yes=async()=>true;
test("tool cycle final response",async()=>{
 let count=0;
 const r=await new Core({model:scripted({kind:"tool",name:"edit",args:{}},{kind:"final",text:"done"}),tools:{edit:async()=>{count++;return "ok"}},authorize:yes}).run({task:"edit"});
 assert.equal(count,1);assert.equal(r.status,"completed");assert.deepEqual(r.events.map(e=>e.type),["run_start","model_response","tool_intent","tool_result","model_response","run_end"]);
});
test("authorization is fail-closed",async()=>{
 let called=false;const r=await new Core({model:scripted({kind:"tool",name:"edit"}),tools:{edit:async()=>{called=true}},authorize:async()=>undefined}).run({task:"edit"});
 assert.equal(r.status,"denied");assert.equal(called,false);
});
test("unknown tool denied",async()=>{
 const r=await new Core({model:scripted({kind:"tool",name:"shell"}),authorize:yes}).run({task:"execute"});
 assert.equal(r.status,"tool_failed");
});
test("abort before model",async()=>{
 const ac=new AbortController();ac.abort();let invoked=false;
 const r=await new Core({model:{next:async()=>{invoked=true}},authorize:yes}).run({task:"cancel",signal:ac.signal});
 assert.equal(r.status,"cancelled");assert.equal(invoked,false);
});
test("bounded step count",async()=>{
 const r=await new Core({model:{next:async()=>({kind:"tool",name:"echo"})},tools:{echo:async()=>""},authorize:yes,maxSteps:3}).run({task:"loop"});
 assert.equal(r.status,"step_limit");assert.equal(r.steps,3);
});
test("bounded observations",async()=>{
 let observed;
 const model={next:async({history})=>history.length===1?{kind:"tool",name:"echo"}:(observed=history.at(-1),{kind:"final",text:"done"})};
 const r=await new Core({model,tools:{echo:async()=>"123456"},authorize:yes,maxObservationBytes:4}).run({task:"test"});
 assert.equal(r.status,"completed");assert.equal(observed.content,"1234");assert.equal(observed.truncated,true);
});
test("tool errors are not success",async()=>{
 const r=await new Core({model:scripted({kind:"tool",name:"bad"}),tools:{bad:async()=>{throw Error("broken")}},authorize:yes}).run({task:"run"});
 assert.equal(r.status,"tool_failed");
});
test("journal failure stops effect",async()=>{
 let effects=0;
 await assert.rejects(new Core({model:scripted({kind:"tool",name:"edit"}),tools:{edit:async()=>effects++},authorize:yes,journal:{append:async e=>{if(e.type==="tool_intent")throw Error("disk unavailable")}}}).run({task:"edit"}),/disk unavailable/);
 assert.equal(effects,0);
});
test("invalid task and mandatory policy",async()=>{
 assert.throws(()=>new Core({model:scripted()}),/POLICY_REQUIRED/);
 await assert.rejects(new Core({model:scripted(),authorize:yes}).run({task:""}),/TASK_REQUIRED/);
});
test("cancel after effect records result",async()=>{
 const ac=new AbortController();let effects=0;
 const r=await new Core({model:scripted({kind:"tool",name:"edit"}),tools:{edit:async()=>{effects++;ac.abort();return "applied"}},authorize:yes}).run({task:"edit",signal:ac.signal});
 assert.equal(effects,1);assert.equal(r.status,"cancelled");assert.ok(r.events.some(e=>e.type==="tool_result"));
});
