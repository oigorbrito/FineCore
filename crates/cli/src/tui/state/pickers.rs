/// Everything the terminal needs to know about the workspace it is
/// rendering, passed once by the runner and updated by slash commands
/// and the event loop.
pub struct TuiConfig {
    pub model_name: String,
    pub workspace_name: String,
    /// Canonicalized workspace root: keys this workspace's saved tool
    /// approvals in the config file.
    pub workspace_root: String,
    /// The active endpoint provider; provider switches update it.
    pub provider: Provider,
    /// Shared handle behind the `subagent` tool's nesting cap;
    /// `/subagents` adjusts it live.
    pub subagent_depth: orca_harness_tools::SubagentDepth,
    /// Live background-work counters rendered in the status line.
    pub stats: orca_harness_tools::BackgroundStats,
    /// The recording session's id, if recording; `/sessions` marks it.
    pub session_id: Option<String>,
    /// Shared handle onto the connected MCP servers; `/mcp` reads each
    /// one's tool count and connection error from it.
    pub mcp: crate::mcp::McpServers,
    /// Shared handle onto the scanned skills; `/skills` renders the
    /// catalog, the shadowed copies, and the parse failures from it.
    pub skills: crate::skills::Skills,
    /// Shared handle onto the session mode; `/mode` flips it and the
    /// status line shows it. Read by the plan gate on every tool call,
    /// so a flip applies to the call in flight, not to the next run.
    pub mode: crate::mode::ModeHandle,
    /// The plan artifact the current planning episode may write. Leaving
    /// plan mode ends the episode and reports where the plan went.
    pub plan: crate::plan::PlanArea,
}

/// The interactive model selector: the fetched catalog, a live-typed
/// filter, and the standard shared picker state for the filtered view.
pub(crate) struct ModelPicker {
    pub(crate) subagent: Option<(String, crate::Provider)>,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) filter: String,
    pub(crate) picker: ListPicker,
}

/// The picker to open when a model-catalog request completes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ModelPickerTarget {
    Models {
        filter: String,
    },
    ActiveModelEffort,
    Subagent {
        tier: String,
        provider: crate::Provider,
    },
}

/// Provider-advertised reasoning efforts for the active or newly chosen model.
pub(crate) struct EffortPicker {
    pub(crate) model_id: String,
    pub(crate) context_window: Option<u64>,
    pub(crate) efforts: Vec<String>,
    pub(crate) picker: ListPicker,
}

/// Workspace-relative file and directory inserted into the composer by
/// the `@` mention picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocationEntry {
    pub(crate) path: String,
    pub(crate) directory: bool,
}

pub(crate) struct LocationPicker {
    pub(crate) entries: Vec<LocationEntry>,
    pub(crate) query: String,
    /// Character offset of the `@` that opened this picker.
    pub(crate) token_start: usize,
    pub(crate) picker: ListPicker,
}

/// An enabled, loaded skill inserted into the composer by the `$` picker.
pub(crate) struct SkillMentionPicker {
    pub(crate) entries: Vec<crate::skills::SkillEntry>,
    pub(crate) query: String,
    /// Character offset of the `$` that opened this picker.
    pub(crate) token_start: usize,
    pub(crate) picker: ListPicker,
}

impl LocationPicker {
    pub(crate) fn filtered(&self) -> Vec<&LocationEntry> {
        let needle = self.query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| needle.is_empty() || entry.path.to_lowercase().contains(&needle))
            .collect()
    }

    pub(crate) fn selected(&self) -> Option<LocationEntry> {
        self.filtered().get(self.picker.index()).cloned().cloned()
    }

    pub(crate) fn sync_len(&mut self) {
        let len = self.filtered().len();
        self.picker.set_len(len);
    }
}

impl SkillMentionPicker {
    pub(crate) fn filtered(&self) -> Vec<&crate::skills::SkillEntry> {
        let needle = self.query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
            .collect()
    }

    pub(crate) fn selected(&self) -> Option<String> {
        self.filtered()
            .get(self.picker.index())
            .map(|entry| entry.name.clone())
    }

    pub(crate) fn sync_len(&mut self) {
        let len = self.filtered().len();
        self.picker.set_len(len);
    }
}

impl ModelPicker {
    pub(crate) fn new(models: Vec<ModelInfo>, filter: String) -> Self {
        let needle = filter.to_lowercase();
        let len = models
            .iter()
            .filter(|model| needle.is_empty() || model.id.to_lowercase().contains(&needle))
            .count();
        Self {
            subagent: None,
            models,
            filter,
            picker: ListPicker::new(len),
        }
    }

    pub(crate) fn filtered(&self) -> Vec<&ModelInfo> {
        let needle = self.filter.to_lowercase();
        self.models
            .iter()
            .filter(|m| needle.is_empty() || m.id.to_lowercase().contains(&needle))
            .collect()
    }

    /// The provider metadata under the cursor, if any model matches.
    pub(crate) fn selected_model(&self) -> Option<ModelInfo> {
        let filtered = self.filtered();
        filtered.get(self.picker.index()).cloned().cloned()
    }

    pub(crate) fn reset_filtered_selection(&mut self) {
        self.picker = ListPicker::new(self.filtered().len());
    }
}

impl EffortPicker {
    pub(crate) fn new(model: ModelInfo) -> Option<Self> {
        let reasoning = model.reasoning?;
        let SupportedEfforts::Listed(efforts) = reasoning.supported_efforts? else {
            return None;
        };
        if efforts.is_empty() {
            return None;
        }
        let selected = reasoning
            .default_effort
            .as_ref()
            .and_then(|default| efforts.iter().position(|effort| effort == default))
            .unwrap_or(0);
        Some(Self {
            model_id: model.id,
            context_window: model.context_length,
            picker: ListPicker::with_selected(efforts.len(), selected),
            efforts,
        })
    }

