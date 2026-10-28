//! Graph sweep through the real `WorkflowTool` path with a deterministic model.
//!
//! Run in release mode. Output is CSV, one row per submitted run:
//! `cargo run -p orca-harness-tools --release --example bench_workflow_graph`
//!
//! The model answers after a fixed timer delay, so every stage costs about the
//! same and the run's critical path can be measured from the stages themselves.
//! What is left over is the engine: how long a stage waits between its last
//! dependency finishing and its own agent reaching the model, and whether the
//! run kept its promises — each stage exactly once, never before its
//! dependencies, with the upstream answers its template asked for.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use orca_harness_core::{
    CancellationToken, Context, Model, ModelError, ModelResponse, Tool, ToolContext, ToolSchema,
};
use orca_harness_tools::{SubagentManager, SubagentTool, WorkflowStore, WorkflowTool};
use serde_json::{json, Value};

/// What one stage's agent did, in microseconds from the run's own epoch.
#[derive(Clone, Copy)]
struct Visit {
    started: u64,
    finished: u64,
}

#[derive(Default)]
struct Observations {
    /// Every model invocation, keyed by the stage it was executing. A second
    /// entry for one key is a duplicate execution, which is a defect.
    visits: HashMap<String, Vec<Visit>>,
    /// Prompts that did not carry the upstream answers their template named.
    template_mismatches: usize,
    /// A map barrier whose joined output was not its children in input order.
    join_mismatches: usize,
}

#[derive(Clone)]
struct GraphModel {
    delay: Duration,
    epoch: Instant,
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    observations: Arc<Mutex<Observations>>,
}

impl GraphModel {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            epoch: Instant::now(),
            active: Arc::new(AtomicUsize::new(0)),
            peak: Arc::new(AtomicUsize::new(0)),
            observations: Arc::new(Mutex::new(Observations::default())),
        }
    }

    fn reset(&mut self) {
        self.epoch = Instant::now();
        self.peak.store(0, Ordering::Relaxed);
        *self.observations.lock().unwrap() = Observations::default();
    }

    fn micros(&self) -> u64 {
        self.epoch.elapsed().as_micros() as u64
    }
}

/// The answer a stage is contractually required to produce. Every check in
/// this probe is a comparison against this one function.
fn token(stage: &str) -> String {
    format!("{stage}#ok")
}

fn field<'a>(prompt: &'a str, name: &str) -> Option<&'a str> {
    prompt
        .split('|')
        .find_map(|part| part.strip_prefix(name))
        .map(str::trim)
}

#[async_trait]
impl Model for GraphModel {
    async fn generate(
        &self,
        context: &Context,
        _tools: &[ToolSchema],
    ) -> Result<ModelResponse, ModelError> {
        let prompt = context
            .messages()
            .iter()
            .rev()
            .find_map(|message| match message {
                orca_harness_core::Message::User { content, .. } => Some(content.clone()),
                _ => None,
            })
            .expect("a stage prompt");
        let started = self.micros();
        let active = self.active.fetch_add(1, Ordering::Relaxed) + 1;
        self.peak.fetch_max(active, Ordering::Relaxed);

        let declared = field(&prompt, "STAGE=").expect("STAGE= in every prompt");
        // A map child is one execution of its map stage; the item names which.
        let stage = match field(&prompt, "ITEM=") {
            Some(item) => format!("{declared}[{}]", item.trim_start_matches("item")),
            None => declared.to_string(),
        };

        // The template contract: every dependency named in the prompt must have
        // been substituted with that stage's answer, not left as a marker.
        let mut mismatches = 0;
        if let Some(expected) = field(&prompt, "NEEDS=") {
            for dep in expected.split(',').filter(|dep| !dep.is_empty()) {
                if !prompt.contains(&token(dep)) {
                    mismatches += 1;
                }
            }
        }
        // A map's joined output is its children's answers in input order.
        let mut join_mismatches = 0;
        if let Some(joined) = field(&prompt, "JOINED=") {
            let expected: Vec<String> = (0..field(&prompt, "ITEMS=")
                .and_then(|count| count.parse::<usize>().ok())
                .unwrap_or(0))
                .map(|index| token(&format!("fan[{index}]")))
                .collect();
            let actual: Vec<String> = serde_json::from_str(joined).unwrap_or_default();
            if actual != expected {
                join_mismatches += 1;
            }
        }

        tokio::time::sleep(self.delay).await;

        let finished = self.micros();
        self.active.fetch_sub(1, Ordering::Relaxed);
        {
            let mut observations = self.observations.lock().unwrap();
            observations
                .visits
                .entry(stage.clone())
                .or_default()
                .push(Visit { started, finished });
            observations.template_mismatches += mismatches;
            observations.join_mismatches += join_mismatches;
        }

        // A map source must answer with the array the map fans out over.
        if let Some(count) = field(&prompt, "EMIT_ITEMS=").and_then(|n| n.parse::<usize>().ok()) {
            let items: Vec<String> = (0..count).map(|index| format!("item{index}")).collect();
            return Ok(ModelResponse::final_text(
                serde_json::to_string(&items).expect("item array"),
            ));
        }
        Ok(ModelResponse::final_text(token(&stage)))
    }
}

