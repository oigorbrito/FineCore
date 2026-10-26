//! Controlled paired trial: same fixture on FineCore integration and upstream #58 port.
//! This deliberately records an actual append-only file side effect before the model
//! fails; counting complete workflows alone would conceal replay risk.
#![allow(dead_code)]

#[path = "workflow_support/mod.rs"]
mod workflow_support;

use orca_harness_core::{
    Context, FnTool, Message, Model, ModelError, ModelResponse, Tool, ToolCall, ToolError, ToolSchema,
};
use orca_harness_tools::{SubagentManager, SubagentTool, WorkflowStore, WorkflowTool};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
enum Fault {
    Transport,
    Incomplete,
    Authentication,
}

#[derive(Clone)]
struct InjectedModel {
    fault: Fault,
    fail_completions: usize,
    completions: Arc<AtomicUsize>,
    generations: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Model for InjectedModel {
    async fn generate(
        &self,
        context: &Context,
        _schemas: &[ToolSchema],
    ) -> Result<ModelResponse, ModelError> {
        self.generations.fetch_add(1, Ordering::SeqCst);
        let completed_tool = context
            .messages()
            .iter()
            .any(|m| matches!(m, Message::Tool { .. }));
        if !completed_tool {
            return Ok(ModelResponse::tool_calls(vec![ToolCall {
                id: "write-effect".into(),
                name: "append_effect".into(),
                arguments: json!({}),
            }]));
        }
        let completed = self.completions.fetch_add(1, Ordering::SeqCst);
        if completed < self.fail_completions {
            return Err(match self.fault {
                Fault::Transport => ModelError::Request("connection reset after tool".into()),
                Fault::Incomplete => ModelError::IncompleteResponse {
                    message: "stream stopped after tool".into(),
                    usage: None,
                },
                Fault::Authentication => ModelError::Authentication("invalid key".into()),
            });
        }
        Ok(ModelResponse::final_text("completed"))
    }
}

fn new_effect_file() -> PathBuf {
    let seq = NEXT_FILE.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "finecore-upstream-effect-{}-{}.log",
        std::process::id(),
        seq
    ));
    assert!(!path.exists(), "unexpected prior side-effect file");
    path
}

async fn run_trial(label: &str, fault: Fault, failures: usize, retry: bool, trial: usize) {
    let file = new_effect_file();
    let effect_file = file.clone();
    let side_effect: Arc<dyn Tool> = Arc::new(FnTool::new(
        "append_effect",
        "Append an irreversible effect to a local file",
        json!({"type": "object"}),
        move |_input: Value, _ctx| {
            let file = effect_file.clone();
            async move {
                let mut handle = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .map_err(|error| ToolError::msg(error.to_string()))?;
                handle
                    .write_all(b"effect\n")
                    .map_err(|error| ToolError::msg(error.to_string()))?;
                Ok::<Value, ToolError>(json!({"ok": true}))
            }
        },
    ));
    let generations = Arc::new(AtomicUsize::new(0));
    let completions = Arc::new(AtomicUsize::new(0));
    let model = InjectedModel {
        fault,
        fail_completions: failures,
        generations: generations.clone(),
        completions: completions.clone(),
    };
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let subagent = Arc::new(
        SubagentTool::with_tools(model, Arc::new(move || vec![side_effect.clone()]))
            .background(SubagentManager::new(2), move |notification| {
                if notification.spawn.run.is_some() && notification.result.is_err() {
                    eprintln!("UPSTREAM_RAW_STAGE_ERROR {:?}", notification.result.as_ref().err());
                }
                let _ = tx.send(notification);
            }),
    );
    let tool = WorkflowTool::new(subagent, WorkflowStore::new()).unwrap();
    let begin = Instant::now();
    tool.call(
        json!({"action": "run", "graph": [{"id": "stage", "prompt": "exercise tool effect"}]}),
        &workflow_support::ctx(),
    )
    .await
    .unwrap();
    let (notification, _stages) = workflow_support::terminal(&mut rx).await;
    let elapsed_micros = begin.elapsed().as_micros();
    let success = notification.result.is_ok();
    let tool_effects = fs::read_to_string(&file)
        .expect("an actual tool effect must be recorded")
        .lines()
        .count();
    fs::remove_file(&file).unwrap();

    let stage_attempts = completions.load(Ordering::SeqCst);
    assert!((1..=2).contains(&stage_attempts), "{label}: unexpected number of runs");
    assert_eq!(tool_effects, stage_attempts, "{label}: effects don't match runs");
    assert_eq!(
        generations.load(Ordering::SeqCst),
        stage_attempts * 2,
        "{label}: model invocations don't match tool/reply turns"
    );
    let error_line = notification
        .result
        .as_ref()
        .err()
        .map(|error| error.lines().next().unwrap_or_default().to_string());
    println!(
        "UPSTREAM_EFFECT_JSON {}",
        json!({
            "arm": if retry { "retry" } else { "baseline" },
            "case": label,
            "phase": "after_tool",
            "trial": trial,
            "success": success,
            "error_line": error_line,
            "tool_effects": tool_effects,
            "model_generations": generations.load(Ordering::SeqCst),
            "stage_attempts": stage_attempts,
            "elapsed_micros": elapsed_micros,
        })
    );
}

