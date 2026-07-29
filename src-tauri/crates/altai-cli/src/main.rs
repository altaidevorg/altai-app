use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod host_adapter;
mod journal_sink;
mod run_output;

#[derive(Debug, Parser)]
#[command(
    name = "altai-cli",
    version,
    about = "ALTAI terminal product",
    long_about = "The ALTAI terminal product. Use `altai agent` for the interactive TUI and `altai run` for one-shot headless sessions."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start the interactive ALTAI terminal experience.
    Agent(AgentArgs),
    /// Run one prompt without starting an interactive terminal session.
    Run(RunArgs),
    /// Print release and dependency information.
    Version {
        /// Include terminal-contract metadata.
        #[arg(long)]
        verbose: bool,
    },
    /// Generate shell completion definitions.
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Inspect the local terminal-product foundation.
    Doctor {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Inspect configuration locations for an ALTAI workspace.
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Inspect the model selected for an ALTAI workspace.
    Models {
        #[command(subcommand)]
        command: ModelsCommands,
    },
    /// Launch ALTAI Desktop. This router is safe to exercise with --dry-run.
    Open(OpenArgs),
    /// Inspect the durable agent event journal shared with ALTAI Desktop.
    Journal {
        #[command(subcommand)]
        command: JournalCommands,
    },
}

#[derive(Debug, Subcommand)]
enum JournalCommands {
    /// List incomplete runs and, optionally, the latest run for one chat.
    Summary(JournalSummaryArgs),
    /// Fetch journal events for one run after a sequence number.
    Fetch(JournalFetchArgs),
}

#[derive(Debug, Args)]
struct JournalSummaryArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Restrict the latest-run lookup to one chat.
    #[arg(long)]
    chat: Option<String>,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct JournalFetchArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Run identifier to fetch events for.
    #[arg(long)]
    run: String,
    /// Only return events with sequence greater than this value.
    #[arg(long, default_value_t = 0)]
    after: u64,
    /// Maximum number of events to return.
    #[arg(long, default_value_t = 200)]
    limit: usize,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
enum PermissionMode {
    /// Prompt before protected shell commands and file edits.
    Ask,
    /// Apply file edits automatically while retaining protected shell prompts.
    AutoEdit,
    /// Read-only planning mode.
    Plan,
    /// Use the guarded bypass policy. This always requires an explicit flag.
    Bypass,
}

impl PermissionMode {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::AutoEdit => "auto-edit",
            Self::Plan => "plan",
            Self::Bypass => "bypass",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum ThemeMode {
    /// Select an ALTAI terminal theme from terminal capabilities.
    Auto,
    /// Use the ALTAI near-black IDE theme.
    Dark,
    /// Use the ALTAI light theme.
    Light,
    /// Preserve terminal structure without ANSI foreground colors.
    NoColor,
}

impl ThemeMode {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
            Self::NoColor => "no-color",
        }
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
enum OutputMode {
    /// Interactive human-oriented output.
    Pretty,
    /// Line-oriented human output.
    Plain,
    /// Print only the terminal assistant result.
    Final,
    /// Emit one structured JSON object for the final result.
    Json,
    /// Emit the versioned ALTAI JSONL event stream.
    Jsonl,
}

impl OutputMode {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Pretty => "pretty",
            Self::Plain => "plain",
            Self::Final => "final",
            Self::Json => "json",
            Self::Jsonl => "jsonl",
        }
    }
}

#[derive(Debug, Args)]
struct AgentOptions {
    /// Explicit workspace root. Defaults to PATH or the current directory.
    #[arg(short, long)]
    workspace: Option<PathBuf>,
    /// Provider/model identifier such as anthropic/claude-sonnet-4-6.
    #[arg(long)]
    model: Option<String>,
    /// Fallback provider/model identifier.
    #[arg(long)]
    fallback_model: Option<String>,
    /// Permission behavior for this process.
    #[arg(long, value_enum)]
    permission: Option<PermissionMode>,
    /// Terminal theme selection.
    #[arg(long, value_enum, default_value_t = ThemeMode::Auto)]
    theme: ThemeMode,
    /// Resume this durable ALTAI chat.
    #[arg(long)]
    resume: Option<String>,
    /// Attach a local file to the next prompt. May be repeated.
    #[arg(long = "file")]
    files: Vec<PathBuf>,
    /// Disable between-turn auto-compaction (manual `/compact` still works).
    #[arg(long)]
    no_auto_compact: bool,
    /// Token threshold that triggers auto-compaction when auto is enabled.
    #[arg(long)]
    compact_threshold: Option<usize>,
    /// Number of recent summaries / tail turns retained after compaction.
    #[arg(long)]
    compact_tail: Option<usize>,
}

#[derive(Debug, Args)]
struct AgentArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Use an accessible line-oriented REPL instead of the IsanAgent TUI.
    #[arg(long)]
    no_tui: bool,
    /// Describe the resolved terminal session without starting the host.
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    options: AgentOptions,
}

#[derive(Debug, Args)]
struct RunArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Prompt text. Use `-` to read the prompt from standard input.
    #[arg(short, long)]
    prompt: String,
    /// Output contract for the foreground run.
    #[arg(long, value_enum, default_value_t = OutputMode::Pretty)]
    output: OutputMode,
    /// Alias for `--output jsonl`.
    #[arg(long, conflicts_with = "output")]
    json: bool,
    /// Complete foreground-run timeout, for example `10m`.
    #[arg(long)]
    timeout: Option<String>,
    /// Suppress non-error diagnostic output.
    #[arg(long)]
    quiet: bool,
    /// Describe the resolved foreground run without starting the host.
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    options: AgentOptions,
}

