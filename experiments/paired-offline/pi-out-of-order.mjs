import assert from "node:assert/strict";
import {performance} from "node:perf_hooks";
import {Agent} from "@earendil-works/pi-agent-core";
import {createAssistantMessageEventStream} from "@earendil-works/pi-ai/compat";
import {Type} from "typebox";
const fanout=Number(process.env.CORE_FANOUT||4), repetitions=Number(process.env.CORE_REPS||30);
const records=[];
const delayMode=process.env.CORE_DELAY_MODE||"reverse";
const delayFor=(j)=>delayMode==="reverse"?(fanout-j)*3:delayMode==="alternating"?(j%2===0?50:1):j===7?70:1;
const inject=true;
const failureIndex=Number(process.env.CORE_FAILURE_INDEX||0);
assert.ok(Number.isInteger(failureIndex)&&failureIndex>=0&&failureIndex<fanout);
const usage={input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}};
for(let i=0;i<repetitions;i++){
 let turns=0, executed=0; const seen=new Set(); const completion=[]; let observedError=false;
 const tool={name:"fetch",label:"fetch",description:"Return a fixed value",parameters:Type.Object({source:Type.Number()}),execute:async(_id,args)=>{executed++; assert.ok(Number.isInteger(args.source)); assert.ok(args.source>=0 && args.source<fanout); assert.ok(!seen.has(args.source), 'duplicate source'); seen.add(args.source); await new Promise(r=>setTimeout(r,delayFor(args.source)));completion.push(args.source); if(inject && args.source===failureIndex) throw new Error("injected-tool-failure"); return {content:[{type:"text",text:"null"}],details:{}}}};
 const agent=new Agent({initialState:{systemPrompt:"You are a concurrent fetcher.",tools:[tool]},toolExecution:"parallel",streamFn:(_model,context)=>{
   const stream=createAssistantMessageEventStream();
   const round=turns++;
   if(inject && round===1){
    const results=context.messages.filter(m=>m.role==="toolResult");
    assert.equal(results.length,fanout,"tool results count mismatch");
    assert.deepEqual(results.map(x=>x.toolCallId),Array.from({length:fanout},(_,j)=>`c${j}`),"result ordering changed");
    for(let j=0;j<fanout;j++){
      const r=results.find(x=>x.toolCallId===`c${j}`);
      assert.ok(r,`missing result c${j}`);
      assert.equal(r.isError,j===failureIndex,`wrong error flag c${j}`);
    }
    observedError=results.some(x=>x.toolCallId===`c${failureIndex}`&&x.isError===true&&JSON.stringify(x).includes("injected-tool-failure"));
   }
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
 assert.equal(turns,2);assert.equal(executed,fanout); assert.equal(seen.size,fanout);assert.notDeepEqual(completion,Array.from({length:fanout},(_,j)=>j),"tool completion did not reorder"); if(inject) assert.ok(observedError,"tool failure not visible to next model call");
 assert.equal(agent.state.messages.filter(x=>x.role==="assistant").at(-1)?.stopReason,"stop");
 records.push({candidate:"pi",iteration:i+1,fanout,elapsed_ms,turns,executed,uniqueSources:seen.size,failureIndex,delayMode,observedError,correct:true});
}
console.log(JSON.stringify({candidate:"pi",fanout,repetitions,records}));
