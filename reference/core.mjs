// Reference contract only. No upstream engine selected.
export class Core {
  constructor({model,tools={},authorize,journal,maxSteps=12,maxObservationBytes=8192}={}) {
    if(typeof model?.next!=="function") throw Error("MODEL_REQUIRED");
    if(typeof authorize!=="function") throw Error("POLICY_REQUIRED");
    if(!Number.isSafeInteger(maxSteps)||maxSteps<1||maxSteps>1000) throw Error("INVALID_STEPS");
    if(!Number.isSafeInteger(maxObservationBytes)||maxObservationBytes<1||maxObservationBytes>1048576) throw Error("INVALID_BYTES");
    Object.assign(this,{model,tools,authorize,journal,maxSteps,maxObservationBytes});
  }
  async run({task,signal,onEvent=()=>{}}={}) {
    if(typeof task!=="string"||!task.trim()) throw Error("TASK_REQUIRED");
    const history=[{role:"user",content:task}],events=[];
    const emit=async(type,data={})=>{
      const event={sequence:events.length+1,type,...data};
      if(this.journal) await this.journal.append(event);
      events.push(event);
      try{onEvent(event)}catch{}
    };
    const end=async(status)=>{await emit("run_end",{status});return {status,history,events,steps:events.filter(x=>x.type==="model_response").length}};
    await emit("run_start");
    for(let step=0;step<this.maxSteps;step++){
      if(signal?.aborted)return end("cancelled");
      let response;
      try{response=await this.model.next({history:structuredClone(history),signal})}
      catch(error){await emit("model_error",{message:String(error?.message||error)});return end(signal?.aborted?"cancelled":"model_failed")}
      if(signal?.aborted)return end("cancelled");
      if(!response||!["final","tool"].includes(response.kind)){await emit("invalid_model_response");return end("model_failed")}
      await emit("model_response",{step,kind:response.kind});
      if(response.kind==="final"){
        if(typeof response.text!=="string"){await emit("invalid_final");return end("model_failed")}
        history.push({role:"assistant",content:response.text});return end("completed");
      }
      if(typeof response.name!=="string"||!Object.hasOwn(this.tools,response.name)||typeof this.tools[response.name]!=="function"){
        await emit("tool_unknown",{name:String(response.name)});return end("tool_failed")
      }
      let allowed=false;
      try{allowed=await this.authorize({name:response.name,args:response.args,task})}catch{}
      if(signal?.aborted)return end("cancelled");
      if(allowed!==true){await emit("tool_denied",{name:response.name});return end("denied")}
      await emit("tool_intent",{name:response.name});
      let result;
      try{result=await this.tools[response.name](response.args,{signal})}
      catch(error){await emit("tool_error",{name:response.name,message:String(error?.message||error)});return end("tool_failed")}
      const serialized=typeof result==="string"?result:JSON.stringify(result);
      const bytes=Buffer.from(String(serialized),"utf8");
      const truncated=bytes.length>this.maxObservationBytes;
      const observation=truncated?bytes.subarray(0,this.maxObservationBytes).toString("utf8").replace(/\uFFFD$/,""):String(serialized);
      await emit("tool_result",{name:response.name,truncated});
      history.push({role:"assistant",tool:response.name,args:response.args});
      history.push({role:"tool",name:response.name,content:observation,truncated});
      if(signal?.aborted)return end("cancelled");
    }
    return end("step_limit");
  }
}