#[derive(Debug, Args)]
struct OpenArgs {
    /// File or folder to open in ALTAI Desktop.
    path: Option<PathBuf>,
    /// Describe the desktop command without spawning it.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Subcommand)]
enum ConfigCommands {
    /// Print the project-local ALTAI and IsanAgent configuration paths.
    Path(ConfigPathArgs),
    /// Resolve non-secret agent settings and optionally report their origins.
    List(ConfigListArgs),
}

#[derive(Debug, Args)]
struct ConfigPathArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ConfigListArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Resolve the effective settings using ALTAI's documented precedence.
    #[arg(long)]
    resolved: bool,
    /// Include the source that supplied each effective setting.
    #[arg(long, requires = "resolved")]
    show_origin: bool,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ModelsCommands {
    /// Print the resolved primary and fallback model selections.
    Current(ModelsCurrentArgs),
}

#[derive(Debug, Args)]
struct ModelsCurrentArgs {
    /// Workspace path. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Include the source that supplied each selection.
    #[arg(long)]
    show_origin: bool,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug)]
enum CliError {
    Message(String),
    #[allow(dead_code)] // Reserved for commands that still lack a host integration.
    HostUnavailable { command: &'static str },
    RunFailed {
        code: run_output::RunExitCode,
        message: String,
    },
}

impl CliError {
    fn exit_code(&self) -> u8 {
        match self {
            // Exit code 10 is reserved by the public contract for an internal
            // error. A missing host integration must never masquerade as an
            // approval, provider, or workspace failure.
            Self::HostUnavailable { .. } => 10,
            Self::RunFailed { code, .. } => (*code).into(),
            Self::Message(_) => 1,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::HostUnavailable { command } => write!(
                f,
                "`altai-cli {command}` is declared but cannot run yet: the ALTAI adapter needs the reusable IsanAgent host API. Use `altai-cli doctor` to inspect the installed foundation."
            ),
            Self::RunFailed { message, .. } => f.write_str(message),
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("altai-cli: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            Cli::command()
                .print_help()
                .map_err(|error| CliError::Message(error.to_string()))?;
            println!();
            Ok(())
        }
        Some(Commands::Agent(args)) => agent(args),
        Some(Commands::Run(args)) => run_prompt(args),
        Some(Commands::Version { verbose }) => print_version(verbose),
        Some(Commands::Completion { shell }) => {
            let mut command = Cli::command();
            let name = command.get_name().to_string();
            generate(shell, &mut command, name, &mut io::stdout());
            Ok(())
        }
        Some(Commands::Doctor { json }) => doctor(json),
        Some(Commands::Config { command }) => config(command),
        Some(Commands::Models { command }) => models(command),
        Some(Commands::Open(args)) => open_desktop(args),
        Some(Commands::Journal { command }) => journal(command),
    }
}

fn journal(command: JournalCommands) -> Result<(), CliError> {
    match command {
        JournalCommands::Summary(args) => journal_summary(args),
        JournalCommands::Fetch(args) => journal_fetch(args),
    }
}

fn open_workspace_journal(
    path: Option<&Path>,
) -> Result<(altai_core::WorkspacePaths, altai_core::EventJournal), CliError> {
    let workspace = altai_core::resolve_workspace(path)
        .map_err(|error| CliError::Message(error.to_string()))?;
    let journal = altai_core::EventJournal::open(workspace.agent_event_journal_db())
        .map_err(|error| CliError::Message(format!("could not open event journal: {error}")))?;
    Ok((workspace, journal))
}

fn journal_summary(args: JournalSummaryArgs) -> Result<(), CliError> {
    let (workspace, journal) = open_workspace_journal(args.path.as_deref())?;
    let incomplete = journal
        .incomplete_run_summaries()
        .map_err(|error| CliError::Message(error.to_string()))?;
    let latest = match &args.chat {
        Some(chat_id) => journal
            .latest_run_summary_for_chat(chat_id)
            .map_err(|error| CliError::Message(error.to_string()))?,
        None => None,
    };

    if args.json {
        return print_preview(serde_json::json!({
            "workspace": workspace.root,
            "incomplete_runs": incomplete.iter().map(run_summary_json).collect::<Vec<_>>(),
            "latest_run": latest.as_ref().map(run_summary_json),
        }));
    }

    println!("Workspace: {}", workspace.root.display());
    if incomplete.is_empty() {
        println!("Incomplete runs: none");
    } else {
        println!("Incomplete runs:");
        for summary in &incomplete {
            println!(
                "  {} (chat {}, last_seq {})",
                summary.run_id, summary.chat_id, summary.last_seq
            );
        }
    }
    if let Some(chat_id) = &args.chat {
        match latest {
            Some(summary) => println!(
                "Latest run for {chat_id}: {} (last_seq {}, terminal {})",
                summary.run_id,
                summary.last_seq,
                summary.terminal_kind.as_deref().unwrap_or("pending")
            ),
            None => println!("Latest run for {chat_id}: none"),
        }
    }
    Ok(())
}

fn journal_fetch(args: JournalFetchArgs) -> Result<(), CliError> {
    let (_workspace, journal) = open_workspace_journal(args.path.as_deref())?;
    let events = journal
        .fetch_after(&args.run, args.after, args.limit)
        .map_err(|error| CliError::Message(error.to_string()))?;

    if args.json {
        return print_preview(serde_json::json!({
            "run_id": args.run,
            "events": events.iter().map(journal_event_json).collect::<Vec<_>>(),
        }));
    }

    for event in &events {
        println!(
            "{:>6}  {}  {}",
            event.seq,
            event.kind,
            serde_json::to_string(&event.payload).unwrap_or_default()
        );
    }
    Ok(())
}

fn run_summary_json(summary: &altai_core::RunJournalSummary) -> serde_json::Value {
    serde_json::json!({
        "run_id": summary.run_id,
        "chat_id": summary.chat_id,
        "last_seq": summary.last_seq,
        "terminal_seq": summary.terminal_seq,
        "terminal_kind": summary.terminal_kind,
        "terminal_payload": summary.terminal_payload,
    })
}

fn journal_event_json(event: &altai_core::JournalEvent) -> serde_json::Value {
    serde_json::json!({
        "version": event.version,
        "run_id": event.run_id,
        "seq": event.seq,
        "chat_id": event.chat_id,
        "recorded_at_ms": event.recorded_at_ms,
        "kind": event.kind,
        "payload": event.payload,
    })
}

fn models(command: ModelsCommands) -> Result<(), CliError> {
    match command {
        ModelsCommands::Current(args) => models_current(args),
    }
}

fn models_current(args: ModelsCurrentArgs) -> Result<(), CliError> {
    let workspace = altai_core::resolve_workspace(args.path.as_deref())
        .map_err(|error| CliError::Message(error.to_string()))?;
    let resolved = load_workspace_agent_config(&workspace)?;

    if args.json {
        return print_preview(serde_json::json!({
            "workspace": workspace.root,
            "model": config_value(resolved.model.as_ref(), args.show_origin),
            "fallback_model": config_value(resolved.fallback_model.as_ref(), args.show_origin),
        }));
    }

    print_config_field("model", resolved.model.as_ref(), args.show_origin);
    print_config_field(
        "fallback_model",
        resolved.fallback_model.as_ref(),
        args.show_origin,
    );
    Ok(())
}

fn config(command: ConfigCommands) -> Result<(), CliError> {
    match command {
        ConfigCommands::Path(args) => config_path(args),
        ConfigCommands::List(args) => config_list(args),
    }
}

fn config_list(args: ConfigListArgs) -> Result<(), CliError> {
    if !args.resolved {
        return Err(CliError::Message(
            "only resolved configuration is available in this build; pass --resolved".into(),
        ));
    }

    let workspace = altai_core::resolve_workspace(args.path.as_deref())
        .map_err(|error| CliError::Message(error.to_string()))?;
    let resolved = load_workspace_agent_config(&workspace)?;
    let values = [
        ("model", resolved.model.as_ref()),
        ("fallback_model", resolved.fallback_model.as_ref()),
        ("provider", resolved.provider.as_ref()),
        ("base_url", resolved.base_url.as_ref()),
    ];

    if args.json {
        let values = values
            .into_iter()
            .map(|(name, value)| (name.to_string(), config_value(value, args.show_origin)))
            .collect::<serde_json::Map<String, serde_json::Value>>();
        return print_preview(serde_json::json!({
            "workspace": workspace.root,
            "values": values,
        }));
    }

    for (name, value) in values {
        print_config_field(name, value, args.show_origin);
    }
    Ok(())
}

fn load_workspace_agent_config(
    workspace: &altai_core::WorkspacePaths,
) -> Result<altai_core::ResolvedAgentConfig, CliError> {
    altai_core::load_agent_config(
        &workspace.root.join(".altai/config.toml"),
        &workspace.isanagent_state.join("config.toml"),
    )
    .map_err(|error| CliError::Message(error.to_string()))
}

fn print_config_field(
    name: &str,
    value: Option<&altai_core::ResolvedConfig<String>>,
    show_origin: bool,
) {
    match (value, show_origin) {
        (Some(value), true) => println!("{name}: {} ({})", value.value, value.source.label()),
        (Some(value), false) => println!("{name}: {}", value.value),
        (None, true) => println!("{name}: <unset> (default)"),
        (None, false) => println!("{name}: <unset>"),
    }
}

fn config_value(
    value: Option<&altai_core::ResolvedConfig<String>>,
    show_origin: bool,
) -> serde_json::Value {
    match (value, show_origin) {
        (Some(value), true) => serde_json::json!({
            "value": value.value.clone(),
            "source": value.source.label(),
        }),
        (Some(value), false) => serde_json::Value::String(value.value.clone()),
        (None, true) => serde_json::json!({ "value": null, "source": "default" }),
        (None, false) => serde_json::Value::Null,
    }
}

fn config_path(args: ConfigPathArgs) -> Result<(), CliError> {
    let workspace = altai_core::resolve_workspace(args.path.as_deref())
        .map_err(|error| CliError::Message(error.to_string()))?;
    let altai_config = workspace.root.join(".altai/config.toml");
    let isanagent_config = workspace.isanagent_state.join("config.toml");
    let workspace_display = workspace.root.display().to_string();
    let altai_config_display = altai_config.display().to_string();
    let isanagent_config_display = isanagent_config.display().to_string();
    let value = serde_json::json!({
        "workspace": workspace.root,
        "altai_config": altai_config,
        "isanagent_config": isanagent_config,
    });

    if args.json {
        return print_preview(value);
    }

    println!("Workspace: {workspace_display}");
    println!("ALTAI config: {altai_config_display}");
    println!("IsanAgent config: {isanagent_config_display}");
    Ok(())
}

fn agent(args: AgentArgs) -> Result<(), CliError> {
    let workspace =
        resolve_command_workspace(args.options.workspace.as_deref(), args.path.as_deref())?;
    let appearance = resolve_cli_theme(args.options.theme);
    let mut host = host_adapter::host_config_for_workspace(&workspace);
    host.model = args.options.model.clone();
    host.fallback_model = args.options.fallback_model.clone();
    host.permission = args.options.permission.as_ref().map(host_permission_mode);
    host.no_color = appearance == altai_core::EffectiveTerminalAppearance::NoColor;
    host.theme = host_theme_mode(appearance);
    host.resume = args.options.resume.clone();
    host.files = args.options.files.clone();
    host.line_mode = args.no_tui;
    apply_compaction_overrides(&mut host, &args.options);

    if !args.dry_run {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                CliError::Message(format!("could not start the host runtime: {error}"))
            })?;
        return runtime
            .block_on(isanagent::host::start_host(host))
            .map_err(|error| {
                CliError::Message(format!("IsanAgent host exited with an error: {error}"))
            });
    }

