use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
use std::time::Duration;
use serde_json::{json,Value};
use orca_harness_core::{Agent,FnTool,Limits};
use orca_harness_core::testing::{ScriptedModel,call};
#[tokio::main]
async fn main(){
 let cap:usize=std::env::var("CORE_CONCURRENCY").unwrap().parse().unwrap();
 let reps:usize=10;let fanout=12usize;let mut records=Vec::new();
 for i in 0..reps{
  let active=Arc::new(AtomicUsize::new(0));let peak=Arc::new(AtomicUsize::new(0));let done=Arc::new(AtomicUsize::new(0));
  let aa=active.clone();let pp=peak.clone();let dd=done.clone();
  let tool=FnTool::new("probe","bounded concurrent probe",json!({"type":"object","properties":{"source":{"type":"number"}},"required":["source"]}),move |_input,_ctx|{
   let active=aa.clone();let peak=pp.clone();let done=dd.clone();async move{
    let now=active.fetch_add(1,Ordering::SeqCst)+1;peak.fetch_max(now,Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(12)).await;
    active.fetch_sub(1,Ordering::SeqCst);done.fetch_add(1,Ordering::SeqCst);Ok(Value::Null)
   }
  });
  let calls=(0..fanout).map(|j|call(&format!("c{j}"),"probe",json!({"source":j}))).collect();
  let agent=Agent::new(ScriptedModel::tool_round(calls,"done")).tool(tool).limits(Limits{max_parallel_tools:cap,..Limits::default()});
  assert_eq!(agent.run("probe all").await.unwrap(),"done");
  let peak=peak.load(Ordering::SeqCst);
  assert_eq!(done.load(Ordering::SeqCst),fanout);
  assert!(peak<=cap,"concurrency cap exceeded");
  assert_eq!(peak,cap.min(fanout),"cap not utilized");
  records.push(json!({"candidate":"orca","iteration":i+1,"cap":cap,"peak":peak,"executed":fanout,"correct":true}));
 }
 println!("{}",json!({"records":records}));
}
