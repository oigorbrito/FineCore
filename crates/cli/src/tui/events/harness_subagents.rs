fn record_subagent_event(
    app: &mut App,
    id: u64,
    parent_id: Option<u64>,
    depth: u32,
    call_id: &str,
    event: &HarnessEvent,
) {
    let previous_status = app.subagent_transcripts.get(&id).map(|t| t.status);
    // A sidekick can fail before its first retained report reaches the parent.
    // The initiating tool call is already in the parent activity, so it is the
    // earliest authoritative UI-side indication that this spawn is persistent.
    let initiated_persistent = app.activity_tools.iter().any(|tool| {
        tool.call_id == call_id
            && tool.tool_name == "subagent"
            && tool
                .input
                .get("persistent")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
    });
    let transcript = app.subagent_transcripts.entry(id).or_insert_with(|| {
        SubagentTranscript::new(
            id,
            parent_id,
            depth,
            call_id.to_string(),
            String::new(),
            None,
        )
    });
    transcript.persistent |= initiated_persistent;
    // Stop is terminal for the handle. Child events already in flight still
    // belong in the transcript, but must not resurrect or reclassify it.
    let stopped = transcript.status == SubagentTranscriptStatus::Stopped;
    // The first event out of the inner loop (`AgentStart`, normally) means
    // a queued background worker got its slot. Elapsed restarts so the row
    // times the run rather than the wait.
    if !stopped
        && (transcript.status == SubagentTranscriptStatus::Queued
            || transcript.status == SubagentTranscriptStatus::Idle
            || (transcript.sidekick_established
                && transcript.status == SubagentTranscriptStatus::Failed))
    {
        transcript.status = SubagentTranscriptStatus::Running;
        transcript.started = Instant::now();
    }
    match event {
        HarnessEvent::AssistantDelta { text } => {
            if transcript.streaming_assistant.is_empty() && !text.is_empty() {
                transcript.commit_settled_activity();
            }
            append_display_text(&mut transcript.streaming_assistant, text);
        }
        HarnessEvent::ReasoningDelta { text } => {
            if transcript.streaming_reasoning.is_empty() && !text.is_empty() {
                transcript.commit_settled_activity();
                transcript.reasoning_started = Some(Instant::now());
            }
            append_display_text(&mut transcript.streaming_reasoning, text);
        }
        HarnessEvent::Assistant { message } => {
            transcript.commit_activity();
            let streamed = std::mem::take(&mut transcript.streaming_assistant);
            transcript.push_assistant(if message.is_empty() {
                streamed
            } else {
                message.clone()
            });
        }
        HarnessEvent::ToolCall {
            tool_call_id,
            tool_name,
            input,
        } => {
            transcript.flush_reasoning();
            let streamed = std::mem::take(&mut transcript.streaming_assistant);
            transcript.push_assistant(streamed);
            let index = transcript.activity_tools.len();
            transcript.activity_tools.push(ToolActivity::new(
                tool_call_id.clone(),
                tool_name.clone(),
                display_value(input),
            ));
            transcript.pending_calls.insert(tool_call_id.clone(), index);
        }
        HarnessEvent::ToolStarted { tool_call_id, .. } => {
            if let Some(index) = transcript.pending_calls.get(tool_call_id).copied() {
                if let Some(tool) = transcript.activity_tools.get_mut(index) {
                    tool.execution_started();
                }
            }
        }
        HarnessEvent::ToolFinished {
            tool_call_id,
            is_error,
            ..
        } => {
            if let Some(index) = transcript.pending_calls.get(tool_call_id).copied() {
                if let Some(tool) = transcript.activity_tools.get_mut(index) {
                    tool.finish(*is_error);
                }
            }
        }
        HarnessEvent::ToolResult {
            tool_call_id,
            output,
            is_error,
            ..
        } => {
            if let Some(index) = transcript.pending_calls.remove(tool_call_id) {
                if let Some(tool) = transcript.activity_tools.get_mut(index) {
                    tool.record_result(display_value(output), *is_error);
                }
            }
        }
        HarnessEvent::Usage { usage } => {
            transcript.input_tokens = transcript.input_tokens.saturating_add(usage.input_tokens);
            transcript.output_tokens = transcript.output_tokens.saturating_add(usage.output_tokens);
            if let Some(tokens) = usage.reasoning_tokens {
                transcript.reasoning_tokens = Some(
                    transcript
                        .reasoning_tokens
                        .unwrap_or(0)
                        .saturating_add(tokens),
                );
            }
        }
        HarnessEvent::Result { message } => {
            transcript.finish_activity();
            let streamed = std::mem::take(&mut transcript.streaming_assistant);
            transcript.push_assistant(if message.is_empty() {
                streamed
            } else {
                message.clone()
            });
            if !stopped {
                transcript.status = if transcript.sidekick_established {
                    SubagentTranscriptStatus::Idle
                } else {
                    SubagentTranscriptStatus::Completed
                };
            }
            transcript.elapsed = Some(transcript.started.elapsed());
        }
        HarnessEvent::Error { message } => {
            transcript.finish_activity();
            let streamed = std::mem::take(&mut transcript.streaming_assistant);
            transcript.push_assistant(streamed);
            transcript.push_entry(SubagentTranscriptEntry::Error(bounded_text(message)));
            // A persistent sidekick retains its handle after a failed or
            // cancelled task. The Error entry records that task's failure;
            // Idle says the sidekick itself can accept another task.
            if !stopped {
                transcript.status = if transcript.sidekick_established {
                    SubagentTranscriptStatus::Idle
                } else {
                    SubagentTranscriptStatus::Failed
                };
            }
            transcript.elapsed = Some(transcript.started.elapsed());
        }
        HarnessEvent::AgentStart | HarnessEvent::ToolInputDelta { .. } => {}
    }
    transcript.compact_activity();
    let status_changed = previous_status != Some(transcript.status);
    let status = transcript.status;
    let terminal = !transcript.status.is_active();
    let terminal_detached = transcript.detached && !transcript.status.is_active();
    if terminal_detached {
        app.subagent_activity.remove(&id);
    }
    if status_changed {
        advance_workflow_stage(app, id, status, None);
        app.invalidate_agent_list();
    }
    if terminal {
        app.retain_agent_history();
    }
}