    let value = serde_json::json!({
        "kind": "agent",
        "workspace": workspace.root,
        "isanagent_state": workspace.isanagent_state,
        "host": {
            "state": host.workspace,
            "config": host.config,
            "sandbox": host.sandbox,
        },
        "tui": !args.no_tui,
        "model": args.options.model,
        "fallback_model": args.options.fallback_model,
        "permission": args.options.permission.as_ref().map(PermissionMode::as_str),
        "theme": args.options.theme.as_str(),
        "effective_theme": appearance.as_str(),
        "resume": args.options.resume,
        "files": args.options.files,
        "compaction": resolved_compaction_preview(&args.options),
    });
    print_preview(value)
}

fn apply_compaction_overrides(host: &mut isanagent::host::HostConfig, options: &AgentOptions) {
    apply_compaction_fields(
        host,
        options.no_auto_compact,
        options.compact_threshold,
        options.compact_tail,
    );
}

fn apply_compaction_fields(
    host: &mut isanagent::host::HostConfig,
    no_auto_compact: bool,
    compact_threshold: Option<usize>,
    compact_tail: Option<usize>,
) {
    let prefs = altai_core::resolve_compaction_prefs(altai_core::CompactionOverrides {
        auto: if no_auto_compact { Some(false) } else { None },
        threshold_tokens: compact_threshold,
        tail_turns: compact_tail,
    });
    let logic = prefs.to_logic_params();
    host.compact_auto = Some(prefs.auto);
    host.compact_threshold_tokens = Some(logic.short_term_threshold_tokens);
    host.compact_tail_turns = Some(logic.max_recent_summaries);
}