    pub(crate) fn selected(&self) -> Option<String> {
        self.efforts.get(self.picker.index()).cloned()
    }
}

/// A modal selector rendered in the live region. Approval prompts win
/// over overlays; overlays win over the slash palette.
pub(crate) enum Overlay {
    /// Browsable slash-command reference; enter places a command in the composer.
    Help { filter: String, picker: ListPicker },
    /// Model selector over the fetched catalog.
    Models(ModelPicker),
    /// Reasoning-effort selector for a model that advertises choices.
    Efforts(EffortPicker),
    /// Workspace file/folder selector opened by typing `@` in the composer.
    Locations(LocationPicker),
    /// Enabled skill selector opened by typing `$` in the composer.
    SkillMentions(SkillMentionPicker),
    /// Provider selector (OpenRouter, Vercel AI Gateway, CheaperInference,
    /// OpenAI, Codex, local).
    Providers { picker: ListPicker },
    /// Theme selector over `view::ThemeName::ALL`.
    Themes { picker: ListPicker },
    /// Transcript layout selector (classic or split inspector).
    Views { picker: ListPicker },
    /// Session mode selector over `crate::mode::Mode::ALL`.
    Mode { picker: ListPicker },
    /// Vertical spacing between transcript sections.
    TranscriptSpacing { picker: ListPicker },
    /// The mark vocabulary: minimal or glyph.
    Style { picker: ListPicker },
    /// The state-mark shape: squares or circles.
    Marks { picker: ListPicker },
    /// The rail's last-branch corner: elbow or curve.
    Branches { picker: ListPicker },
    /// Default split Tool Inspector rendering.
    Inspector { picker: ListPicker },
    /// Read-only session usage panel; any dismissal key closes it.
    Usage,
    /// Process-tool children that are still running.
    Processes,
    /// Masked API-key entry for a provider whose key is not in the env.
    ApiKey { provider: Provider, input: String },
    /// Settings menu: shows the persisted preferences and jumps into
    /// the provider, model, theme, api-key, and approval pickers.
    Settings { picker: ListPicker },
    /// Session-live subagent governance menu.
    Subagents { picker: ListPicker },
    /// Route or model choices for one subagent setting row.
    SubagentValues {
        setting: SubagentSetting,
        values: Vec<String>,
        picker: ListPicker,
    },
    SubagentNumber {
        setting: SubagentSetting,
        input: String,
        error: String,
    },
    /// This workspace's saved always-allowed tools; enter revokes one.
    Approvals {
        tools: Vec<String>,
        picker: ListPicker,
    },
    /// The harness extension catalog; enter toggles the selected one.
    Extensions { picker: ListPicker },
    /// Standalone and plugin MCP servers. Standalone rows toggle on or
    /// off; plugin rows are read-only and direct management to `/plugin`.
    Mcp {
        entries: Vec<crate::tui::mcp_picker::Entry>,
        filter: String,
        picker: ListPicker,
    },
    /// Registered Agent Plugins with saved and process-live state separated.
    Plugins {
        entries: Vec<crate::config::RegisteredPlugin>,
        filter: String,
        picker: ListPicker,
    },
    /// The skills found on disk; space (or enter) turns the selected one
    /// on or off. Rows are a snapshot of the last scan, so a toggle
    /// redraws immediately while the rescan runs behind it.
    Skills {
        entries: Vec<crate::skills::SkillEntry>,
        filter: String,
        picker: ListPicker,
    },
    /// Recorded sessions for this workspace; enter resumes the selection.
    Sessions {
        sessions: Vec<orca_harness_extensions::SessionFile>,
        picker: ListPicker,
    },
    /// Live persistent sidekicks selectable for `/sidekick stop`.
    SidekickStop { ids: Vec<u64>, picker: ListPicker },
}

pub(crate) use crate::subagent_settings::{SubagentSetting, SUBAGENT_ROWS};

/// Rows in the settings overlay: provider, model, theme, view, inspector,
/// api key, approvals, spacing, style, marks, branches.
/// The split's right pane tabs; Tab switches them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SplitTab {
    #[default]
    Tools,
    Agents,
}

pub(crate) const SETTINGS_ROWS: usize = 11;

/// Row actions in the /sessions picker (space arms them).
pub(crate) const SESSION_ACTIONS: &[PickerAction] = &[PickerAction {
    key: 'd',
    label: "delete",
}];

/// Row actions in the /skills picker. Enter still toggles — the common
/// case stays one key — and space reveals the rest, so deleting a skill
/// is never one stray keystroke away.
pub(crate) const SKILL_ACTIONS: &[PickerAction] = &[
    PickerAction {
        key: 't',
        label: "toggle",
    },
    PickerAction {
        key: 'd',
        label: "delete",
    },
];

/// A finished tool call kept around so the user can expand its full
/// output later with `/expand n`.
pub(crate) struct ToolRecord {
    pub(crate) call_line: String,
    pub(crate) tool_name: String,
    pub(crate) output: serde_json::Value,
    /// Folded inner tool log of a finished subagent call, empty otherwise.
    pub(crate) inner: Vec<String>,
}

/// A completed turn's compacted work rail. The transcript keeps only the
/// summary until the user asks to inspect the tree with ctrl+o.
pub(crate) struct CompletedWork {
    pub(crate) turn: usize,
    pub(crate) summaries: Vec<String>,
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) expanded: bool,
}

#[derive(Clone)]
pub(crate) struct ThinkingRecord {
    pub(crate) elapsed: Duration,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    Classic,
    Split,
}
