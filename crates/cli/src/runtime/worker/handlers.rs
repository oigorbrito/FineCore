use std::sync::Arc;

use orca_harness_core::{
    Agent, CancellationToken, Context, Model, Tool, ToolCall, ToolContext, ToolResult,
};
use orca_harness_extensions::{HarnessEvent, SessionHandler};
use tokio::sync::mpsc;

use crate::msg::{RunId, UiMsg};
use crate::{skills::Skills, Endpoint};

pub(super) type BuiltAgent = (
    Agent<Arc<dyn Model>>,
    Arc<orca_harness_tools::SubagentTool<Arc<dyn Model>>>,
);

pub(super) struct CancelSubagentsOnDrop(pub(super) orca_harness_tools::SubagentManager);

impl Drop for CancelSubagentsOnDrop {
    fn drop(&mut self) {
        // `cancel_all` spares sidekicks by design; session end must not.
        self.0.close();
    }
}

/// Rebuild the parent agent, carrying live sidekicks onto the new
/// `subagent` tool so they outlive model switches and reloads.
pub(super) fn rebuild<F>(
    build: &mut F,
    endpoint: &Endpoint,
    preserve_local_tools: bool,
    previous: &orca_harness_tools::SubagentTool<Arc<dyn Model>>,
) -> BuiltAgent
where
    F: FnMut(&Endpoint, bool) -> BuiltAgent,
{
    let built = build(endpoint, preserve_local_tools);
    built.1.adopt_sidekicks(previous);
    built
}

/// A path shown to the user: relative to the working directory when it
/// is inside it, absolute otherwise. Transcript notices stay short.
pub(super) fn workspace_relative(path: &std::path::Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok())
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(super) async fn handle_shell(
    id: RunId,
    command: String,
    working_dir: String,
    cancel: CancellationToken,
    user_shell_call_id: &mut u64,
    context: &mut Context,
    session: Option<&Arc<SessionHandler>>,
    ui: &mpsc::UnboundedSender<UiMsg>,
) -> bool {
    let _ = ui.send(UiMsg::RunStarted {
        id: id.clone(),
        cancel: cancel.clone(),
    });

    *user_shell_call_id += 1;
    let call = ToolCall {
        id: format!("user-shell-{user_shell_call_id}"),
        name: "shell".into(),
        arguments: serde_json::json!({ "command": command }),
    };
    context.push_user(format!(
        "!{}",
        call.arguments["command"].as_str().unwrap_or_default()
    ));
    context.push_assistant_tool_calls(None, vec![call.clone()]);
    let _ = ui.send(UiMsg::Event(HarnessEvent::ToolCall {
        tool_call_id: call.id.clone(),
        tool_name: call.name.clone(),
        input: call.arguments.clone(),
    }));

    let tool = orca_harness_tools::ShellTool::local().working_dir(working_dir);
    let tool_context = ToolContext {
        call_id: call.id.clone(),
        tool_name: call.name.clone(),
        cancellation: cancel,
        deadline: None,
    };
    let result = match tool.call(call.arguments.clone(), &tool_context).await {
        Ok(output) => ToolResult::ok(&call, output),
        Err(err) => ToolResult::error(&call, err.to_string()),
    };
    let _ = ui.send(UiMsg::Event(HarnessEvent::ToolResult {
        tool_call_id: result.call_id.clone(),
        tool_name: result.tool_name.clone(),
        output: result.output.clone(),
        is_error: result.is_error,
    }));
    context.append_tool_results(vec![result]);
    if let Some(session) = session {
        session.sync(context);
    }
    ui.send(UiMsg::ShellDone { id }).is_err()
}

pub(super) async fn handle_refine(
    endpoint: &Endpoint,
    ui: &mpsc::UnboundedSender<UiMsg>,
    skills: &Skills,
    context: &Context,
    refine_applied: &mut Vec<crate::refine::Applied>,
) -> bool {
    // A one-shot proposer pass over a scratch context: the user's
    // conversation is read, never written. The model is built fresh
    // from the live endpoint so provider and model switches are respected.
    let model = endpoint.build_model_for_ui(Some(ui.clone()));
    let existing: Vec<(String, String)> = skills
        .catalog()
        .into_iter()
        .map(|entry| (entry.name, entry.description))
        .collect();
    let result = crate::refine::run_proposer(model.as_ref(), context.messages(), &existing).await;
    let result = match result {
        // Declining is success, not silence: say why and stop.
        Ok(crate::refine::Refined::Nothing(reason)) => {
            let reason = match reason.is_empty() {
                true => String::new(),
                false => format!(": {reason}"),
            };
            let _ = ui.send(UiMsg::Notice(format!(
                "refine: no skill proposed — nothing in this trajectory is worth \
                 packaging{reason}"
            )));
            return false;
        }
        Ok(crate::refine::Refined::Skill(outcome)) => Ok(outcome),
        Err(err) => Err(err),
    };
    let proposal = result.as_ref().ok().map(|outcome| outcome.proposal.clone());
    if ui.send(UiMsg::RefineDone(Box::new(result))).is_err() {
        return true;
    }
    let Some(proposal) = proposal else {
        return false;
    };
    // Accept/reject rides the standard tool-approval gate (y/n/a/A).
    let (respond, decision) = tokio::sync::oneshot::channel();
    let request = crate::msg::ApprovalRequest {
        tool_name: "refine".into(),
        detail: format!("apply skill {}", proposal.name),
        yes_no: true,
        respond,
    };
    if ui.send(UiMsg::Approval(request)).is_err() {
        return true;
    }
    use crate::msg::ApprovalResponse as R;
    let approved = matches!(
        decision.await,
        Ok(R::AllowOnce | R::AllowAlways | R::AllowAlwaysSave)
    );
    if !approved {
        let _ = ui.send(UiMsg::Notice(format!(
            "refine: proposal {} discarded",
            proposal.name
        )));
        return false;
    }
    let applied = skills
        .project_root()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "no project skills folder to apply into".to_string())
        .and_then(|root| crate::refine::apply(&root, &proposal));
    let notice = match applied {
        Ok(applied) => {
            skills.reload();
            let notice = format!(
                "applied skill {} → {} · /refine undo reverts it",
                proposal.name,
                workspace_relative(&applied.dir)
            );
            refine_applied.push(applied);
            notice
        }
        Err(err) => format!("refine apply failed: {err}"),
    };
    let _ = ui.send(UiMsg::Notice(notice));
    false
}

pub(super) fn is_current_process_generation(
    event_generation: u64,
    current: &std::sync::atomic::AtomicU64,
) -> bool {
    current.load(std::sync::atomic::Ordering::Acquire) == event_generation
}