/// Join a spawned agent to the graph stage it executes. Called for workflow
/// stages only; ordinary subagents have no stage to bind.
pub(crate) fn bind_workflow_stage(app: &mut App, spawn: u64, run: u64, stage: &str) {
    let Some(workflow) = app.workflows.get_mut(&run) else {
        return;
    };
    let Some(at) = workflow.position(stage) else {
        return;
    };
    workflow.spawned(stage, spawn);
    app.workflow_stages.insert(spawn, (run, at));
}

/// Mirror an executing agent's status onto the stage it runs, so the graph
/// panel and the agent list can never disagree about one stage.
fn advance_workflow_stage(
    app: &mut App,
    spawn: u64,
    status: SubagentTranscriptStatus,
    detail: Option<String>,
) {
    let Some((run, at)) = app.workflow_stages.get(&spawn).copied() else {
        return;
    };
    let Some(workflow) = app.workflows.get_mut(&run) else {
        return;
    };
    let state = match status {
        SubagentTranscriptStatus::Queued => StageState::Queued,
        SubagentTranscriptStatus::Running => StageState::Running,
        SubagentTranscriptStatus::Idle | SubagentTranscriptStatus::Stopped => StageState::Done,
        SubagentTranscriptStatus::Completed => StageState::Done,
        SubagentTranscriptStatus::Failed => StageState::Failed,
    };
    workflow.advance(at, state, detail);
}

fn mark_subagent_detached(app: &mut App, id: u64) {
    let terminal = app
        .subagent_transcripts
        .get_mut(&id)
        .is_some_and(|transcript| {
            transcript.detached = true;
            !transcript.status.is_active()
        });
    if terminal {
        app.subagent_activity.remove(&id);
    }
}

fn mark_sidekick_result(app: &mut App, output: &serde_json::Value) {
    let Some(id) = output.get("spawnId").and_then(serde_json::Value::as_u64) else {
        return;
    };
    let Some(transcript) = app.subagent_transcripts.get_mut(&id) else {
        return;
    };
    if transcript.status == SubagentTranscriptStatus::Stopped {
        return;
    }
    if output
        .get("persistent")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        transcript.persistent = true;
        transcript.sidekick_established = true;
        transcript.status = match output.get("status").and_then(serde_json::Value::as_str) {
            Some("queued") => SubagentTranscriptStatus::Queued,
            Some("running") | Some("busy") => SubagentTranscriptStatus::Running,
            _ => transcript.status,
        };
        app.invalidate_agent_list();
        return;
    }
    match output
        .get("termination")
        .and_then(serde_json::Value::as_str)
    {
        Some("retained") => {
            transcript.persistent = true;
            transcript.sidekick_established = true;
            transcript.status = SubagentTranscriptStatus::Idle;
        }
        Some("stopped") => {
            transcript.persistent = true;
            transcript.sidekick_established = false;
            transcript.status = SubagentTranscriptStatus::Stopped;
        }
        _ => match output.get("status").and_then(serde_json::Value::as_str) {
            // A successful status lookup itself proves that the runtime
            // returned a reusable handle after initial creation.
            Some("idle") => {
                transcript.persistent = true;
                transcript.sidekick_established = true;
                transcript.status = SubagentTranscriptStatus::Idle;
            }
            Some("busy") => {
                transcript.persistent = true;
                transcript.sidekick_established = true;
                transcript.status = SubagentTranscriptStatus::Running;
            }
            Some("stopped") => {
                transcript.persistent = true;
                transcript.sidekick_established = false;
                transcript.status = SubagentTranscriptStatus::Stopped;
            }
            _ => return,
        },
    }
    transcript.elapsed = Some(transcript.started.elapsed());
    app.invalidate_agent_list();
}

fn detached_spawn_id(tool_name: &str, output: &serde_json::Value) -> Option<u64> {
    if matches!(tool_name, "subagent" | "workflow")
        && output
            .get("termination")
            .and_then(serde_json::Value::as_str)
            == Some("detached")
    {
        output
            .get(if tool_name == "workflow" {
                "runId"
            } else {
                "spawnId"
            })
            .and_then(serde_json::Value::as_u64)
    } else {
        None
    }
}
