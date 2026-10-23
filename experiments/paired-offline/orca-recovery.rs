use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
use serde_json::{json,Value};
use orca_harness_core::{Agent,FnTool,ModelResponse,ToolError,Context,Message,Limits,CancellationToken,HarnessError};
use orca_harness_core::testing::{ScriptedModel,call};
#[tokio::main]
async fn main(){
 let scenario=std::env::var("CORE_GATE_SCENARIO").unwrap_or_else(|_|"recovery".into());
 let mut records=Vec::new();
 for i in 0..10 {
  let counter=Arc::new(AtomicUsize::new(0));
  let c=counter.clone();
  let cancel=CancellationToken::new();let tool_cancel=cancel.clone();let is_cancel=scenario=="cancel";
  let tool=FnTool::new("probe","Return probe value",json!({"type":"object","properties":{"attempt":{"type":"number"}},"required":["attempt"]}),move |args,_ctx|{
   let c=c.clone();let t=tool_cancel.clone();async move {
    c.fetch_add(1,Ordering::SeqCst);
    if is_cancel {t.cancel();tokio::time::sleep(std::time::Duration::from_millis(5)).await;return Ok(Value::Null);}
    if args["attempt"]==1 {Err(ToolError::msg("injected-transient"))}else{Ok(json!("recovered"))}
   }
  });
  let script=vec![
   ModelResponse::tool_calls(vec![call("try-0","probe",json!({"attempt":1}))]),
   ModelResponse::tool_calls(vec![call("try-1","probe",json!({"attempt":2}))]),
   ModelResponse::final_text("done"),
  ];
  let model=Arc::new(ScriptedModel::new(script));
  let agent=Agent::new(model.clone()).tool(tool).limits(Limits{max_steps:if scenario=="step-limit"{1}else{5},..Limits::default()});
  let mut ctx=Context::new();ctx.push_user("Use probe and recover.");
  let result=agent.run_context(&mut ctx,cancel).await;
  let executions=counter.load(Ordering::SeqCst);
  let observed=model.observed_contexts();
  let saw_failure=observed.iter().any(|ctx|ctx.messages().iter().any(|m|matches!(m,Message::Tool{results} if results.iter().any(|r|r.is_error && r.output.to_string().contains("injected-transient")))));
  let saw_success=observed.iter().any(|ctx|ctx.messages().iter().any(|m|matches!(m,Message::Tool{results} if results.iter().any(|r|!r.is_error && r.output.to_string().contains("recovered")))));
  if scenario=="recovery" {
    assert_eq!(result.unwrap(),"done");assert_eq!(executions,2);assert!(saw_failure);assert!(saw_success);
  }else if scenario=="step-limit"{
    assert!(matches!(result,Err(HarnessError::StepLimitExceeded)));assert_eq!(executions,1);
  }else{
    assert!(matches!(result,Err(HarnessError::Cancelled)));assert_eq!(executions,1);
  }
  records.push(json!({"candidate":"orca","scenario":scenario,"iteration":i+1,"turns":observed.len(),"executions":executions,"sawFailure":saw_failure,"sawSuccess":saw_success,"correct":true}));
 }
 println!("{}",json!({"candidate":"orca","scenario":scenario,"records":records}));
}
