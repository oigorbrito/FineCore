fn sidekick_command(app: &mut App, args: &str, worker: &mpsc::UnboundedSender<WorkerCmd>) {
    const USAGE: &str = "usage: /sidekick [local|fast|mid|frontier] <task> | /sidekick stop";
    let mut parts = args.split_whitespace();
    let Some(first) = parts.next() else {
        push_error(app, USAGE);
        return;
    };
    if first == "stop" {
        if parts.next().is_some() {
            push_error(app, USAGE);
            return;
        }
        let mut ids = app
            .subagent_transcripts
            .values()
            .filter(|sidekick| {
                sidekick.persistent
                    && matches!(
                        sidekick.status,
                        crate::tui::state::SubagentTranscriptStatus::Queued
                            | crate::tui::state::SubagentTranscriptStatus::Running
                            | crate::tui::state::SubagentTranscriptStatus::Idle
                    )
            })
            .map(|sidekick| sidekick.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        if ids.is_empty() {
            push_notice(app, "No active sidekicks.");
        } else {
            app.overlay = Some(Overlay::SidekickStop {
                picker: ListPicker::new(ids.len()),
                ids,
            });
        }
        return;
    }
    let (tier, task) = match first {
        "local" | "mid" | "frontier" => {
            (Some(first.to_string()), parts.collect::<Vec<_>>().join(" "))
        }
        "fast" => (
            Some("flash".to_string()),
            parts.collect::<Vec<_>>().join(" "),
        ),
        _ => (None, args.to_string()),
    };
    if task.trim().is_empty() {
        push_error(app, USAGE);
        return;
    }
    if worker
        .send(WorkerCmd::SidekickStart { task, tier })
        .is_err()
    {
        push_error(app, "worker is gone; restart finecore");
    }
}

/// Apply a session mode: the notice, the yolo warning, and the plan
/// episode end all live here so every entry point says the same thing.
pub(crate) fn apply_mode(app: &mut App, next: crate::mode::Mode) {
    let previous = app.cfg.mode.get();
    app.cfg.mode.set(next);
    // One plain-text line, glyph included — deliberately not the
    // shared notification (its dot is accent-colored): a mode change
    // is text like any other, nothing highlighted.
    app.push_line(Line::from(Span::styled(
        format!("• {} mode · {}", next.label(), next.description()),
        theme().dim,
    )));
    // Leaving plan mode ends the episode. Report the plans the agent
    // actually wrote — observed from tool results, not guessed — and say
    // nothing when it wrote none: plan mode is also a fine way to just
    // look around, and announcing a missing file would be nagging.
    if previous == crate::mode::Mode::Plan && next != crate::mode::Mode::Plan {
        for path in app.cfg.plan.end() {
            push_notice(app, format!("plan saved to {path}"));
        }
    }
}

/// The name the plan decision is shown under, and how its answer is told
/// apart from an ordinary tool approval.
pub(crate) const PLAN_APPROVAL: &str = "plan";

/// A plan-mode turn that wrote or revised a plan ends by asking whether to
/// approve it and start implementing — the ordinary yes/no approval prompt.
/// Nothing awaits the answer on the channel: the key handler reports it and
/// [`approve_plan`] acts on a yes.
pub(crate) fn request_plan_approval(app: &mut App) {
    if !app.cfg.mode.is_plan() {
        return;
    }
    let fresh = app.cfg.plan.take_fresh();
    if fresh.is_empty() {
        return;
    }
    let (respond, _) = tokio::sync::oneshot::channel();
    app.approval_choice = None;
    app.approval = Some(crate::msg::ApprovalRequest {
        tool_name: PLAN_APPROVAL.into(),
        detail: format!(
            "{} written · approve and start implementing?",
            fresh.join(", ")
        ),
        yes_no: true,
        respond,
    });
}

/// The plan was approved: leave plan mode and run the implementation turn.
pub(crate) fn approve_plan(app: &mut App, worker: &mpsc::UnboundedSender<WorkerCmd>, width: usize) {
    let plans = app.cfg.plan.written().join(", ");
    apply_mode(app, crate::mode::Mode::Normal);
    start_submission(
        app,
        worker,
        format!(
            "Implement the approved plan in {plans}: work through its steps in order, verify \
             each as it says, and tick its checkbox in the file when done."
        ),
        width,
    );
}

/// `/mode [normal|plan|auto|yolo]` — no argument opens the standard picker,
/// preselected on the current mode, exactly like /provider and /theme.
/// A name picks directly: `mode` is a command typed hourly, so the
/// text form stays.
pub(crate) fn mode_command(app: &mut App, arg: &str) {
    use crate::mode::Mode;
    if arg.is_empty() {
        let index = Mode::ALL
            .iter()
            .position(|mode| *mode == app.cfg.mode.get())
            .unwrap_or(0);
        app.overlay = Some(Overlay::Mode {
            picker: ListPicker::with_selected(Mode::ALL.len(), index),
        });
        return;
    }
    match Mode::from_label(arg) {
        Some(mode) => apply_mode(app, mode),
        None => {
            push_error(
                app,
                format!("unknown mode: {arg} — valid modes: normal, plan, orchestrate, auto, yolo"),
            );
        }
    }
}

/// `/rewind [n]` — drop the last n user turns (default 1) from the
/// conversation and from the recorded session, so the next prompt
/// continues from before them.
pub(crate) fn rewind_command(app: &mut App, arg: &str, worker: &mpsc::UnboundedSender<WorkerCmd>) {
    let turns = if arg.is_empty() {
        1
    } else {
        match arg.parse::<usize>() {
            Ok(0) | Err(_) => {
                push_error(
                    app,
                    "usage: /rewind [n] — n is how many turns to drop (default 1)",
                );
                return;
            }
            Ok(turns) => turns,
        }
    };
    if worker.send(WorkerCmd::Rewind { turns }).is_err() {
        push_error(app, "worker is gone; restart finecore");
    }
}

/// `/hotkeys` — a scrollable inventory of the keyboard bindings handled by
/// the TUI. The catalog is shared data so additions are visible and testable.
pub(crate) fn hotkeys_command(app: &mut App, width: usize) {
    let t = theme();
    let key_width = crate::tui::command_catalog::HOTKEYS
        .iter()
        .map(|hotkey| hotkey.keys.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = vec![Line::from(Span::styled("  hotkeys", t.strong))];
    for hotkey in crate::tui::command_catalog::HOTKEYS {
        let text = format!(
            "  {:key_width$}  {}  {}",
            hotkey.keys, hotkey.description, hotkey.category
        );
        lines.push(Line::from(Span::styled(
            view::truncate_line(&text, width),
            t.dim,
        )));
    }
    app.push_transcript_block(lines, BlockSpacing::Tight);
}
