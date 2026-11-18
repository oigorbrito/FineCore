//! Parsing for normal interactive and headless runs.

use std::path::PathBuf;

use orca_harness_model_providers::openrouter;

use crate::msg::Provider;
use crate::{config, resolve_theme, select_provider, Config};

const USAGE: &str = "\
orcacode — terminal host for Orca Harness

USAGE:
  orcacode [OPTIONS]                interactive session
  orcacode [OPTIONS] -p \"prompt\"    headless single run (streams to stdout)
  orcacode resume ID [OPTIONS]      resume a recorded session (same as --resume ID)
  orcacode update                   install the latest release
  orcacode plugin <COMMAND>         manage Agent Plugin packages

OPTIONS:
  --model NAME       model id (env ORCA_MODEL; otherwise selected from the
                     provider's live catalog)
  --base-url URL     API root; native Messages API with --anthropic,
                     otherwise OpenAI-compatible (env ORCA_BASE_URL;
                     default: api.openai.com if OPENAI_API_KEY is set,
                     otherwise http://localhost:11434/v1)
  --api-key KEY      API key (provider environment variable, including
                     ANTHROPIC_API_KEY for Anthropic,
                     AI_GATEWAY_API_KEY for Vercel AI Gateway,
                     CHEAPERINFERENCE_API_KEY for CheaperInference)
  --firecrawl-key K  Firecrawl key (env FIRECRAWL_API_KEY); enables the
                     web_search tool
  --openrouter       use OpenRouter (openrouter.ai) as the endpoint
  --anthropic        use the native Anthropic Messages API
  --list-models      print the endpoint's model catalog and exit
  --workspace DIR    tool workspace root (default: current directory)
  --max-steps N      model invocations per run (default 48)
  --max-output-tokens N
                     output-token cap (Anthropic default: 8192)
  --effort LEVEL     reasoning effort (for example low, medium, high)
  --prompt-cache     enable provider prompt-cache hints (default)
  --no-prompt-cache  disable provider prompt-cache hints
  --tools NAMES      headless: comma-separated tool allowlist
  --bare             headless: skip user instructions, skills, memory,
                     MCP, compute, subagents, and web tools; defaults
                     --tools to read_file,grep,glob
  --subagent-depth N subagent nesting levels, positive integer (env ORCA_SUBAGENT_DEPTH;
                     default 1; /subagents adjusts it live in the TUI)
  --theme NAME       theme: default, mono, dracula,
                     solarized-dark, one-dark, monokai, nord, orca
  --plan             start in plan mode: read-only tools, and docs/plan/
                     the only writable directory — the agent decides
                     whether to write a plan (/mode opens a picker)
  --orchestrate      start in orchestrate mode: the agent investigates
                     and delegates substantial work; basic edits permitted;
                     spawned workers keep their full tools
  --normal           start in normal mode (default): gated tools ask for approval
  --auto             start in auto mode: safe tools and guarded
                     file edits run; risk-bearing actions are reviewed
  --yolo             start in yolo mode: every gated tool runs without
                     approval prompts, interactive or headless. The
                     status line reads yolo for the whole session
                     (/mode opens a picker)
  --continue         resume the latest recorded session for this workspace
  --resume ID        resume a recorded session by id (a unique prefix works)
  --no-session       do not record this session to disk
  --json             headless: emit NDJSON harness events on stdout
  --auto-approve     headless: allow shell/write/edit without approval
                     (implied by --yolo)
  -p, --prompt TEXT  headless prompt
  -h, --help         show this help
  -v, -V, --version  print the version and exit

In the TUI, /provider selects local, OpenAI API, OpenRouter, Vercel AI Gateway,
CheaperInference, Anthropic, or the OpenAI Codex ChatGPT-subscription provider.
Selecting openai-codex starts the browser login (device code if port 1455 is busy); an existing
official Codex login is also imported.
No API key is required. /models [filter] opens the model catalog,
and /settings changes the provider, model, theme, and API key.
API keys entered in the TUI, the active provider, the theme, and the
last model picked per provider are saved to
~/.config/orcacode/config.json and reused on later runs; flags and
environment variables above always win over the saved values.
Approval prompts accept y (once), a (always, this session), A (always,
saved for this workspace only; revoke in /settings), and n (deny).
Sessions are recorded under ~/.config/orcacode/sessions/ per workspace;
/sessions in the TUI lists and resumes them, /rewind drops the last
turns, and /fork branches the conversation into a new session.
Standing instructions are read at startup from AGENTS.md beside
config.json, in the workspace, and in the workspace's .orca/ folder.
";

fn usage() -> String {
    USAGE
        .replace(
            "orcacode — terminal host for Orca Harness",
            "EvCore — terminal coding agent",
        )
        .replace("  orcacode ", &format!("  {} ", env!("CARGO_BIN_NAME")))
        .replace(
            "install the latest release",
            "unavailable until an EvCore update channel is configured",
        )
}

pub(crate) fn parse_run_args(args: Vec<String>) -> Result<Config, String> {
    let mut model = std::env::var("ORCA_MODEL").ok();
    let mut base_url = std::env::var("ORCA_BASE_URL").ok();
    let mut api_key: Option<String> = None;
    let mut firecrawl_key: Option<String> = None;
    let mut openrouter = false;
    let mut anthropic = false;
    let mut list_models = false;
    let mut workspace = std::env::current_dir().map_err(|e| e.to_string())?;
    let mut prompt = None;
    let mut json = false;
    let mut auto_approve = false;
    let mut max_steps = 48;
    let mut max_output_tokens = None;
    let mut reasoning_effort = None;
    let mut prompt_cache = true;
    let mut tools = None;
    let mut bare = false;
    let mut subagent_depth: u32 = std::env::var("ORCA_SUBAGENT_DEPTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let mut continue_latest = false;
    let mut resume_id: Option<String> = None;
    let mut no_session = false;
    let mut plan = false;
    let mut orchestrate = false;
    let mut normal = false;
    let mut auto = false;
    let mut yolo = false;
    let mut theme = std::env::var("ORCA_THEME").ok();

    let mut args = args.into_iter().peekable();
    if args.peek().is_some_and(|arg| arg == "resume") {
        args.next();
        resume_id = Some(
            args.next()
                .filter(|id| !id.is_empty() && !id.starts_with('-'))
                .ok_or_else(|| format!("usage: {} resume ID [OPTIONS]", env!("CARGO_BIN_NAME")))?,
        );
    }
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match arg.as_str() {
            "--model" => model = Some(value("--model")?),
            "--base-url" => base_url = Some(value("--base-url")?),
            "--api-key" => api_key = Some(value("--api-key")?),
            "--firecrawl-key" => firecrawl_key = Some(value("--firecrawl-key")?),
            "--openrouter" => openrouter = true,
            "--anthropic" => anthropic = true,
            "--list-models" => list_models = true,
            "--workspace" => workspace = PathBuf::from(value("--workspace")?),
            "--max-steps" => {
                max_steps = value("--max-steps")?
                    .parse()
                    .map_err(|_| "--max-steps expects a number".to_string())?
            }
            "--max-output-tokens" => {
                max_output_tokens = Some(
                    value("--max-output-tokens")?
                        .parse()
                        .map_err(|_| "--max-output-tokens expects a number".to_string())?,
                )
            }
            "--effort" => reasoning_effort = Some(value("--effort")?),
            "--prompt-cache" => prompt_cache = true,
            "--no-prompt-cache" => prompt_cache = false,
            "--tools" => {
                let names = value("--tools")?
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                if names.is_empty() {
                    return Err("--tools expects at least one tool name".into());
                }
                tools = Some(names);
            }
            "--bare" => bare = true,
            "--subagent-depth" => {
                subagent_depth = value("--subagent-depth")?
                    .parse()
                    .map_err(|_| "--subagent-depth expects a number".to_string())?
            }
            "--theme" => theme = Some(value("--theme")?),
            "--continue" => continue_latest = true,
            "--resume" => resume_id = Some(value("--resume")?),
            "--no-session" => no_session = true,
            "--plan" => plan = true,
            "--orchestrate" => orchestrate = true,
            "--normal" => normal = true,
            "--auto" => auto = true,
            "--yolo" => yolo = true,
            "--json" => json = true,
            "--auto-approve" => auto_approve = true,
            "-p" | "--prompt" => prompt = Some(value("-p")?),
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            "-v" | "-V" | "--version" => {
                println!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => return Err(format!("unknown flag: {other}\n\n{}", usage())),
        }
    }

    if openrouter && anthropic {
        return Err("--anthropic and --openrouter cannot be combined".into());
    }
    let stored_provider = if openrouter || anthropic || base_url.is_some() {
        None
    } else {
        config::stored_provider().and_then(|label| Provider::from_label(&label))
    };
    let openrouter = openrouter || stored_provider == Some(Provider::OpenRouter);
    let provider = if anthropic {
        Provider::Anthropic
    } else {
        select_provider(openrouter, base_url.is_some(), stored_provider)
    };

    if max_output_tokens.is_some() && provider == Provider::OpenAiCodex {
        return Err("--max-output-tokens is not supported by openai-codex".into());
    }

    let api_key = api_key.or_else(|| provider.resolve_key());
    let firecrawl_key = firecrawl_key.or_else(|| std::env::var("FIRECRAWL_API_KEY").ok());
    let base_url = base_url.unwrap_or_else(|| match provider {
        Provider::OpenRouter => openrouter::OPENROUTER_BASE_URL.into(),
        Provider::Vercel => Provider::Vercel.base_url().into(),
        Provider::CheaperInference => Provider::CheaperInference.base_url().into(),
        Provider::OpenAiCodex => orca_harness_model_providers::openai_codex::CODEX_BASE_URL.into(),
        Provider::Anthropic => Provider::Anthropic.base_url().into(),
        Provider::OpenAi if api_key.is_some() => "https://api.openai.com/v1".into(),
        Provider::OpenAi | Provider::Local => "http://localhost:11434/v1".into(),
    });
    let model = model
        .or_else(|| config::stored_model(provider.label()))
        .unwrap_or_else(|| provider.default_model().into());
    let theme = resolve_theme(theme);

    if prompt.is_none() && (bare || tools.is_some()) {
        return Err("--bare and --tools require headless -p/--prompt".into());
    }

    Ok(Config {
        provider,
        model,
        base_url,
        api_key,
        firecrawl_key,
        openrouter,
        list_models,
        workspace,
        prompt,
        json,
        auto_approve,
        max_steps,
        max_output_tokens,
        reasoning_effort,
        prompt_cache,
        tools,
        bare,
        subagent_depth,
        continue_latest,
        resume_id,
        no_session,
        theme,
        plan,
        orchestrate,
        normal,
        auto,
        yolo,
    })
}
