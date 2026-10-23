use std::sync::{Arc,Mutex,atomic::{AtomicUsize,Ordering}};
use std::collections::HashSet;
use std::time::Instant;
use serde_json::{json,Value};
use orca_harness_core::testing::{ScriptedModel,call};
use orca_harness_core::{Agent,FnTool,Limits};
use orca_harness_core::{ToolError,Message};

#[tokio::main]
async fn main(){
 let fanout:usize=std::env::var("CORE_FANOUT").unwrap_or_else(|_|"4".into()).parse().unwrap();
 let reps:usize=std::env::var("CORE_REPS").unwrap_or_else(|_|"30".into()).parse().unwrap();
 let mut rows=Vec::new();
 let inject=true;
 let delay_mode=std::env::var("CORE_DELAY_MODE").unwrap_or_else(|_|"reverse".into());
 let failure_index:usize=std::env::var("CORE_FAILURE_INDEX").unwrap_or_else(|_|"0".into()).parse().expect("failure index");
 assert!(failure_index<fanout);
 for i in 0..reps {
  let count=Arc::new(AtomicUsize::new(0));
  let counter=count.clone();
  let seen=Arc::new(Mutex::new(HashSet::<usize>::new()));
  let recorded=seen.clone();
  let completed=Arc::new(Mutex::new(Vec::<usize>::new()));
  let finished=completed.clone();
  let selected_mode=delay_mode.clone();
  let tool=FnTool::new("fetch","Return a fixed value",json!({"type":"object","properties":{"source":{"type":"number"}},"required":["source"]}),
   move |input,_ctx|{let c=counter.clone();let recorded=recorded.clone();let finished=finished.clone();let mode=selected_mode.clone();async move {
    let source=input.get("source").and_then(Value::as_u64).expect("source must be integer") as usize;
    assert!(source<fanout,"out of range source");
    assert!(recorded.lock().unwrap().insert(source),"duplicate source");
    c.fetch_add(1,Ordering::SeqCst);
    let wait=if mode=="reverse"{(fanout-source)*3}else if mode=="alternating"{if source%2==0{50}else{1}}else{if source==7{70}else{1}};
    tokio::time::sleep(std::time::Duration::from_millis(wait as u64)).await;
    finished.lock().unwrap().push(source);
    if inject && source==failure_index {return Err(ToolError::msg("injected-tool-failure"));}
    Ok(Value::Null)
   }});
  let calls=(0..fanout).map(|j|call(&format!("c{j}"),"fetch",json!({"source":j}))).collect();
  let model=Arc::new(ScriptedModel::tool_round(calls,"done"));
  let agent=Agent::new(model.clone())
    .system_prompt("You are a concurrent fetcher.")
    .tool(tool)
    .limits(Limits {max_parallel_tools:64,..Limits::default()});
  let start=Instant::now();
  let result=agent.run(format!("fetch {fanout} sources").as_str()).await.expect("offline loop failed");
  let elapsed_ms=start.elapsed().as_secs_f64()*1000.0;
  assert_eq!(count.load(Ordering::SeqCst),fanout,"tool executions mismatch");
  let _=result;
  let observed_error=if inject {
   let contexts=model.observed_contexts();
   let tool_results:Vec<_>=contexts.iter().flat_map(|ctx|ctx.messages().iter()).filter_map(|m|if let Message::Tool{results}=m{Some(results)}else{None}).flatten().collect();
   assert_eq!(tool_results.len(),fanout,"tool result count mismatch");
   assert_eq!(tool_results.iter().map(|r|r.call_id.clone()).collect::<Vec<_>>(),(0..fanout).map(|j|format!("c{j}")).collect::<Vec<_>>(),"result ordering changed");
   for j in 0..fanout {let id=format!("c{j}");let r=tool_results.iter().find(|r|r.call_id==id).expect("missing call result");assert_eq!(r.is_error,j==failure_index,"wrong error position");}
   
   model.observed_contexts().iter().any(|ctx|ctx.messages().iter().any(|m|matches!(m,Message::Tool{results} if results.iter().any(|r|r.is_error && r.output.to_string().contains("injected-tool-failure")))))
  }else{false};
  if inject {assert!(observed_error,"tool failure not visible to next model call");}
  let unique=seen.lock().unwrap().len();
  assert_eq!(unique,fanout,"unique source coverage mismatch");
  assert_ne!(*completed.lock().unwrap(),(0..fanout).collect::<Vec<_>>(),"completion order did not change");
  rows.push(json!({"candidate":"orca","iteration":i+1,"fanout":fanout,"elapsed_ms":elapsed_ms,"turns":2,"executed":fanout,"uniqueSources":unique,"failureIndex":failure_index,"delayMode":delay_mode,"observedError":observed_error,"correct":true}));
 }
 println!("{}",json!({"candidate":"orca","fanout":fanout,"repetitions":reps,"records":rows}));
}
