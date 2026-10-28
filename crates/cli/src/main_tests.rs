#[cfg(test)]
mod main_tests {
    use std::path::PathBuf;

    use tokio::sync::mpsc;

    use orca_harness_core::{Context, Message, ToolResult};
    use orca_harness_extensions::MEMORY_GUIDANCE;
    use orca_harness_tools::{SubagentSpawn, Workspace};

    use crate::instructions;
    use crate::mode::{Mode, ModeHandle};
    use crate::msg::Provider;
    use crate::plan::PlanArea;
    use crate::runtime::{context_from, rewind_cut, subagent_extensions};
    use crate::{parse_run_args, resolve_theme, select_provider, system_prompt, Config, Planning};

    #[test]
    fn resume_subcommand_reuses_session_options() {
        for command in ["resume", "--resume"] {
            let cfg = parse_run_args(
                [
                    command,
                    "session-id",
                    "--workspace",
                    "/tmp/orca",
                    "--openrouter",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            )
            .unwrap();
            assert_eq!(cfg.resume_id.as_deref(), Some("session-id"));
            assert_eq!(cfg.workspace, PathBuf::from("/tmp/orca"));
            assert!(!cfg.continue_latest);
        }
        for args in [
            vec!["resume"],
            vec!["resume", "--workspace"],
            vec!["resume", ""],
        ] {
            assert!(parse_run_args(args.into_iter().map(str::to_owned).collect()).is_err());
        }
    }

    #[test]
    fn bare_prompt_is_small_and_enforces_the_requested_output_contract() {
        let prompt = crate::headless::bare_system_prompt(
            &Workspace::new(PathBuf::from(".")),
            &["read_file".into(), "grep".into()],
        );
        assert!(prompt.contains("read_file, grep"));
        assert!(prompt.contains("entire response must be only that line"));
        assert!(prompt.contains("Batch independent reads"));
        assert!(!prompt.contains("subagent"));
        assert!(!prompt.contains("memory"));
    }

    #[test]
    fn headless_benchmark_flags_parse_without_changing_interactive_defaults() {
        let cfg = parse_run_args(
            [
                "--openrouter",
                "--model",
                "anthropic/claude-sonnet-4.6",
                "--effort",
                "high",
                "--prompt-cache",
                "--bare",
                "--tools",
                "read_file,grep,glob",
                "--max-output-tokens",
                "512",
                "-p",
                "inspect",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        )
        .unwrap();

        assert_eq!(cfg.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(cfg.max_output_tokens, Some(512));
        assert!(cfg.prompt_cache);
        assert!(cfg.bare);
        assert_eq!(cfg.tools.unwrap(), ["read_file", "grep", "glob"]);
    }

    #[test]
    fn prompt_cache_defaults_on_and_can_be_disabled() {
        let enabled = parse_run_args(
            ["--openrouter", "-p", "inspect"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        assert!(enabled.prompt_cache);

        let disabled = parse_run_args(
            ["--openrouter", "--no-prompt-cache", "-p", "inspect"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        assert!(!disabled.prompt_cache);
    }

    #[test]
    fn bare_and_tool_allowlists_are_headless_only() {
        let error = match parse_run_args(vec!["--bare".into()]) {
            Ok(_) => panic!("--bare without a prompt must fail"),
            Err(error) => error,
        };
        assert!(error.contains("require headless"), "{error}");
    }

    #[test]
    fn normal_is_default_and_auto_requires_its_flag() {
        let default = parse_run_args(Vec::new()).unwrap();
        assert_eq!(default.mode(), Mode::Normal);

        let cfg = parse_run_args(vec!["--auto".into()]).unwrap();
        assert_eq!(cfg.mode(), Mode::Auto);

        let normal = parse_run_args(vec!["--normal".into()]).unwrap();
        assert_eq!(normal.mode(), Mode::Normal);

        let legacy_auto_approve = parse_run_args(vec!["--auto-approve".into()]).unwrap();
        assert_eq!(legacy_auto_approve.mode(), Mode::Normal);
    }

    #[test]
    fn theme_prefers_explicit_then_stored_then_orca() {
        assert_eq!(resolve_theme(None), "orca");
        assert_eq!(crate::config::stored_theme(), None);
        assert_eq!(resolve_theme(Some("default".to_string())), "default");
        assert_eq!(resolve_theme(Some("mono".to_string())), "mono");
        crate::config::save_theme("nord").unwrap();
        assert_eq!(resolve_theme(None), "nord");
        assert_eq!(resolve_theme(Some("mono".to_string())), "mono");
    }

    /// The prompt must keep telling the model to plan and batch independent
    /// calls — dropping this silently reverts the agent to one call per turn.
    #[test]
    fn system_prompt_instructs_concurrent_batching() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(prompt.contains("execute concurrently"));
        assert!(prompt.contains("plan the batch"));
        assert!(prompt.contains("one response"));
    }

    #[test]
    fn system_prompt_requires_bounded_subagent_delegation() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(prompt.contains("Delegate to subagents deliberately"));
        assert!(prompt.contains("exact result or deliverable expected"));
        assert!(prompt.contains("explicit stopping condition"));
        assert!(prompt.contains("Avoid open-ended delegation"));
    }

    /// The advertised tool list must match what build_agent registers:
    /// web_fetch is always on, web_search only with a Firecrawl key.
    #[test]
    fn system_prompt_advertises_web_tools_to_match_registration() {
        let ws = Workspace::new(PathBuf::from("."));
        let without = system_prompt(&ws, false);
        assert!(without.contains("web_fetch"));
        assert!(!without.contains("web_search"));
        let with = system_prompt(&ws, true);
        assert!(with.contains("web_search"));
    }

    /// `skill` is the one registered tool the prompt must *not* name.
    /// Whether it exists depends on the skill folders and on /skills
    /// toggles, both of which move mid-session, while this string is
    /// built once and re-pushed verbatim by /clear — so a mention here
    /// would eventually advertise a tool that is not registered. The
    /// tool's own description carries the explanation and the catalog.
    #[test]
    fn system_prompt_leaves_skills_to_the_tool_description() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(!prompt.contains("skill"), "{prompt}");
    }

    #[test]
    fn system_prompt_advertises_persistent_compute_and_subagent() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(prompt.contains("pykernel"));
        assert!(prompt.contains("bun_repl"));
        assert!(!prompt.contains("processes), kernel (persistent Python"));
        assert!(prompt.contains("subagent"));
    }

    /// The prompt must keep telling the model to plan ahead of each call and
    /// to keep intermediate state in the persistent compute tool that fits.
    #[test]
    fn system_prompt_instructs_planning_and_persistent_compute_state() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(prompt.contains("Plan before every tool call"));
        assert!(prompt.contains("pykernel for persistent Python state"));
        assert!(prompt.contains("bun_repl for persistent JavaScript"));
        assert!(prompt.contains("fanning out"));
    }

    #[test]
    fn system_prompt_does_not_advertise_removed_task_list() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(!prompt.contains("todo_write"));
    }

    #[test]
    fn system_prompt_advertises_scoped_memory_tools() {
        let prompt = system_prompt(&Workspace::new(PathBuf::from(".")), false);
        assert!(prompt.contains("memory_search"));
        assert!(prompt.contains("global and current-workspace memory"));
        assert!(prompt.contains("memory_manage"));
        assert!(prompt.contains("save, update, or forget durable memory"));
        assert!(prompt.contains(MEMORY_GUIDANCE));
        assert!(prompt.contains("Memory recall is automatic"));
        assert!(prompt.contains("Do not call memory_search merely to repeat"));
        assert!(prompt.contains("or when you identify stable, directly stated user information"));
        assert!(prompt.contains("likely to help in future sessions"));
        assert!(prompt.contains("do not store ordinary conversation, one-off task details"));
        assert!(prompt.contains("duplicates of recalled memory"));
        assert!(prompt.contains("workspace scope (is_global=false)"));
        assert!(prompt.contains("Choose global scope (is_global=true)"));
        assert!(prompt.contains("When approval is enabled"));
        assert!(prompt.contains("approval gate is the user's final decision"));
        assert!(!prompt.contains("only when the user explicitly asks"));
    }

    /// `system_prompt` is a fixed string with tests asserting what is and
    /// is not in it; the user's instructions are appended by the caller,
    /// never folded in, so no AGENTS.md can change what it returns.
    #[test]
    fn instructions_append_to_the_prompt_without_entering_it() {
        let ws = Workspace::new(PathBuf::from("."));
        let base = system_prompt(&ws, false);
        let instructions = instructions::Instructions {
            sources: vec![instructions::Source {
                label: "AGENTS.md".into(),
                body: "never mention skill".into(),
                bytes: 20,
                truncated: false,
            }],
        };
        let mut composed = system_prompt(&ws, false);
        composed.push_str(&instructions.block().unwrap());

        assert!(composed.starts_with(&base), "the base prompt is unchanged");
        assert!(composed.contains("never mention skill"));
        // ...and the function itself still returns the base prompt only.
        assert_eq!(system_prompt(&ws, false), base);
    }

    include!("main_tests/continuation.rs");
}