fn resolved_compaction_preview(options: &AgentOptions) -> serde_json::Value {
    let prefs = altai_core::resolve_compaction_prefs(altai_core::CompactionOverrides {
        auto: if options.no_auto_compact {
            Some(false)
        } else {
            None
        },
        threshold_tokens: options.compact_threshold,
        tail_turns: options.compact_tail,
    });
    let logic = prefs.to_logic_params();
    serde_json::json!({
        "auto": prefs.auto,
        "threshold_tokens": prefs.threshold_tokens,
        "tail_turns": prefs.tail_turns,
        "logic": {
            "max_recent_summaries": logic.max_recent_summaries,
            "short_term_threshold_turns": logic.short_term_threshold_turns,
            "short_term_threshold_tokens": logic.short_term_threshold_tokens,
        }
    })
}

fn resolve_cli_theme(theme: ThemeMode) -> altai_core::EffectiveTerminalAppearance {
    let cli = match theme {
        ThemeMode::Auto => altai_core::TerminalThemeMode::Auto,
        ThemeMode::Dark => altai_core::TerminalThemeMode::Dark,
        ThemeMode::Light => altai_core::TerminalThemeMode::Light,
        ThemeMode::NoColor => altai_core::TerminalThemeMode::NoColor,
    };
    altai_core::resolve_terminal_appearance_from_env(cli)
}

const fn host_theme_mode(
    appearance: altai_core::EffectiveTerminalAppearance,
) -> isanagent::host::HostThemeMode {
    match appearance {
        altai_core::EffectiveTerminalAppearance::Dark => isanagent::host::HostThemeMode::Dark,
        altai_core::EffectiveTerminalAppearance::Light => isanagent::host::HostThemeMode::Light,
        altai_core::EffectiveTerminalAppearance::NoColor => isanagent::host::HostThemeMode::NoColor,
    }
}

const fn host_permission_mode(permission: &PermissionMode) -> isanagent::host::HostPermissionMode {
    match permission {
        PermissionMode::Ask => isanagent::host::HostPermissionMode::Ask,
        PermissionMode::AutoEdit => isanagent::host::HostPermissionMode::AutoEdit,
        PermissionMode::Plan => isanagent::host::HostPermissionMode::Plan,
        PermissionMode::Bypass => isanagent::host::HostPermissionMode::Bypass,
    }
}

