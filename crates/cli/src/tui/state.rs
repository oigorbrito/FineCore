//! The interactive terminal's persistent state: configuration handed in
//! from the runner, the full-screen [`App`] structure, and the modal
//! selectors (model/location/provider/theme/view/spacing/usage/api-key/
//! settings/approvals/extensions/mcp/plugins/skills/sessions) that overlays can
//! host. Rendering and key handling live in the sibling modules; this one
//! only owns the data.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use ratatui::text::Line;

use crate::msg::{ApprovalRequest, Provider};
use crate::tui::components::ask::AskForm;
use crate::tui::components::picker::{ListPicker, PickerAction};
use orca_harness_core::{CancellationToken, Image};
use orca_harness_model_providers::openrouter::ModelInfo;
use orca_harness_model_providers::SupportedEfforts;

use super::format::TokenEstimator;

mod activity;
pub(crate) mod subagent_history;
mod subagents;
pub(crate) mod workflow;
pub(crate) use activity::{ToolActivity, ToolStatus};
pub(crate) use subagents::{
    AgentBodyCache, AgentBrowser, AgentTab, SpawnActivity, SubagentDisplay, SubagentTranscript,
    SubagentTranscriptEntry, SubagentTranscriptStatus,
};

include!("state/pickers.rs");

impl ViewMode {
    pub(crate) const ALL: [Self; 2] = [Self::Classic, Self::Split];