include!("bench_workflow_graph/shapes.rs");

fn percentile(sorted: &[u64], percentile: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len().saturating_sub(1)) as f64 * percentile).round() as usize;
    sorted[index]
}

/// The longest chain of measured stage service times through the graph: what
/// the run would have cost with a scheduler that never hesitated. Measured
/// rather than assumed, because a timer sleeps for at least its delay, not
/// exactly it, and that rounding would otherwise be charged to the engine.
fn critical_path_us(shape: &Shape, visits: &HashMap<String, Vec<Visit>>) -> u64 {
    fn finish(
        id: &str,
        shape: &Shape,
        visits: &HashMap<String, Vec<Visit>>,
        memo: &mut HashMap<String, u64>,
    ) -> u64 {
        if let Some(known) = memo.get(id) {
            return *known;
        }
        // Guard against a cycle the engine would have rejected anyway.
        memo.insert(id.to_string(), 0);
        let service = visits
            .get(id)
            .and_then(|visits| visits.first())
            .map_or(0, |visit| visit.finished.saturating_sub(visit.started));
        let upstream = shape
            .needs
            .get(id)
            .into_iter()
            .flatten()
            .map(|dep| finish(dep, shape, visits, memo))
            .max()
            .unwrap_or(0);
        let total = upstream + service;
        memo.insert(id.to_string(), total);
        total
    }
    let mut memo = HashMap::new();
    shape
        .expected
        .iter()
        .map(|id| finish(id, shape, visits, &mut memo))
        .max()
        .unwrap_or(0)
}

struct Measurement {
    wall_us: u64,
    critical_path_us: u64,
    nominal_path_us: u64,
    delay_us: u64,
    limit: u32,
    service: Vec<u64>,
    dispatch: Vec<u64>,
    peak_observed: usize,
    peak_admitted: usize,
    peak_running: usize,
    ordering_violations: usize,
    duplicate_stages: usize,
    missing_stages: usize,
    template_mismatches: usize,
    failures: usize,
}