/// Control arm: the model fails *before* any tool or side effect.
#[derive(Clone)]
struct PretoolModel {
    fault: Fault,
    failures: usize,
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Model for PretoolModel {
    async fn generate(
        &self,
        _context: &Context,
        _schemas: &[ToolSchema],
    ) -> Result<ModelResponse, ModelError> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst);
        if attempt < self.failures {
            return Err(match self.fault {
                Fault::Transport => ModelError::Request("connection reset before tool".into()),
                Fault::Incomplete => ModelError::IncompleteResponse {
                    message: "stream stopped before tool".into(),
                    usage: None,
                },
                Fault::Authentication => ModelError::Authentication("invalid key".into()),
            });
        }
        Ok(ModelResponse::final_text("completed without effects"))
    }
}

async fn run_pretool_trial(label: &str, fault: Fault, failures: usize, retry: bool, trial: usize) {
    let calls = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let subagent = Arc::new(
        SubagentTool::with_tools(
            PretoolModel {
                fault,
                failures,
                calls: calls.clone(),
            },
            Arc::new(|| Vec::<Arc<dyn Tool>>::new()),
        )
        .background(SubagentManager::new(2), move |notification| {
            if notification.spawn.run.is_some() && notification.result.is_err() {
                eprintln!("UPSTREAM_RAW_STAGE_ERROR {:?}", notification.result.as_ref().err());
            }
            let _ = tx.send(notification);
        }),
    );
    let tool = WorkflowTool::new(subagent, WorkflowStore::new()).unwrap();
    let start = Instant::now();
    tool.call(
        json!({"action": "run", "graph": [{"id": "stage", "prompt": "control stage"}]}),
        &workflow_support::ctx(),
    )
    .await
    .unwrap();
    let (notification, _stages) = workflow_support::terminal(&mut rx).await;
    let elapsed_micros = start.elapsed().as_micros();
    let stage_attempts = calls.load(Ordering::SeqCst);
    assert!((1..=2).contains(&stage_attempts), "{label}: unexpected runs");
    let error_line = notification
        .result
        .as_ref()
        .err()
        .map(|error| error.lines().next().unwrap_or_default().to_string());
    println!(
        "UPSTREAM_EFFECT_JSON {}",
        json!({
            "arm": if retry { "retry" } else { "baseline" },
            "case": label,
            "phase": "before_tool",
            "trial": trial,
            "success": notification.result.is_ok(),
            "error_line": error_line,
            "tool_effects": 0,
            "model_generations": stage_attempts,
            "stage_attempts": stage_attempts,
            "elapsed_micros": elapsed_micros,
        })
    );
}

#[tokio::test]
async fn compare_real_tool_side_effects_with_faults() {
    let retry = match std::env::var("EXPECT_RETRY").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        other => panic!("EXPECT_RETRY must be 0 or 1: {other:?}"),
    };
    for trial in 0..20 {
        run_trial("healthy", Fault::Transport, 0, retry, trial).await;
        run_trial("transport_once", Fault::Transport, 1, retry, trial).await;
        run_trial("transport_twice", Fault::Transport, 2, retry, trial).await;
        run_trial("incomplete_once", Fault::Incomplete, 1, retry, trial).await;
        run_trial("authentication", Fault::Authentication, 1, retry, trial).await;
        run_pretool_trial("pretool_transport_once", Fault::Transport, 1, retry, trial).await;
        run_pretool_trial("pretool_incomplete_once", Fault::Incomplete, 1, retry, trial).await;
        run_pretool_trial("pretool_authentication", Fault::Authentication, 1, retry, trial).await;
    }
}