    pub(crate) fn stored() -> Self {
        match crate::config::stored_view().as_deref() {
            Some("split") => Self::Split,
            _ => Self::Classic,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Split => "Split",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Split => "split",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectorMode {
    Summary,
    Debug,
}

impl InspectorMode {
    pub(crate) const ALL: [Self; 2] = [Self::Summary, Self::Debug];

    pub(crate) fn stored() -> Self {
        match crate::config::stored_inspector().as_deref() {
            Some("debug") => Self::Debug,
            _ => Self::Summary,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Debug => "Debug",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Debug => "debug",
        }
    }
}

pub(crate) struct InspectorBodyCache {
    pub(crate) call_id: String,
    pub(crate) complete: bool,
    pub(crate) has_output: bool,
    pub(crate) is_error: bool,
    pub(crate) width: usize,
    pub(crate) mode: InspectorMode,
    pub(crate) lines: Vec<Line<'static>>,
}

pub(crate) enum RunState {
    Idle,
    Running {
        id: crate::msg::RunId,
        started: Instant,
        cancel: CancellationToken,
    },
}

/// One status-line item reached by pressing down from an empty composer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusFocus {
    Context,
    Processes,
    Agents,
}

pub(crate) enum HeldInput {
    Text(String),
    Image { label: String, image: Image },
}

impl HeldInput {
    pub(crate) fn marker(&self, index: usize) -> String {
        match self {
            Self::Text(text) => {
                let lines = text.lines().count().max(1);
                let unit = if lines == 1 { "line" } else { "lines" };
                format!("[▤ Pasted text #{index}, {lines} {unit}]")
            }
            Self::Image { label, .. } => format!("[▧ {label}]"),
        }
    }
}

pub(crate) struct App {
    pub(crate) cfg: TuiConfig,
    /// The workspace's branch captured when the TUI starts. `None` covers
    /// non-Git directories and detached HEADs without adding Git work to
    /// every frame.
    pub(crate) git_branch: Option<String>,
    /// Explicit reasoning effort chosen for the active model. `None` lets
    /// the provider use its advertised default.
    pub(crate) reasoning_effort: Option<String>,
    /// Lines waiting to move into the transcript on the next tick.
    pub(crate) pending_history: Vec<Line<'static>>,
    /// The full session transcript (capped at [`super::TRANSCRIPT_CAP`]).
    pub(crate) transcript: Vec<Line<'static>>,
    /// Lines scrolled up from the bottom of the transcript.
    pub(crate) scroll: usize,
    /// When the reader last scrolled. Drives the transient selection hint.
    pub(crate) scroll_hint_at: Option<Instant>,
    /// Previous wrapped overflow height. Used to keep the same top row
    /// anchored while live content or the composer region changes size.
    pub(crate) transcript_max_scroll: usize,
    pub(crate) reasoning: String,
    /// When the current reasoning phase started streaming.
    pub(crate) reasoning_started: Option<Instant>,
    pub(crate) thinking_log: Vec<ThinkingRecord>,
    pub(crate) text: String,
    pub(crate) streaming_markdown: std::cell::RefCell<super::render::StreamingMarkdownCache>,
    pub(crate) pending_assistant: Option<String>,
    pub(crate) assistant_started: bool,
    pub(crate) run: RunState,
    pub(crate) next_run_id: u64,
    pub(crate) approval: Option<ApprovalRequest>,
    /// The approval choice the arrows have moved to. `None` until the
    /// person moves it, so an enter typed for the composer never answers
    /// a prompt that appeared under it.
    pub(crate) approval_choice: Option<usize>,
    /// The newest recorded sessions, read once at startup for the welcome
    /// card; never read from disk while drawing.
    pub(crate) recent_sessions: Vec<crate::tui::components::welcome::RecentSession>,
    /// Active structured clarification form requested by the `ask` tool.
    pub(crate) ask: Option<AskForm>,
    pub(crate) composer: String,
    pub(crate) cursor: usize,
    /// Prompts waiting for the active turn to finish, oldest first.
    /// Held in marker form; expanded against [`App::pastes`] on send.
    pub(crate) prompt_queue: VecDeque<String>,
    /// Held composer entities, indexed by their stable marker number.
    /// Entries are never removed because queued and recalled prompts can
    /// still refer to them.
    pub(crate) pastes: Vec<HeldInput>,
    pub(crate) prompt_history: Vec<String>,
    pub(crate) history_pos: Option<usize>,
    pub(crate) tokens_in: u64,
    pub(crate) tokens_out: u64,
    /// FX-style live turn progress: estimated submitted prompt tokens
    /// once, plus generated output reconciled to provider usage per step.
    pub(crate) turn_tokens_in: u64,
    pub(crate) turn_tokens_out: u64,
    pub(crate) turn_thinking_tokens: Option<u64>,
    pub(super) turn_output_settled: u64,
    pub(super) turn_reasoning_tokens: TokenEstimator,
    pub(super) turn_text_tokens: TokenEstimator,
    pub(super) turn_tool_input_tokens: TokenEstimator,
    pub(crate) cache_read_total: u64,
    pub(crate) cache_write_total: u64,
    /// Model steps that reported usage this session.
    pub(crate) usage_steps: u64,
    /// Approximate size of the model's current context, pi-style: the
    /// last step's provider-reported total, plus bytes/4 estimates for
    /// content appended since (tool results, the next prompt), plus
    /// /compact's estimate. Distinct from the cumulative session totals.
    pub(crate) context_tokens: u64,
    /// The active model's context window, when the catalog knows it.
    pub(crate) context_window: Option<u64>,
    pub(crate) spinner_frame: usize,
    pub(crate) quit: bool,
    pub(crate) tool_log: Vec<ToolRecord>,
    /// Completed per-turn work trees, newest last.
    pub(crate) work_log: Vec<CompletedWork>,
    /// Number of user turns rendered in this session.
    pub(crate) turn_count: usize,
    /// Whether the initial welcome has been replaced by user activity.
    /// Kept separate from `turn_count`: local slash commands do not create
    /// model turns, but their transcript output must still be visible.
    pub(crate) welcome_dismissed: bool,
    /// Background MCP reload progress, independent of startup transcript notices.
    pub(crate) mcp_connecting: bool,
    /// Top-level tool calls made during the current turn.
    pub(crate) turn_tool_calls: usize,
    /// Call lines for in-flight tool calls, keyed by call id.
    pub(crate) pending_calls: HashMap<String, usize>,
    pub(crate) activity_tools: Vec<ToolActivity>,
    pub(crate) view_mode: ViewMode,
    pub(crate) split_tool: Option<usize>,
    /// Which tab the split's right pane shows: the Tool Inspector or the
    /// subagents' status.
    pub(crate) split_tab: SplitTab,
    /// Last inspected call retained across model phases so Split never
    /// collapses or flashes while the next call is being prepared.
    pub(crate) split_snapshot: Option<ToolActivity>,
    pub(crate) split_inspector_cache: Option<InspectorBodyCache>,
    pub(crate) inspector_mode: InspectorMode,
    pub(crate) split_scroll: u16,
    /// Where the last frame drew the inspector, so the wheel can be routed
    /// to it without the event loop re-deriving the split geometry.
    pub(crate) inspector_area: Option<ratatui::layout::Rect>,
    /// Live inner activity of running subagents, keyed by spawn id.
    pub(crate) subagent_activity: HashMap<u64, SpawnActivity>,
    /// Session-visible agent history used by the live transcript browser.
    pub(crate) subagent_transcripts: HashMap<u64, SubagentTranscript>,
    /// Submitted workflow graphs, keyed by run id, so a run's stages are
    /// visible before any of them spawns.
    pub(crate) workflows: HashMap<u64, workflow::WorkflowRun>,
    /// Spawn id to the (run, stage index) it executes, for workflow stages.
    pub(crate) workflow_stages: HashMap<u64, (u64, usize)>,
    pub(crate) evicted_agent_histories: usize,
    pub(crate) agent_list_cache:
        std::cell::RefCell<Option<std::sync::Arc<super::render::AgentListCache>>>,
    /// The dedicated read-only agent browser, when open.
    pub(crate) agent_browser: Option<AgentBrowser>,
    /// Down from an empty newest composer focuses the interactive status row.
    pub(crate) status_focus: Option<StatusFocus>,
    /// Resolved top-level worker identity retained until the work phase commits,
    /// including when a failed tool result has no structured identity payload.
    pub(crate) subagent_display: HashMap<String, SubagentDisplay>,
    /// Shared selection state for the slash-command palette.
    pub(crate) palette_picker: ListPicker,
    /// Open modal selector, if any.
    pub(crate) overlay: Option<Overlay>,
    /// Parent picker pages. Right/enter descends, left returns one page,
    /// and escape clears the whole flow.
    pub(crate) overlay_stack: Vec<Overlay>,
    /// Picker to open once the requested model catalog arrives.
    pub(crate) picker_pending: Option<(u64, ModelPickerTarget)>,
    pub(crate) next_picker_request: u64,
    /// Text waiting to be handed to the terminal's clipboard: decided
    /// here, written by the run loop between frames.
    pub(crate) clipboard_pending: Option<String>,
    /// The last complete answer the model produced, kept verbatim so
    /// `/copy` yields markdown source rather than the wrapped, styled,
    /// syntax-highlighted lines the transcript holds.
    pub(crate) last_answer: Option<String>,
}