fn run_prompt(args: RunArgs) -> Result<(), CliError> {
    let output = if args.json {
        OutputMode::Jsonl
    } else {
        args.output.clone()
    };
    let prompt = resolve_prompt(&args.prompt)?;
    let workspace =
        resolve_command_workspace(args.options.workspace.as_deref(), args.path.as_deref())?;

    if args.dry_run {
        let value = serde_json::json!({
            "kind": "run",
            "workspace": workspace.root,
            "isanagent_state": workspace.isanagent_state,
            "prompt": prompt,
            "output": output.as_str(),
            "timeout": args.timeout,
            "quiet": args.quiet,
            "model": args.options.model,
            "fallback_model": args.options.fallback_model,
            "permission": args.options.permission.as_ref().map(PermissionMode::as_str),
            "theme": args.options.theme.as_str(),
            "resume": args.options.resume,
            "files": args.options.files,
            "compaction": resolved_compaction_preview(&args.options),
        });
        return print_preview(value);
    }

    let permission = resolve_run_permission(args.options.permission.clone())?;
    let timeout = args
        .timeout
        .as_deref()
        .map(run_output::parse_timeout)
        .transpose()
        .map_err(CliError::Message)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            CliError::Message(format!("could not start the host runtime: {error}"))
        })?;

    runtime.block_on(async_run_prompt(AsyncRunRequest {
        workspace,
        prompt,
        output,
        timeout,
        quiet: args.quiet,
        model: args.options.model,
        fallback_model: args.options.fallback_model,
        permission,
        no_color: args.options.theme == ThemeMode::NoColor
            || std::env::var_os("NO_COLOR").is_some(),
        resume: args.options.resume,
        files: args.options.files,
        no_auto_compact: args.options.no_auto_compact,
        compact_threshold: args.options.compact_threshold,
        compact_tail: args.options.compact_tail,
    }))
}

struct AsyncRunRequest {
    workspace: altai_core::WorkspacePaths,
    prompt: String,
    output: OutputMode,
    timeout: Option<std::time::Duration>,
    quiet: bool,
    model: Option<String>,
    fallback_model: Option<String>,
    permission: PermissionMode,
    no_color: bool,
    resume: Option<String>,
    files: Vec<PathBuf>,
    no_auto_compact: bool,
    compact_threshold: Option<usize>,
    compact_tail: Option<usize>,
}

async fn async_run_prompt(request: AsyncRunRequest) -> Result<(), CliError> {
    use isanagent::host::{OneshotOutcome, OneshotResult};
    use std::io::{self, Write};

    let (observe_tx, mut observe_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut host = host_adapter::oneshot_host_config(
        &request.workspace,
        request.prompt.clone(),
        Some(observe_tx),
    );
    host.model = request.model.clone();
    host.fallback_model = request.fallback_model.clone();
    host.permission = Some(host_permission_mode(&request.permission));
    host.no_color = request.no_color;
    host.resume = request.resume.clone();
    host.files = request.files.clone();
    apply_compaction_fields(
        &mut host,
        request.no_auto_compact,
        request.compact_threshold,
        request.compact_tail,
    );

    let workspace_display = request.workspace.root.display().to_string();
    let output_mode = request.output.clone();
    let quiet = request.quiet;
    let journal_sink = std::sync::Arc::new(tokio::sync::Mutex::new(
        journal_sink::JournalSink::open(&request.workspace),
    ));
    let journal_sink_for_observer = journal_sink.clone();
    let observer = tokio::spawn(async move {
        let mut emitter = run_output::JsonlEmitter::new(workspace_display);
        let mut stdout = io::stdout();
        let mut stderr = io::stderr();
        while let Some(message) = observe_rx.recv().await {
            if let Some(sink) = journal_sink_for_observer.lock().await.as_mut() {
                sink.observe_bus_message(&message);
            }
            match output_mode {
                OutputMode::Jsonl => {
                    if let Err(error) = emitter.observe_bus_message(&message, &mut stdout) {
                        let _ = writeln!(stderr, "altai-cli: failed to emit JSONL: {error}");
                    }
                }
                OutputMode::Pretty | OutputMode::Plain if !quiet => {
                    if let isanagent::bus::BusMessage::Telemetry(
                        isanagent::bus::TelemetryEvent::ToolCallStarted { tool_name, .. },
                    ) = &message
                    {
                        let _ = writeln!(stderr, "tool: {tool_name}");
                    }
                }
                _ => {}
            }
        }
    });

    let mut oneshot_task = tokio::spawn(isanagent::host::run_oneshot(host));
    let result = tokio::select! {
        joined = &mut oneshot_task => {
            match joined {
                Ok(Ok(result)) => result,
                Ok(Err(error)) => {
                    return Err(CliError::RunFailed {
                        code: run_output::RunExitCode::Internal,
                        message: format!("IsanAgent oneshot host failed: {error}"),
                    });
                }
                Err(error) => {
                    return Err(CliError::RunFailed {
                        code: run_output::RunExitCode::Internal,
                        message: format!("IsanAgent oneshot task failed: {error}"),
                    });
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            oneshot_task.abort();
            OneshotResult {
                chat_id: request.resume.clone().unwrap_or_default(),
                run_id: None,
                outcome: OneshotOutcome::Cancelled,
                final_text: None,
            }
        }
        _ = async {
            match request.timeout {
                Some(duration) => tokio::time::sleep(duration).await,
                None => std::future::pending().await,
            }
        } => {
            oneshot_task.abort();
            OneshotResult {
                chat_id: request.resume.clone().unwrap_or_default(),
                run_id: None,
                outcome: OneshotOutcome::TimedOut,
                final_text: None,
            }
        }
    };

    let _ = tokio::time::timeout(std::time::Duration::from_millis(250), observer).await;

    if let Some(sink) = journal_sink.lock().await.as_mut() {
        sink.finalize(&result);
    }

    let final_result =
        run_output::FinalRunResult::from_oneshot(&request.workspace.root.display().to_string(), &result);
    match request.output {
        OutputMode::Jsonl => {
            let mut emitter =
                run_output::JsonlEmitter::new(request.workspace.root.display().to_string());
            emitter
                .emit_final_result(&final_result, &mut io::stdout())
                .map_err(|error| CliError::Message(error.to_string()))?;
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&final_result)
                    .map_err(|error| CliError::Message(error.to_string()))?
            );
        }
        OutputMode::Final | OutputMode::Plain | OutputMode::Pretty => {
            run_output::render_pretty(&final_result, &mut io::stdout())
                .map_err(|error| CliError::Message(error.to_string()))?;
        }
    }

    let code = run_output::RunExitCode::from_oneshot_outcome(&result.outcome);
    if matches!(code, run_output::RunExitCode::Success) {
        Ok(())
    } else {
        Err(CliError::RunFailed {
            code,
            message: final_result
                .detail
                .unwrap_or_else(|| format!("run ended with {}", final_result.outcome)),
        })
    }
}

fn resolve_prompt(prompt: &str) -> Result<String, CliError> {
    if prompt == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|error| CliError::Message(format!("could not read prompt from stdin: {error}")))?;
        let trimmed = buffer.trim_end().to_string();
        if trimmed.is_empty() {
            return Err(CliError::Message("stdin prompt was empty".into()));
        }
        return Ok(trimmed);
    }
    if prompt.trim().is_empty() {
        return Err(CliError::Message("prompt must not be empty".into()));
    }
    Ok(prompt.to_string())
}

