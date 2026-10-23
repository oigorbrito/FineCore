import assert from "node:assert/strict";
import { Agent } from "@earendil-works/pi-agent-core";
import {createAssistantMessageEventStream} from "@earendil-works/pi-ai/compat";
import {Type} from "typebox";
const scenario=process.env.CORE_SCENARIO||"cancel";
const repeats=10, records=[];
for(let i=0;i<repeats;i++){
 let turns=0,executions=0,ended=0,abortObserved=false;
 const usage={input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}};
 let agent;
 const tools=[{name:"probe",label:"Probe",description:"No side effects",parameters:Type.Object({step:Type.Number()}),execute:async()=>{
   executions++;
   if(scenario==="cancel"||scenario==="cancel-repeat"){agent.abort();if(scenario==="cancel-repeat")agent.abort();abortObserved=agent.signal?.aborted===true;await new Promise(r=>setTimeout(r,1));}
   return {content:[{type:"text",text:"ok"}],details:{}};
 }}];
 agent=new Agent({initialState:{systemPrompt:"Test controlled stop",tools},finishTurn:()=>executions>=1?{action:"end"}:undefined,streamFn:()=>{
   const n=turns++,stream=createAssistantMessageEventStream();
   queueMicrotask(()=>{const msg={role:"assistant",api:"openai-responses",provider:"openai",model:"mock",usage,timestamp:Date.now(),stopReason:"toolUse",content:[{type:"toolCall",id:"call-"+n,name:"probe",arguments:{step:n}}]};stream.push({type:"done",reason:"toolUse",message:msg});});

   return stream;
 }});
 agent.subscribe(event=>{if(event.type==="agent_end")ended++;});
 await agent.prompt("Run probe until cancelled");await agent.waitForIdle();
 assert.equal(executions,1,"must execute exactly one tool");assert.equal(turns,1,"host stop after first tool turn");assert.equal(ended,1,"must settle exactly once");assert.equal(abortObserved,scenario!=="host-budget","abort signal must match scenario");
 records.push({candidate:"pi",scenario,iteration:i+1,turns,executions,ended,abortObserved,correct:true});
}
console.log(JSON.stringify({scenario,records}));
