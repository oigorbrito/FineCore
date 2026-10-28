use super::integrations::*;

// The slash-command handlers (`/mode`, `/rewind`, `/skills`,
// `/mcp`, ...) behind the composer palette. Each takes the trimmed
// argument text and drives the [`App`] overlay/queue/notice machinery; a
// few hand a follow-up `WorkerCmd` to the agent for anything that needs
// the model loop (model list fetch, session load, MCP reconnect).

use tokio::sync::mpsc;

use ratatui::text::{Line, Span};

use crate::msg::{Provider, WorkerCmd};
use crate::tui::components::picker::ListPicker;
use crate::tui::components::transcript::BlockSpacing;
use crate::view::{self, theme};

use super::super::state::{
    App, ModelPickerTarget, Overlay, SESSION_ACTIONS, SETTINGS_ROWS, SUBAGENT_ROWS,
};
use super::super::{
    copy_command, expand_tool, push_error, push_notice, start_submission, SESSIONS_WINDOW,
};

pub(crate) fn slash_command(
    app: &mut App,
    command: &str,
    worker: &mpsc::UnboundedSender<WorkerCmd>,
    width: usize,
) {
    // A slash command is user activity even though it is not a model turn.
    // Dismiss the empty-state card before writing command output so `/help`
    // (and command errors/notices) are visible on the first frame afterward.
    app.welcome_dismissed = true;
    app.overlay_stack.clear();
    app.picker_pending = None;
    let dim = theme().dim;
    if command == "queue" {
        let queued = app.prompt_queue.len();
        let message = match queued {
            0 => "queue empty".to_string(),
            1 => "1 prompt queued".to_string(),
            _ => format!("{queued} prompts queued"),
        };
        app.push_line(Line::from(Span::styled(message, dim)));
        return;
    }
    if let Some(rest) = command.strip_prefix("changelog") {
        let arg = rest.trim();
        if rest.is_empty() || arg == "full" {
            let lines = view::markdown_lines(&crate::changelog::notes(arg == "full"), width, "");
            app.push_transcript_block(lines, BlockSpacing::Section);
        } else {
            push_error(app, "usage: /changelog [full]");
        }
        return;
    }
    if let Some(rest) = command.strip_prefix("queue ") {
        if rest.trim() == "clear" {
            let cleared = app.prompt_queue.len();
            app.prompt_queue.clear();
            let message = match cleared {
                0 => "queue already empty".to_string(),
                1 => "cleared 1 queued prompt".to_string(),
                _ => format!("cleared {cleared} queued prompts"),
            };
            app.push_line(Line::from(Span::styled(message, dim)));
            return;
        }
        push_error(app, "usage: /queue [clear]");
        return;
    }
    if let Some(rest) = command.strip_prefix("expand") {
        let nth = rest.trim().parse::<usize>().unwrap_or(1).max(1);
        expand_tool(app, nth, width);
        return;
    }
    if let Some(rest) = command.strip_prefix("mode") {
        if rest.is_empty() || rest.starts_with(' ') {
            mode_command(app, rest.trim());
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("rewind") {
        if rest.is_empty() || rest.starts_with(' ') {
            rewind_command(app, rest.trim(), worker);
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("sidekick") {
        if rest.is_empty() || rest.starts_with(' ') {
            sidekick_command(app, rest.trim(), worker);
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("subagents") {
        if rest.is_empty() {
            app.overlay = Some(Overlay::Subagents {
                picker: ListPicker::new(SUBAGENT_ROWS),
            });
            return;
        }
        if let Some(arg) = rest.strip_prefix(' ') {
            match arg.trim().parse::<u32>() {
                Ok(depth) => {
                    let set = app.cfg.subagent_depth.set(depth);
                    let message = match crate::config::save_subagent_settings(&app.cfg.subagent_depth) {
                        Ok(_) => format!(
                            "subagent nesting depth set to {set} (saved; applies to the next spawn)"
                        ),
                        Err(err) => format!(
                            "subagent nesting depth set to {set} for this session (save failed: {err})"
                        ),
                    };
                    app.push_line(Line::from(Span::styled(message, dim)));
                }
                Err(_) => {
                    push_error(app, "usage: /subagents [positive depth]");
                }
            }
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("extensions") {
        if rest.is_empty() {
            // Same interface as /theme and /provider: a picker overlay
            // where enter toggles the selected extension.
            app.overlay = Some(Overlay::Extensions {
                picker: ListPicker::new(crate::extensions::EXTENSIONS.len()),
            });
            return;
        }
        if let Some(args) = rest.strip_prefix(' ') {
            let mut parts = args.split_whitespace();
            let enabled = match parts.next() {
                Some("enable" | "add" | "on") => true,
                Some("disable" | "remove" | "delete" | "off") => false,
                _ => {
                    push_error(app, "usage: /extensions [enable|disable <name>]");
                    return;
                }
            };
            let name = parts.next().unwrap_or("");
            if crate::extensions::find(name).is_none() {
                let known = crate::extensions::EXTENSIONS
                    .iter()
                    .map(|spec| spec.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                push_error(
                    app,
                    format!("unknown extension: {name} — valid extensions: {known}"),
                );
                return;
            }
            let state = if enabled { "enabled" } else { "disabled" };
            match crate::config::save_extension(name, enabled) {
                Ok(_) => {
                    push_notice(
                        app,
                        format!("extension {name} {state} (applies to the next run)"),
                    );
                    if worker.send(WorkerCmd::ReloadExtensions).is_err() {
                        push_error(app, "worker is gone; restart finecore");
                    }
                }
                Err(err) => {
                    push_error(
                        app,
                        format!("extension {name} not {state} (save failed: {err})"),
                    );
                }
            }
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("mcp") {
        if rest.is_empty() {
            let entries = crate::tui::mcp_picker::snapshot(&app.cfg.mcp);
            if entries.is_empty() {
                push_notice(
                    app,
                    "no MCP servers configured or loaded — /mcp add <name> <command>",
                );
                return;
            }
            app.overlay = Some(Overlay::Mcp {
                picker: ListPicker::new(entries.len()),
                filter: String::new(),
                entries,
            });
            return;
        }
        if let Some(args) = rest.strip_prefix(' ') {
            mcp_command(app, args, worker);
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("plugin") {
        if rest.is_empty() || rest.starts_with(' ') {
            plugin_command(app, rest.trim(), worker);
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("copy") {
        if rest.is_empty() || rest.starts_with(' ') {
            copy_command(app, rest.trim());
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("refine") {
        if rest.is_empty() || rest.starts_with(' ') {
            super::refine_command(app, rest.trim(), worker);
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("skills") {
        if rest.is_empty() || rest.starts_with(' ') {
            skills_command(app, rest.trim(), worker);
            return;
        }
    }
    if command == "effort" {
        let request_id = app.next_picker_request;
        app.next_picker_request += 1;
        app.picker_pending = Some((request_id, ModelPickerTarget::ActiveModelEffort));
        if worker
            .send(WorkerCmd::ListModels {
                request_id,
                filter: String::new(),
            })
            .is_err()
        {
            app.picker_pending = None;
            push_error(app, "worker is gone; restart finecore");
        } else {
            push_notice(app, "fetching model efforts…");
        }
        return;
    }
    if let Some(rest) = command.strip_prefix("models") {
        if rest.is_empty() || rest.starts_with(' ') {
            // Fetch the full catalog; the argument seeds the picker's
            // live filter so the user can widen it without refetching.
            let request_id = app.next_picker_request;
            app.next_picker_request += 1;
            app.picker_pending = Some((
                request_id,
                ModelPickerTarget::Models {
                    filter: rest.trim().to_lowercase(),
                },
            ));
            if worker
                .send(WorkerCmd::ListModels {
                    request_id,
                    filter: String::new(),
                })
                .is_err()
            {
                app.picker_pending = None;
                push_error(app, "worker is gone; restart finecore");
            } else {
                push_notice(app, "fetching models…");
            }
            return;
        }
    }
    if let Some(rest) = command.strip_prefix("sessions") {
        let arg = rest.trim();
        let Some(base) = crate::config::sessions_dir() else {
            push_error(app, "no home directory for session storage");
            return;
        };
        let dir = base.join(orca_harness_extensions::workspace_key(
            &app.cfg.workspace_root,
        ));
        let sessions = orca_harness_extensions::SessionFile::list(&dir);
        if arg.is_empty() {
            if sessions.is_empty() {
                push_notice(app, "no recorded sessions for this workspace");
                return;
            }
            // Same interface as /provider and /theme: a picker overlay,
            // preselected on the current session. The display windows to
            // the last few sessions (newest first), and ↑↓/PgUp/PgDn
            // navigate the whole list just like the /models picker.
            let index = sessions
                .iter()
                .position(|s| app.cfg.session_id.as_deref() == Some(s.meta.id.as_str()))
                .unwrap_or(0)
                // An old-but-current session stays reachable; the window
                // just opens on the newest rows.
                .min(SESSIONS_WINDOW.saturating_sub(1));
            app.overlay = Some(Overlay::Sessions {
                picker: ListPicker::with_selected(sessions.len(), index).actions(SESSION_ACTIONS),
                sessions,
            });
            return;
        }
        match sessions.iter().find(|s| s.meta.id.starts_with(arg)) {
            Some(session) => {
                if worker
                    .send(WorkerCmd::LoadSession {
                        path: session.path.clone(),
                    })
                    .is_err()
                {
                    push_error(app, "worker is gone; restart finecore");
                } else {
                    push_notice(app, format!("loading session {}…", session.meta.id));
                }
            }
            None => {
                push_error(
                    app,
                    format!("no session matching {arg} — /sessions lists them"),
                );
            }
        }
        return;
    }
    if let Some(rest) = command.strip_prefix("theme") {
        let arg = rest.trim();
        if arg.is_empty() {
            // Same interface as /models and /provider: a picker overlay,
            // preselected on the active theme.
            let current = view::theme_name();
            let index = view::ThemeName::ALL
                .iter()
                .position(|name| *name == current)
                .unwrap_or(0);
            app.overlay = Some(Overlay::Themes {
                picker: ListPicker::with_selected(view::ThemeName::ALL.len(), index),
            });
            return;
        }
        match view::ThemeName::from_str(arg) {
            Some(name) => {
                view::set_theme(name);
                let note = match crate::config::save_theme(name.slug()) {
                    Ok(_) => format!("theme set to {}", name.label()),
                    Err(err) => format!("theme set to {} (not saved: {err})", name.label()),
                };
                push_notice(app, note);
            }
            None => {
                push_error(app, format!(
                        "unknown theme: {arg} — valid themes: default, mono, dracula, solarized-dark, one-dark, monokai, nord, orca"
                    ));
            }
        }
        return;
    }
    match command {
        "quit" | "exit" | "q" => app.quit = true,
        "clear" => {
            if worker.send(WorkerCmd::Clear).is_err() {
                push_error(app, "worker is gone; restart finecore");
            }
        }
        "usage" => {
            app.overlay = Some(Overlay::Usage);
        }
        "compact" => {
            if worker.send(WorkerCmd::Compact).is_err() {
                push_error(app, "worker is gone; restart finecore");
            } else {
                push_notice(app, "compacting conversation…");
            }
        }
        "fork" => {
            if worker.send(WorkerCmd::Fork).is_err() {
                push_error(app, "worker is gone; restart finecore");
            } else {
                push_notice(app, "forking session…");
            }
        }
        "provider" => {
            app.overlay = Some(Overlay::Providers {
                picker: ListPicker::new(Provider::ALL.len()),
            });
        }
        "settings" => {
            app.overlay = Some(Overlay::Settings {
                picker: ListPicker::new(SETTINGS_ROWS),
            });
        }
        "hotkeys" => {
            hotkeys_command(app, width);
        }
        "help" | "" => {
            app.overlay = Some(Overlay::Help {
                filter: String::new(),
                picker: ListPicker::new(crate::tui::command_catalog::COMMANDS.len()),
            });
        }
        other => {
            push_error(app, format!("unknown command: /{other}"));
        }
    }
}

include!("dispatch/sidekick.rs");