fn resolve_run_permission(explicit: Option<PermissionMode>) -> Result<PermissionMode, CliError> {
    use std::io::IsTerminal;
    if let Some(permission) = explicit {
        if matches!(permission, PermissionMode::Ask | PermissionMode::AutoEdit)
            && !std::io::stdin().is_terminal()
        {
            // Still allow the run: the oneshot host rejects interactive approvals
            // with exit code 4 instead of silently approving.
            return Ok(permission);
        }
        return Ok(permission);
    }
    if std::io::stdin().is_terminal() {
        Ok(PermissionMode::Ask)
    } else {
        Ok(PermissionMode::Plan)
    }
}

fn resolve_command_workspace(
    explicit_workspace: Option<&Path>,
    positional_path: Option<&Path>,
) -> Result<altai_core::WorkspacePaths, CliError> {
    altai_core::resolve_workspace(explicit_workspace.or(positional_path))
        .map_err(|error| CliError::Message(error.to_string()))
}

fn print_preview(value: serde_json::Value) -> Result<(), CliError> {
    println!(
        "{}",
        serde_json::to_string_pretty(&value)
            .map_err(|error| CliError::Message(error.to_string()))?
    );
    Ok(())
}

fn print_version(verbose: bool) -> Result<(), CliError> {
    if verbose {
        let value = serde_json::json!({
            "product": "ALTAI CLI",
            "version": env!("CARGO_PKG_VERSION"),
            "event_schema_version": altai_core::EVENT_SCHEMA_VERSION,
            "terminal_palette_schema_version": altai_core::load_terminal_palette()
                .map_err(|error| CliError::Message(error.to_string()))?
                .schema_version,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value)
                .map_err(|error| CliError::Message(error.to_string()))?
        );
    } else {
        println!("altai-cli {}", env!("CARGO_PKG_VERSION"));
    }
    Ok(())
}

fn doctor(json: bool) -> Result<(), CliError> {
    let palette = altai_core::load_terminal_palette()
        .map_err(|error| CliError::Message(error.to_string()))?;
    let value = serde_json::json!({
        "ok": true,
        "event_schema_version": altai_core::EVENT_SCHEMA_VERSION,
        "palette": {
            "source": palette.source,
            "schema_version": palette.schema_version,
            "modes": palette.modes.keys().collect::<Vec<_>>(),
            "fallbacks": palette.fallbacks,
        },
    });

    if json {
        println!(
            "{}",
            serde_json::to_string(&value).map_err(|error| CliError::Message(error.to_string()))?
        );
    } else {
        println!("ALTAI CLI foundation: ready");
        println!("Event schema: v{}", altai_core::EVENT_SCHEMA_VERSION);
        println!("Palette source: {}", value["palette"]["source"]);
        println!("Theme modes: dark, light, no-color");
    }
    Ok(())
}

fn open_desktop(args: OpenArgs) -> Result<(), CliError> {
    let current_exe =
        std::env::current_exe().map_err(|error| CliError::Message(error.to_string()))?;
    let desktop = desktop_executable_path(&current_exe);
    let mut desktop_args = Vec::new();
    if let Some(path) = args.path {
        desktop_args.push(path.to_string_lossy().to_string());
    }

    if args.dry_run {
        let value = serde_json::json!({
            "desktop_executable": desktop,
            "args": desktop_args,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value)
                .map_err(|error| CliError::Message(error.to_string()))?
        );
        return Ok(());
    }

    if !desktop.exists() {
        return Err(CliError::Message(format!(
            "ALTAI Desktop was not found at {}. Use --dry-run to inspect the expected route.",
            desktop.display()
        )));
    }

    Command::new(&desktop)
        .args(&desktop_args)
        .spawn()
        .map_err(|error| {
            CliError::Message(format!("could not launch {}: {error}", desktop.display()))
        })?;
    Ok(())
}