/// One submitted run, from the acknowledgement to the terminal notification.
async fn run_once(shape: &Shape, limit: u32, model: &mut GraphModel) -> Measurement {
    let delay_us = model.delay.as_micros() as u64;
    model.reset();
    let manager = SubagentManager::new(limit);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let subagent = Arc::new(
        SubagentTool::with_tools(model.clone(), Arc::new(Vec::new)).background(
            manager,
            move |notification| {
                let _ = tx.send(notification);
            },
        ),
    );
    let tool = WorkflowTool::new(subagent, WorkflowStore::new()).expect("depth-zero background");
    let context = ToolContext {
        call_id: "bench".into(),
        tool_name: "workflow".into(),
        cancellation: CancellationToken::new(),
        deadline: None,
    };

    let started = Instant::now();
    let submitted = model.micros();
    let acknowledgement = tool
        .call(
            json!({
                "action": "run",
                "graph": shape.stages,
                // Both the submitted count and everything a map expands into.
                "maxStages": shape.expected.len() * 2 + 16,
            }),
            &context,
        )
        .await;
    let mut failures = usize::from(acknowledgement.is_err());
    let mut outcome = Value::Null;
    if failures == 0 {
        loop {
            let notification = rx.recv().await.expect("the run must terminate");
            if notification.spawn.run.is_none() {
                outcome = match notification.result {
                    Ok(value) => value["workflow"].clone(),
                    Err(error) => {
                        failures += 1;
                        json!({"state": "failed", "error": error})
                    }
                };
                break;
            }
        }
    }
    let wall_us = started.elapsed().as_micros() as u64;

    let observations = model.observations.lock().unwrap();
    let mut dispatch = Vec::with_capacity(shape.expected.len());
    let mut service = Vec::with_capacity(shape.expected.len());
    let mut ordering_violations = 0;
    let mut duplicate_stages = 0;
    let mut missing_stages = 0;
    let expected: HashSet<&String> = shape.expected.iter().collect();
    for id in &shape.expected {
        let Some(visits) = observations.visits.get(id) else {
            missing_stages += 1;
            continue;
        };
        if visits.len() > 1 {
            duplicate_stages += visits.len() - 1;
        }
        let visit = visits[0];
        service.push(visit.finished.saturating_sub(visit.started));
        // Ready is the moment the last dependency's agent returned; anything
        // after that is the engine deciding to run this stage.
        let ready = shape.needs[id]
            .iter()
            .filter_map(|dep| observations.visits.get(dep))
            .filter_map(|visits| visits.first())
            .map(|dep| dep.finished)
            .max()
            .unwrap_or(submitted);
        if visit.started < ready {
            ordering_violations += 1;
        }
        dispatch.push(visit.started.saturating_sub(ready));
    }
    // An execution of something the graph never declared is also a defect.
    duplicate_stages += observations
        .visits
        .keys()
        .filter(|id| !expected.contains(id))
        .count();
    let template_mismatches = observations.template_mismatches + observations.join_mismatches;
    let peak_observed = model.peak.load(Ordering::Relaxed);
    let critical_path_us = critical_path_us(shape, &observations.visits);
    drop(observations);

    if outcome["state"].as_str() != Some("done") {
        failures += 1;
    }
    for id in &shape.terminal {
        if outcome["outputs"][id].as_str() != Some(token(id).as_str()) {
            failures += 1;
        }
    }
    // Admitted counts every stage the run had in flight; running counts the
    // ones holding a concurrency slot. They diverge exactly when a limit binds.
    let peak_admitted = outcome["peakAdmitted"].as_u64().unwrap_or(0) as usize;
    let peak_running = outcome["peakRunning"].as_u64().unwrap_or(0) as usize;

    dispatch.sort_unstable();
    service.sort_unstable();
    Measurement {
        wall_us,
        critical_path_us,
        nominal_path_us: shape.depth as u64 * delay_us,
        delay_us,
        limit,
        service,
        dispatch,
        peak_observed,
        peak_admitted,
        peak_running,
        ordering_violations,
        duplicate_stages,
        missing_stages,
        template_mismatches,
        failures,
    }
}

fn environment<T: std::str::FromStr>(name: &str, fallback: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let levels: Vec<usize> = std::env::var("LEVELS")
        .unwrap_or_else(|_| "8,32,128,512".into())
        .split(',')
        .map(|value| value.parse().expect("LEVELS must be integers"))
        .collect();
    let shapes: Vec<String> = std::env::var("SHAPES")
        .unwrap_or_else(|_| "chain,diamond,wide-join,fanout,mesh".into())
        .split(',')
        .map(str::to_string)
        .collect();
    let delay = Duration::from_millis(environment("DELAY_MS", 2u64));
    let repetitions: usize = environment("REPETITIONS", 3);
    let limit: u32 = environment("LIMIT", 0);
    let mut model = GraphModel::new(delay);

    eprintln!("warming up");
    let warmup = build("diamond", 8);
    run_once(&warmup, limit, &mut model).await;

    println!(
        "shape,stages,run,wall_us,critical_path_us,nominal_path_us,service_p50_us,\
         service_p95_us,delay_us,limit,dispatch_p50_us,dispatch_p95_us,dispatch_p99_us,peak_observed,\
         peak_admitted,peak_running,ordering_violations,duplicate_stages,missing_stages,\
         template_mismatches,failures"
    );
    for name in &shapes {
        for level in &levels {
            let shape = build(name, *level);
            let stages = shape.expected.len();
            for run in 0..repetitions {
                let measured = run_once(&shape, limit, &mut model).await;
                println!(
                    "{name},{stages},{run},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    measured.wall_us,
                    measured.critical_path_us,
                    measured.nominal_path_us,
                    percentile(&measured.service, 0.50),
                    percentile(&measured.service, 0.95),
                    measured.delay_us,
                    measured.limit,
                    percentile(&measured.dispatch, 0.50),
                    percentile(&measured.dispatch, 0.95),
                    percentile(&measured.dispatch, 0.99),
                    measured.peak_observed,
                    measured.peak_admitted,
                    measured.peak_running,
                    measured.ordering_violations,
                    measured.duplicate_stages,
                    measured.missing_stages,
                    measured.template_mismatches,
                    measured.failures,
                );
            }
        }
    }
}
