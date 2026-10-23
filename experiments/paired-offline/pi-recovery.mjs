import assert from "node:assert/strict";
import { Agent } from "@earendil-works/pi-agent-core";
import { createAssistantMessageEventStream } from "@earendil-works/pi-ai/compat";
import { Type } from "typebox";
const scenario=process.env.CORE_GATE_SCENARIO||"recovery";
const usage={input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}};
const records=[];
for(let run=0;run<10;run++){
 let turns=0, executions=0, sawFailure=false, sawSuccess=false;
 const tool={name:"probe",label:"Probe",description:"Return probe value",parameters:Type.Object({attempt:Type.Number()}),execute:async (_id,args)=>{
  executions++;if(scenario==="cancel"){agent.abort();await new Promise(r=>setTimeout(r,2));throw Error("cancelled tool");}
  if(args.attempt===1) throw Error("injected-transient");
  return {content:[{type:"text",text:"recovered"}],details:{}};
 }};
 const agent=new Agent({initialState:{systemPrompt:"Test.",tools:[tool]},toolExecution:"sequential",streamFn:(_model,context)=>{
  const stream=createAssistantMessageEventStream();const n=turns++;
  const snapshot=JSON.stringify(context);
  if(n===1)sawFailure=snapshot.includes("injected-transient");
  if(n===2)sawSuccess=snapshot.includes("recovered");
  queueMicrotask(()=>{
   const call=(id,attempt)=>({type:"toolCall",id,name:"probe",arguments:{attempt}});
   const final=n===2;
   const msg={role:"assistant",api:"openai-responses",provider:"openai",model:"mock",usage,timestamp:Date.now(),stopReason:final?"stop":"toolUse",content:final?[{type:"text",text:"done"}]:[call("try-"+n,n+1)]};
   stream.push({type:"done",reason:msg.stopReason,message:msg});
  });return stream;
 }});
 await agent.prompt("Use probe and recover.");
 await agent.waitForIdle();
 if(scenario==="recovery"){assert.equal(turns,3);assert.equal(executions,2);assert.ok(sawFailure);assert.ok(sawSuccess);assert.equal(agent.state.messages.filter(x=>x.role==="assistant").at(-1)?.stopReason,"stop");}
 else {assert.ok(executions>=1);assert.ok(turns<=2);}
 records.push({candidate:"pi",scenario,iteration:run+1,turns,executions,sawFailure,sawSuccess,correct:true});
}
console.log(JSON.stringify({candidate:"pi",scenario,records}));
