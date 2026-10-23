import assert from "node:assert/strict";
import {performance} from "node:perf_hooks";
import {Agent} from "@earendil-works/pi-agent-core";
import {createAssistantMessageEventStream} from "@earendil-works/pi-ai/compat";
import {Type} from "typebox";
const fanout=Number(process.env.CORE_FANOUT||4), repetitions=Number(process.env.CORE_REPS||30);
const records=[];
const inject=process.env.CORE_INJECT_FAILURE==="1";
const usage={input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}};
for(let i=0;i<repetitions;i++){
 let contextBytes=0,contextMessages=0;let turns=0, executed=0; const seen=new Set(); let observedError=false;
 const tool={name:"fetch",label:"fetch",description:"Return a fixed value",parameters:Type.Object({source:Type.Number()}),execute:async(_id,args)=>{executed++; assert.ok(Number.isInteger(args.source)); assert.ok(args.source>=0 && args.source<fanout); assert.ok(!seen.has(args.source), 'duplicate source'); seen.add(args.source); if(inject && args.source===0) throw new Error("injected-tool-failure"); return {content:[{type:"text",text:"null"}],details:{}}}};
 const agent=new Agent({initialState:{systemPrompt:"You are a concurrent fetcher.",tools:[tool]},toolExecution:"parallel",streamFn:(_model,context)=>{
   const stream=createAssistantMessageEventStream();
   const round=turns++;
   contextBytes+=Buffer.byteLength(JSON.stringify(context.messages||[]),"utf8");
   contextMessages+=(context.messages||[]).length;
   if(inject && round===1) observedError=JSON.stringify(context).includes("injected-tool-failure");
   queueMicrotask(()=>{
     const msg={role:"assistant",api:"openai-responses",provider:"openai",model:"mock",usage,timestamp:Date.now(),stopReason:round===0?"toolUse":"stop",
       content:round===0?Array.from({length:fanout},(_,j)=>({type:"toolCall",id:`c${j}`,name:"fetch",arguments:{source:j}})):[{type:"text",text:"done"}]};
     stream.push({type:"done",reason:msg.stopReason,message:msg});
   });
   return stream;
 }});
 const start=performance.now();
 await agent.prompt(`fetch ${fanout} sources`);
 await agent.waitForIdle();
 const elapsed_ms=performance.now()-start;
 assert.equal(turns,2);assert.equal(executed,fanout); assert.equal(seen.size,fanout); if(inject) assert.ok(observedError,"tool failure not visible to next model call");
 assert.equal(agent.state.messages.filter(x=>x.role==="assistant").at(-1)?.stopReason,"stop");
 records.push({candidate:"pi",iteration:i+1,fanout,elapsed_ms,turns,executed,uniqueSources:seen.size,observedError,contextBytes,contextMessages,correct:true});
}
console.log(JSON.stringify({candidate:"pi",fanout,repetitions,records}));