fn desktop_executable_path(cli_executable: &Path) -> PathBuf {
    let file_name = if cfg!(windows) {
        "altai-desktop.exe"
    } else {
        "altai-desktop"
    };
    cli_executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_router_uses_platform_binary_name() {
        let path = desktop_executable_path(Path::new("/opt/altai/altai-cli"));
        assert_eq!(
            path.file_name().and_then(|value| value.to_str()),
            Some(if cfg!(windows) {
                "altai-desktop.exe"
            } else {
                "altai-desktop"
            })
        );
    }

    #[test]
    fn agent_contract_parses_tui_options() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "agent",
            ".",
            "--model",
            "anthropic/claude-sonnet-4-6",
            "--permission",
            "auto-edit",
            "--theme",
            "dark",
            "--resume",
            "chat-1",
            "--dry-run",
        ])
        .expect("agent contract should parse");

        let Some(Commands::Agent(args)) = cli.command else {
            panic!("agent command should parse");
        };
        assert_eq!(args.options.permission, Some(PermissionMode::AutoEdit));
        assert_eq!(args.options.theme, ThemeMode::Dark);
        assert_eq!(args.options.resume.as_deref(), Some("chat-1"));
        assert!(args.dry_run);
        assert_eq!(
            altai_core::resolve_terminal_appearance(
                altai_core::TerminalThemeMode::Dark,
                false,
                None,
                None
            ),
            altai_core::EffectiveTerminalAppearance::Dark
        );
        assert_eq!(
            host_theme_mode(altai_core::EffectiveTerminalAppearance::Dark),
            isanagent::host::HostThemeMode::Dark
        );
    }

    #[test]
    fn plain_agent_contract_can_start_the_embedded_tui() {
        let cli = Cli::try_parse_from(["altai-cli", "agent", "."])
            .expect("plain agent command should parse");

        let Some(Commands::Agent(args)) = cli.command else {
            panic!("agent command should parse");
        };
        assert!(!args.dry_run);
        assert_eq!(args.options.theme, ThemeMode::Auto);
    }

    #[test]
    fn no_color_env_wins_over_dark_theme() {
        // resolve_terminal_appearance is pure; NO_COLOR is simulated via the helper.
        assert_eq!(
            altai_core::resolve_terminal_appearance(
                altai_core::TerminalThemeMode::Dark,
                true,
                None,
                None
            ),
            altai_core::EffectiveTerminalAppearance::NoColor
        );
    }

    #[test]
    fn run_contract_parses_compaction_flags() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "run",
            ".",
            "--prompt",
            "hi",
            "--no-auto-compact",
            "--compact-threshold",
            "50000",
            "--compact-tail",
            "8",
            "--dry-run",
        ])
        .expect("compaction flags should parse");

        let Some(Commands::Run(args)) = cli.command else {
            panic!("run command should parse");
        };
        assert!(args.options.no_auto_compact);
        assert_eq!(args.options.compact_threshold, Some(50_000));
        assert_eq!(args.options.compact_tail, Some(8));

        let preview = resolved_compaction_preview(&args.options);
        assert_eq!(preview["auto"], false);
        assert_eq!(preview["threshold_tokens"], 50_000);
        assert_eq!(preview["tail_turns"], 8);
        assert_eq!(
            preview["logic"]["short_term_threshold_tokens"],
            serde_json::json!(usize::MAX)
        );
        assert_eq!(preview["logic"]["max_recent_summaries"], 8);
    }

    #[test]
    fn run_contract_parses_jsonl_output_and_stdin_prompt() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "run",
            ".",
            "--prompt",
            "-",
            "--output",
            "jsonl",
            "--file",
            "README.md",
            "--dry-run",
        ])
        .expect("run contract should parse");

        let Some(Commands::Run(args)) = cli.command else {
            panic!("run command should parse");
        };
        assert_eq!(args.prompt, "-");
        assert_eq!(args.output, OutputMode::Jsonl);
        assert_eq!(args.options.files, vec![PathBuf::from("README.md")]);
        assert!(args.dry_run);
    }

    #[test]
    fn run_json_flag_aliases_jsonl_output() {
        let cli = Cli::try_parse_from(["altai-cli", "run", ".", "--prompt", "hi", "--json"])
            .expect("run --json should parse");
        let Some(Commands::Run(args)) = cli.command else {
            panic!("run command should parse");
        };
        assert!(args.json);
    }

    #[tokio::test]
    async fn oneshot_smoke_completes_with_scripted_provider() {
        use std::fs;

        let tmp_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        fs::create_dir_all(&tmp_root).expect("tmp root");
        let temp = tempfile::TempDir::new_in(&tmp_root).expect("tempdir");
        let root = temp.path();
        let state = root.join(".isanagent");
        fs::create_dir_all(state.join(".system_generated")).expect("state dir");
        fs::write(
            state.join("config.toml"),
            "[terminal]\nenabled = false\n\n[logging]\nenabled = false\n",
        )
        .expect("config");

        let workspace = altai_core::WorkspacePaths {
            root: root.to_path_buf(),
            isanagent_state: state,
        };
        let mut host = host_adapter::oneshot_host_config(&workspace, "say smoke-ok".into(), None);
        host.permission = Some(isanagent::host::HostPermissionMode::Plan);
        host.no_color = true;
        host.scripted_responses = Some(vec!["smoke-ok".to_string()]);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(45),
            isanagent::host::run_oneshot(host),
        )
        .await
        .expect("oneshot timed out")
        .expect("oneshot failed");

        assert_eq!(result.outcome, isanagent::host::OneshotOutcome::Completed);
        assert_eq!(result.final_text.as_deref(), Some("smoke-ok"));
    }

    #[test]
    fn host_unavailable_has_the_reserved_exit_code() {
        assert_eq!(
            CliError::HostUnavailable { command: "agent" }.exit_code(),
            10
        );
    }

    #[test]
    fn machine_contract_labels_are_stable() {
        assert_eq!(PermissionMode::AutoEdit.as_str(), "auto-edit");
        assert_eq!(
            host_permission_mode(&PermissionMode::Plan),
            isanagent::host::HostPermissionMode::Plan
        );
        assert_eq!(ThemeMode::NoColor.as_str(), "no-color");
        assert_eq!(OutputMode::Jsonl.as_str(), "jsonl");
    }

    #[test]
    fn config_path_contract_parses_machine_output() {
        let cli = Cli::try_parse_from(["altai-cli", "config", "path", ".", "--json"])
            .expect("config path contract should parse");

        let Some(Commands::Config {
            command: ConfigCommands::Path(args),
        }) = cli.command
        else {
            panic!("config path command should parse");
        };
        assert_eq!(args.path, Some(PathBuf::from(".")));
        assert!(args.json);
    }

    #[test]
    fn config_list_contract_parses_resolved_origins() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "config",
            "list",
            ".",
            "--resolved",
            "--show-origin",
            "--json",
        ])
        .expect("config list contract should parse");

        let Some(Commands::Config {
            command: ConfigCommands::List(args),
        }) = cli.command
        else {
            panic!("config list command should parse");
        };
        assert!(args.resolved);
        assert!(args.show_origin);
        assert!(args.json);
    }

    #[test]
    fn config_value_keeps_origins_machine_readable() {
        let value = altai_core::ResolvedConfig {
            value: "test/model".to_string(),
            source: altai_core::ConfigSource::ProjectConfig,
        };
        assert_eq!(
            config_value(Some(&value), true),
            serde_json::json!({
                "value": "test/model",
                "source": "project-config",
            })
        );
    }

    #[test]
    fn journal_summary_contract_parses_chat_filter_and_json() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "journal",
            "summary",
            ".",
            "--chat",
            "chat-1",
            "--json",
        ])
        .expect("journal summary contract should parse");

        let Some(Commands::Journal {
            command: JournalCommands::Summary(args),
        }) = cli.command
        else {
            panic!("journal summary command should parse");
        };
        assert_eq!(args.path, Some(PathBuf::from(".")));
        assert_eq!(args.chat.as_deref(), Some("chat-1"));
        assert!(args.json);
    }

    #[test]
    fn journal_fetch_contract_parses_run_and_cursor() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "journal",
            "fetch",
            ".",
            "--run",
            "run-1",
            "--after",
            "3",
            "--limit",
            "50",
            "--json",
        ])
        .expect("journal fetch contract should parse");

        let Some(Commands::Journal {
            command: JournalCommands::Fetch(args),
        }) = cli.command
        else {
            panic!("journal fetch command should parse");
        };
        assert_eq!(args.run, "run-1");
        assert_eq!(args.after, 3);
        assert_eq!(args.limit, 50);
        assert!(args.json);
    }

    #[test]
    fn journal_summary_and_fetch_round_trip_a_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = altai_core::WorkspacePaths {
            root: temp.path().to_path_buf(),
            isanagent_state: temp.path().join(".isanagent"),
        };
        std::fs::create_dir_all(workspace.agent_event_journal_db().parent().unwrap())
            .expect("state dir");
        let journal =
            altai_core::EventJournal::open(workspace.agent_event_journal_db()).expect("open");
        journal
            .append(&altai_core::JournalEvent::now(
                1,
                "run-1",
                1,
                "chat-1",
                "run_started",
                serde_json::json!({ "type": "run_started", "run_id": "run-1" }),
            ))
            .expect("seed run_started");
        drop(journal);

        journal_summary(JournalSummaryArgs {
            path: Some(workspace.root.clone()),
            chat: Some("chat-1".to_string()),
            json: true,
        })
        .expect("journal summary should succeed");
        journal_fetch(JournalFetchArgs {
            path: Some(workspace.root),
            run: "run-1".to_string(),
            after: 0,
            limit: 10,
            json: false,
        })
        .expect("journal fetch should succeed");
    }

    #[test]
    fn models_current_contract_parses_origin_json() {
        let cli = Cli::try_parse_from([
            "altai-cli",
            "models",
            "current",
            ".",
            "--show-origin",
            "--json",
        ])
        .expect("models current contract should parse");

        let Some(Commands::Models {
            command: ModelsCommands::Current(args),
        }) = cli.command
        else {
            panic!("models current command should parse");
        };
        assert_eq!(args.path, Some(PathBuf::from(".")));
        assert!(args.show_origin);
        assert!(args.json);
    }
}
