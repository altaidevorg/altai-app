# ALTAI Complete CLI Implementation Plan

**Status:** Proposed  
**Date:** 2026-07-29  
**Scope:** A production-ready, cross-platform ALTAI command-line product that
shares the desktop application's agent engine, project state, credentials, and
orchestration data.

## 1. Outcome

Ship a real headless ALTAI CLI, not only the existing desktop launcher.

The completed product must support:

- an interactive terminal UI;
- one-shot and stdin-driven automation;
- stable plain-text and JSONL output;
- the same IsanAgent reasoning loop and tools used by ALTAI Desktop;
- shared workspaces, sessions, memory, checkpoints, skills, MCP configuration,
  background jobs, notifications, automations, and orchestration state;
- safe interactive approvals and deterministic non-interactive behavior;
- macOS, Linux, and Windows installation;
- backward-compatible desktop-launch commands.

The public entry point remains `altai`. The command routes either to the
headless CLI or to the desktop application:

```text
altai agent [PATH]                 # interactive terminal UI
altai run [PATH] --prompt TEXT     # one-shot/headless agent run
altai chat ...                     # session management
altai jobs ...                     # background work
altai orchestration ...            # local multi-agent operations
altai open [PATH]                  # launch ALTAI Desktop
altai [PATH]                       # backward-compatible alias for `altai open`
```

## 2. Current State

The repository already has several pieces needed by the CLI:

- `src-tauri/tauri.conf.json` declares launcher arguments such as paths,
  `--new-chat`, `--new-window`, `--explain`, and `--refactor`.
- `src-tauri/src/lib.rs` parses those arguments and forwards them to a Tauri
  window. This is a GUI launcher, not a headless CLI.
- `src-tauri/src/altai/agent/runtime.rs` embeds IsanAgent and adds ALTAI-specific
  routing, event journaling, approvals, sessions, notifications, automations,
  background work, checkpoints, MCP tools, and model failover.
- IsanAgent already contains a capable terminal channel and TUI, including
  model switching, sessions, tool activity, sub-agent activity, execution
  browsing, cancellation, compaction, skills, and background jobs.
- IsanAgent's complete terminal host currently lives in its binary
  `src/main.rs`; it is not exposed as a reusable library entry point.
- `src-tauri/src/main.rs` uses the Windows GUI subsystem in release builds. A
  console CLI cannot share that executable on Windows.
- desktop model preferences live in `altai-settings.json`, while API keys use
  `src-tauri/src/modules/secrets.rs`. These implementations currently depend on
  Tauri APIs or TypeScript-only model metadata.
- the release pipeline bundles only the Tauri application.

The implementation must reuse these pieces while removing host-specific
coupling. Copying IsanAgent's `main.rs` or creating a second agent loop is not
acceptable.

## 3. Product Boundaries

### 3.1 In scope

- All agent workflows that can be represented safely in a terminal.
- Interactive TUI and simple line-oriented fallback.
- Scriptable one-shot execution.
- Session, job, notification, automation, checkpoint, skill, MCP, model,
  credential, configuration, and orchestration commands.
- Project-local and desktop-shared configuration.
- Attachments supplied as local paths or stdin.
- Desktop launching and existing OS integration.
- Cross-platform packages, shell completion, man pages, and update guidance.

### 3.2 Not literal UI parity

The CLI will not recreate visual editor panes, web previews, xterm tabs,
notebook cells, settings forms, or GitHub project-board screens. Their
terminal equivalents are:

- normal filesystem and Git tools;
- unified diffs and approval prompts;
- agent and orchestration commands;
- structured status output;
- links or explicit `altai open` handoff when a visual surface is required.

This is functional parity for agent operations, not a terminal rendering of
every React component.

### 3.3 Explicit non-goals

- No second provider implementation.
- No external Codex/Claude CLI runner.
- No separate session database for CLI use.
- No implicit upload of workspace files.
- No silent permission bypass in non-interactive mode.
- No parsing human-formatted output as an internal API.

### 3.4 CLI-Anything development contract

CLI-Anything is a required development harness for this product, not the
production runtime or a Python replacement for ALTAI. Its GUI-to-CLI SOP,
stateful-REPL/subcommand model, JSON discipline, real-backend validation,
subprocess testing, and generated agent skill are adopted as release gates.

Integration rules:

1. Pin the CLI-Anything source revision under `tools/cli-anything/` as a Git
   submodule or an equivalent immutable vendor reference; document the resolved
   revision in `docs/cli/CLI_ANYTHING_ANALYSIS.md`.
2. Run its analysis/design workflow against ALTAI before implementation. Store
   the resulting GUI-to-terminal action map, state map, command matrix, and
   gap analysis in that document.
3. Use its `refine`/`validate` methodology at each feature milestone to compare
   the desktop capability map against the shipped CLI capability map.
4. Create `docs/cli/TEST.md` before implementation, then append actual unit,
   integration, PTY, packaging, and real-workspace results as required by the
   harness methodology.
5. Generate and ship `skills/altai-cli/SKILL.md`, including command groups,
   JSONL use, exit codes, permissions, preview artifacts, and recovery paths.
6. Preserve the CLI-Anything principle of exercising the real backend: E2E
   tests run the installed ALTAI CLI against the actual Rust/IsanAgent runtime,
   not a mocked Python wrapper.

Generated Click/Python harness code is reference material only. Production code
remains Rust so it can share ALTAI's IsanAgent runtime, Tauri-owned services,
workspace databases, secret store, and cross-platform packaging.

## 4. Command Contract

### 4.1 Top-level commands

```text
altai [PATH] [desktop launcher options]
altai open [PATH] [--new-window] [--new-chat] [--explain FILE] [--refactor FILE]
altai agent [PATH] [agent options]
altai run [PATH] --prompt TEXT [run options]
altai chat <list|show|resume|fork|delete|export>
altai jobs <list|show|logs|cancel|dismiss>
altai inbox <list|show|reply|resolve|dismiss>
altai automation <list|add|remove|run>
altai checkpoint <list|show|restore>
altai skills <list|add|remove|show>
altai paper <fetch>
altai mcp <list|add|remove|enable|disable|probe>
altai models <list|current|set>
altai auth <login|logout|status>
altai config <get|set|unset|list|path|edit>
altai orchestration <init|status|start|pause|resume|stop|readiness|quality|plan|tasks|graph|garden|support-bundle>
altai doctor
altai completion <bash|zsh|fish|powershell|elvish>
altai version
altai help [COMMAND]
```

### 4.2 Interactive agent

```bash
altai agent
altai agent .
altai agent ./service --model anthropic/claude-sonnet-4-6
altai agent . --resume CHAT_ID
altai agent . --permission auto-edit
altai agent . --no-tui
```

Behavior:

- `PATH` defaults to the current directory.
- ALTAI state is rooted at `<PATH>/.isanagent`, matching the desktop runtime.
- A TTY starts the IsanAgent TUI by default.
- `--no-tui` starts an accessible, line-oriented REPL.
- `Ctrl+C` cancels the active run; a second `Ctrl+C` exits.
- `Ctrl+D` exits when the input buffer is empty.
- terminal restoration is guaranteed through an RAII guard, including panic
  and signal paths.
- existing IsanAgent slash commands remain supported. ALTAI-specific commands
  are added without changing their current meanings.

Required slash commands:

```text
/help                 /new
/chats                /resume <chat-id>
/model [model]        /permission [mode]
/context              /compact [focus]
/cancel               /retry
/background           /jobs
/tools                /exec
/agents               /inbox
/checkpoint           /skills
/mcp                  /open
/exit
```

### 4.2.1 IsanAgent TUI feature baseline

`altai agent` must host the IsanAgent TUI; it must not replace it with a
new, ALTAI-specific terminal renderer. The following upstream capabilities are
required product scope and remain available after the ALTAI integration:

| IsanAgent TUI capability | ALTAI CLI requirement |
|---|---|
| Transcript, markdown rendering, scrolling, selection, and copy-last-reply | Preserve the interaction model and add ALTAI run IDs to tool/error details. |
| Composer, slash-command completion, `/help`, `/new`, `/retry`, `/cancel`, and `/exit` | Preserve existing behavior; map cancellation to ALTAI's run lease. |
| Past-session browser and `/chats` | Read the same ALTAI/IsanAgent session history that the desktop uses; allow resume and fork. |
| Interactive `/model` selector and direct model switch | Resolve ALTAI's shared model catalog, key store, fallback model, and per-run provider snapshot. |
| Tool activity pane and tool-progress notices | Render ALTAI tool telemetry, approvals, edit diffs, errors, and usage accounting. |
| Execution browser, synchronous-run promotion, and background-job pane | Use ALTAI execution jobs, durable job records, cancellation, and notifications. |
| Sub-agent task pane | Display ALTAI subagent lifecycle events and link each task to its chat/run identity. |
| Notifications and background-job controls | Back the pane with ALTAI's durable inbox and job services, not transient terminal-only state. |
| `/context` and `/compact [focus]` | Use the existing ALTAI compaction policy and journal the request. |
| `/skills` and installed-skill listing | Use the workspace-local ALTAI/IsanAgent skill registry. |
| Terminal accessibility controls and `NO_COLOR` behavior | Preserve upstream terminal behavior and provide ALTAI line mode when a full TUI is unsuitable. |

ALTAI additions are limited to adapters, status information, and new panes or
commands backed by ALTAI services. They must not fork the upstream TUI state
machine or duplicate its keyboard/event loop.

### 4.2.2 ALTAI terminal visual system

The TUI visual layer is an ALTAI theme for IsanAgent's existing Ratatui
components. It derives semantic tokens and density from
`src/styles/globals.css`; it does not attempt to render the web UI inside a
terminal.

#### Theme modes

```text
auto       use terminal/background capability detection where possible
dark       ALTAI's default near-black IDE surface
light      ALTAI's light semantic counterpart
no-color   preserve hierarchy through labels, borders, bold/dim, and symbols
```

`altai agent --theme auto|dark|light|no-color` and `ALTAI_TUI_THEME` select the
mode. `NO_COLOR` always wins. A terminal cannot reliably choose Inter or
JetBrains Mono itself, so the design relies on the user's monospace terminal
font; terminal setup documentation recommends JetBrains Mono to match ALTAI
Desktop's code font.

#### Token translation

| ALTAI App token | Terminal role | Usage rule |
|---|---|---|
| `--background` | canvas | Near-black default canvas; never use as an active state. |
| `--card`, `--raised`, `--overlay` | panel depth | Three restrained surface levels for transcript, side pane, and modal/picker. |
| `--foreground`, `--muted-foreground` | primary and secondary text | High-contrast content; dim metadata remains legible. |
| `--primary` (acid lime) | active/focus/progress | Reserve for focused row, active pane, model state, progress, and primary confirmation. |
| `--success`, `--warning`, `--info`, `--destructive` | semantic outcome | Never substitute lime for success, warning, information, or error. |
| `--border`, `--border-subtle` | 1-cell separators | Use precision rules and square terminal geometry; avoid decorative boxes. |
| `--ring` | keyboard focus | Show a clear focus marker in addition to color. |

The implementation uses truecolor when available, an intentional 256-color
fallback, and the existing no-color styles otherwise. Exact OKLCH conversion
values are generated once into an `AltaiTerminalPalette`; hand-picked ANSI
colors are not allowed to drift from the application tokens.

#### Component specifications

| TUI surface | ALTAI visual treatment |
|---|---|
| Header/status bar | One dense row: `ALTAI`, workspace, current model, permission mode, active run state, and token/job indicators. Lime marks the active control only. |
| Transcript | Card canvas; user turns use a muted, bordered block; assistant turns stay open and content-led; thinking and metadata are subdued. |
| Tool rail | Raised rows with semantic pending/success/error markers, concise tool name, elapsed time, and expandable detail. |
| Composer | A precision bordered input on the card surface; command completion and attachments appear as an overlay layer. |
| Side panes | Sessions, tools, executions, subagents, jobs, and inbox retain IsanAgent navigation but use card/raised surfaces and a 2-cell lime active rail. |
| Model picker and command help | Overlay surface, high-contrast selected row, provider/model metadata muted, keyboard hints always visible. |
| Approval and clarification | Blocking overlay with risk label, exact command or diff summary, choices, and a non-color text indicator. |
| Diff review | Unified diff with semantic add/remove colors, line prefixes, filenames, hunk IDs, and a textual fallback when color is unavailable. |
| Background work | Compact status strip and jobs pane; status icons and labels distinguish waiting, running, complete, failed, cancelled, and timed out. |

#### Responsive and accessible layout

- At 120 columns or wider, show transcript plus the focused secondary pane.
- Between 80 and 119 columns, retain the transcript and switch secondary panes
  through IsanAgent's existing tab navigation.
- Below 80 columns, collapse to a single pane with a persistent status line and
  command hints; never truncate an approval decision or diff path silently.
- Every color state has a text label, icon, or border treatment. Focus, active
  run, approval, error, and background-job status are understandable with
  `NO_COLOR` and screen-reader-compatible line mode.
- Respect reduced motion by using static status symbols instead of animated
  spinners when `ALTAI_TUI_REDUCE_MOTION=1` or terminal capability is limited.

### 4.3 One-shot execution

```bash
altai run . --prompt "Fix the failing tests"
altai run . -p "Explain the authentication flow" --permission plan
printf '%s' "Review this diff" | altai run . --prompt -
altai run . -p "Implement issue 42" --output jsonl
altai run . -p "Summarize the repository" --output final
```

Required behavior:

- `--prompt -` reads the prompt from stdin.
- `--file PATH` may be repeated for text, image, and document attachments.
- `--resume CHAT_ID` continues an existing session.
- `--new-chat` always creates a new session.
- `--timeout DURATION` applies to the complete foreground run.
- `--output pretty|plain|final|jsonl` controls stdout.
- progress goes to stderr in `plain` and `final` modes.
- `--quiet` suppresses non-error stderr output.
- `--no-color` and the standard `NO_COLOR` variable disable color.
- `--json` is an alias for `--output jsonl`.
- stdout remains machine-safe in `jsonl` mode; diagnostics never contaminate
  it.

### 4.4 Shared agent options

```text
--workspace PATH
--model PROVIDER/MODEL
--fallback-model PROVIDER/MODEL
--provider PROVIDER
--base-url URL
--api-key-env NAME
--agent PROFILE
--instructions TEXT
--permission ask|auto-edit|plan|bypass
--compact-threshold TOKENS
--compact-tail TURNS
--no-auto-compact
--no-prune
--resume CHAT_ID
--file PATH
--mcp-server NAME
--no-mcp
```

`--api-key` is intentionally omitted from normal help because command-line
arguments leak through shell history and process listings. An emergency hidden
alias may exist only if it prints a warning; environment variables, secure
prompting, and `altai auth login` are the supported paths.

## 5. Output and Exit-Code Contract

### 5.1 JSONL envelope

Every JSONL record uses one versioned envelope:

```json
{
  "schema_version": 1,
  "sequence": 12,
  "timestamp_ms": 1785300000000,
  "workspace": "/absolute/project/path",
  "chat_id": "chat-id",
  "run_id": "run-id",
  "type": "tool_call_started",
  "data": {}
}
```

Required event types:

```text
run_started
assistant_delta
assistant_message
thinking
tool_call_started
tool_call_progress
tool_call_finished
approval_requested
clarification_requested
edit_diff
usage
background_job_updated
notification_created
subagent_started
subagent_finished
run_warning
run_finished
error
```

The schema is additive within version 1. Removing or changing a field requires
a new schema version. Golden fixtures must protect the contract.

### 5.2 Exit codes

```text
0  successful run or successful management command
1  agent run failed
2  CLI syntax or validation error
3  configuration or credential error
4  approval/clarification required but unavailable
5  workspace authorization or filesystem error
6  provider/network error after retry and fallback
7  cancelled by user
8  timeout
9  partial orchestration failure
10 internal error
```

When the agent finishes with a structured terminal outcome, that outcome maps
to exactly one exit code. The JSONL `run_finished` or `error` record must agree
with the process exit code.

## 6. Configuration and Credential Contract

### 6.1 Precedence

Resolve every setting once, before creating the runtime:

1. explicit CLI flag;
2. `ALTAI_*` environment variable;
3. project CLI overrides in `<project>/.altai/config.toml`;
4. desktop AI preferences and shared secret store;
5. IsanAgent configuration in `<project>/.isanagent/config.toml`;
6. compiled default.

The resolved configuration is immutable for a one-shot run. Interactive
`/model` and `/permission` changes create an explicit runtime update and persist
only when the user confirms.

`altai config list --resolved --show-origin` must show the effective value and
its source without revealing secret values.

### 6.2 Environment variables

Support:

```text
ALTAI_MODEL
ALTAI_FALLBACK_MODEL
ALTAI_PROVIDER
ALTAI_BASE_URL
ALTAI_API_KEY_ENV
ALTAI_PERMISSION
ALTAI_OUTPUT
ALTAI_NO_COLOR
ALTAI_CONFIG
ALTAI_DISABLE_AUTOCOMPACT
ALTAI_DISABLE_PRUNE
OPENAI_API_KEY
ANTHROPIC_API_KEY
GEMINI_API_KEY
XAI_API_KEY
CEREBRAS_API_KEY
GROQ_API_KEY
DEEPSEEK_API_KEY
MISTRAL_API_KEY
ZAI_API_KEY
OPENROUTER_API_KEY
```

Provider-specific variables take precedence over the desktop secret only when
that provider is selected.

### 6.3 Shared model catalog

The TypeScript-only provider/model catalog cannot be duplicated in Rust.

Create:

```text
shared/model-catalog.json
shared/model-catalog.schema.json
src-tauri/crates/altai-core/src/model_catalog.rs
src/modules/ai/generated/modelCatalog.ts
```

`model-catalog.json` becomes the canonical source for provider identifiers,
model identifiers, API names, endpoints, key requirements, context limits, and
capabilities. A generation/check script creates the TypeScript projection and
CI fails when generated output is stale.

### 6.4 Shared credentials

Extract the non-Tauri parts of `src-tauri/src/modules/secrets.rs` into
`altai-core`:

```rust
trait SecretStore {
    fn get(&self, service: &str, account: &str) -> Result<Option<SecretString>>;
    fn set(&self, service: &str, account: &str, value: SecretString) -> Result<()>;
    fn delete(&self, service: &str, account: &str) -> Result<()>;
}
```

Backends:

- macOS/Linux: the existing mode-`0600` ALTAI secret file, with atomic writes;
- Windows: Credential Manager;
- tests: in-memory store.

The desktop Tauri commands become thin adapters over the same trait. Existing
credentials remain readable without migration.

`altai auth login PROVIDER` reads a secret from a no-echo terminal prompt.
`altai auth status` reports only presence and source. Secrets are redacted from
logs, events, support bundles, panic messages, and JSONL output.

## 7. Architecture

### 7.1 Binary split

Use separate console and GUI executables:

```text
altai                   console router and CLI
altai-desktop           Tauri GUI executable
```

This is required because the release desktop binary uses
`windows_subsystem = "windows"`, while a Windows CLI must attach to the console.

Routing rules:

- known headless subcommands run in the console process;
- `open` launches `altai-desktop`;
- a bare existing path preserves today's behavior and launches the desktop;
- legacy launcher flags are translated to `open`;
- no arguments show concise CLI help in a terminal, unless a platform GUI
  launch explicitly invokes `altai-desktop`;
- OS file associations, deep links, Dock items, and Jump Lists continue to
  target `altai-desktop` directly.

### 7.2 Cargo workspace

Convert `src-tauri/Cargo.toml` into the root package plus workspace and add:

```text
src-tauri/crates/altai-core/
src-tauri/crates/altai-cli/
```

Responsibilities:

```text
altai-core
  configuration resolution
  model catalog
  secret store
  workspace identity and authorization
  runtime construction
  event envelope
  session/job/notification/checkpoint services
  orchestration service facade
  no Tauri, WebKit, React, or terminal-rendering dependency

altai-cli
  clap command definitions
  terminal and line-mode adapters
  JSONL/plain renderers
  signal handling
  desktop process routing
  shell completion and man-page generation

altai_lib
  Tauri application
  Tauri event adapter
  webview commands
  OS integrations
  visual-only features
```

The CLI crate depends on `altai-core` and IsanAgent terminal components. It
must not depend on `tauri`, WebKit, GTK, or frontend assets.

### 7.3 IsanAgent host extraction

Submit a focused upstream change to `altaidevorg/isanagent`:

1. move runtime construction from IsanAgent `src/main.rs` into a public library
   module such as `isanagent::host`;
2. expose a typed `HostConfig`, `HostHandle`, and lifecycle stream;
3. keep the current `isanagent` binary as a thin caller of that API;
4. expose terminal channel construction without embedding ALTAI branding;
5. add hooks for extra tools, system-prompt sections, event observers, workspace
   services, and shutdown;
6. preserve all existing IsanAgent CLI and TUI behavior through tests, including
   transcript, session, model, tool, execution, sub-agent, job, notification,
   compaction, and skill flows.
7. expose a supported theme/palette boundary so ALTAI can supply its terminal
   visual system without patching upstream rendering internals.

ALTAI consumes the merged upstream commit through its existing branch
dependency. Do not copy upstream `main.rs` and do not maintain an ALTAI fork.

Proposed API shape:

```rust
pub struct HostConfig {
    pub workspace: PathBuf,
    pub provider: ProviderSpec,
    pub fallback: Option<ProviderSpec>,
    pub permission: PermissionPolicy,
    pub compaction: CompactionPolicy,
    pub terminal: Option<TerminalConfig>,
}

pub struct HostExtensions {
    pub tools: Vec<Arc<dyn Tool>>,
    pub prompt_sections: Vec<String>,
    pub event_sink: Arc<dyn EventSink>,
}

pub async fn start_host(
    config: HostConfig,
    extensions: HostExtensions,
) -> Result<HostHandle, HostError>;
```

### 7.4 ALTAI runtime extraction

Refactor `src-tauri/src/altai/agent/runtime.rs` in small, testable moves:

1. replace direct `AppHandle` event emission with an `EventSink` trait;
2. replace `tauri::async_runtime::spawn` with an injected spawner or
   `tokio::spawn`;
3. move workspace-owned memory, event journal, session, job, notification,
   automation, and checkpoint setup into `altai-core`;
4. keep `TauriChannel` as a desktop adapter;
5. add `CliChannel` as a console adapter;
6. move `DocumentArg` into a shared attachment type;
7. inject MCP status reporting rather than looking it up through
   `AppHandle::try_state`;
8. retain the same event journal and runtime fingerprints for desktop and CLI.

The core runtime must accept multiple simultaneous chat owners and preserve the
existing run-lease protections. A CLI process must never corrupt a session
that is active in the desktop process.

### 7.5 Cross-process ownership

Add a per-workspace runtime lock and lease record under:

```text
<workspace>/.isanagent/.system_generated/runtime.lock
<workspace>/.isanagent/.system_generated/runtime-lease.json
```

Rules:

- read-only management commands may run concurrently;
- a foreground reasoning run obtains a chat-scoped lease;
- SQLite remains in WAL mode with a busy timeout;
- attempting to run the same chat from desktop and CLI returns a structured
  ownership error;
- stale leases are recoverable after PID/start-time validation;
- `--force` may recover only a proven-stale lease, never a live one.

## 8. Permission and Safety Model

Map CLI modes to the same shell and edit policy used by the desktop:

```text
ask        prompt before protected shell commands and file edits
auto-edit  apply file edits; continue prompting for protected shell commands
plan       deny mutations; permit safe inspection according to policy
bypass     use the existing guarded bypass policy
```

Requirements:

- `ask` requires an interactive terminal. If approval is needed without a TTY,
  emit `approval_requested` and exit `4`.
- `bypass` requires the explicit `--permission bypass` flag. It is never
  selected from config on a first run without a warning.
- `--yes` answers ordinary confirmations, but does not imply bypass.
- dangerous-command guards remain active in every mode.
- file-edit approvals show a unified diff and the exact workspace-relative
  path.
- approval input accepts `yes`, `no`, `always for this run`, and `abort`.
- "always" applies only to the current process unless a separate config command
  persists it.
- non-interactive runs default to `plan` unless the user explicitly selects
  another mode.
- symlink and canonical-path checks use the same authorized workspace root as
  the desktop.

## 9. Feature Workstreams

### 9.1 Interactive terminal

- Host the reusable IsanAgent TUI inside `altai agent`; do not reimplement its
  terminal renderer, input loop, or standard panes.
- Preserve every capability in the IsanAgent TUI feature baseline above:
  transcript, composer, slash completion, sessions, model selector, tool
  activity, executions, background jobs, sub-agent tasks, notifications,
  compaction, skills, copy, retry, cancellation, and keyboard navigation.
- Rebrand the startup banner and status metadata as ALTAI without obscuring the
  upstream interaction model.
- Implement `AltaiTerminalPalette` from the desktop semantic tokens and apply
  it to existing IsanAgent TUI components through the upstream theme boundary.
- Implement dark, light, auto, and no-color modes; include truecolor,
  256-color, and monochrome fallbacks.
- Apply the component specifications for header, transcript, tool rail,
  composer, side panes, overlays, approval/diff review, and background work.
- Add permission mode and workspace to the status bar.
- Add ALTAI event-journal replay on startup.
- Render streamed markdown, tool progress, usage, warnings, diffs,
  clarifications, and terminal outcomes.
- Provide a line-mode fallback for unsupported terminals and screen readers.
- Honor `NO_COLOR`, terminal width, reduced motion, and Unicode capability.

### 9.2 One-shot runner

- Inject exactly one inbound prompt.
- Stream events through the selected renderer.
- Support attachments and resumed sessions.
- Handle approvals using `/dev/tty` or `CONIN$` only when available.
- Await a terminal run outcome, flush output, release leases, and exit.
- On `SIGINT`, send run cancellation, wait for bounded cleanup, then exit `7`.
- On timeout, cancel the run and exit `8`.

### 9.3 Sessions

Implement:

```text
altai chat list [--workspace PATH] [--limit N] [--json]
altai chat show CHAT_ID [--events|--messages] [--json]
altai chat resume CHAT_ID
altai chat fork CHAT_ID [--after MESSAGE_ID]
altai chat delete CHAT_ID [--yes]
altai chat export CHAT_ID [--format markdown|json]
```

Use the existing IsanAgent memory database and ALTAI event journal. Deletion is
recoverable where practical and must reject active chats.

### 9.4 Jobs, inbox, and automations

Expose the existing ALTAI services:

```text
altai jobs list|show|logs|cancel|dismiss
altai inbox list|show|reply|resolve|dismiss
altai automation list|add|remove|run
```

Automation creation accepts either an ISO-8601 `--at` value or a validated
`--every` duration. Arbitrary cron input is accepted only through the trusted
IsanAgent tool/config path unless the existing scheduler validator is reused.

### 9.5 Checkpoints and skills

```text
altai checkpoint list
altai checkpoint show CHECKPOINT_ID
altai checkpoint restore CHECKPOINT_ID [--dry-run] [--yes]
altai skills list
altai skills show NAME
altai skills add REPOSITORY [--skill NAME]
altai skills remove NAME [--yes]
```

Checkpoint restore displays affected paths before confirmation. Skill
installation retains the current repository validation and normal agent
approval policy.

### 9.6 MCP

```text
altai mcp list
altai mcp add NAME --command COMMAND [--arg VALUE ...] [--env NAME=SOURCE ...]
altai mcp add NAME --url URL
altai mcp enable|disable|remove NAME
altai mcp probe NAME
```

Use the same `<workspace>/.isanagent/mcp.json` and the same SSRF, environment,
process, and secret-handling rules as the desktop. `list` never prints secret
values.

### 9.7 Papers

```text
altai paper fetch ARXIV_URL_OR_ID [--output PATH] [--attach]
```

Reuse the existing paper-fetch implementation and validation from
`agent_fetch_paper`. `--attach` starts or resumes an agent chat with the
downloaded paper as a document attachment; plain fetch prints metadata and the
saved path.

### 9.8 Models and auth

```text
altai models list [--provider NAME] [--available]
altai models current [--show-origin]
altai models set PROVIDER/MODEL [--fallback]
altai auth login PROVIDER
altai auth logout PROVIDER
altai auth status
```

Model aliases resolve through the shared catalog. A custom OpenAI-compatible
model requires an explicit base URL and model ID.

### 9.9 Orchestration

Expose the existing local orchestration backend through a typed service facade,
not by invoking Tauri command wrappers internally:

```text
altai orchestration init
altai orchestration status
altai orchestration start [--workflow FILE]
altai orchestration pause|resume|stop
altai orchestration readiness
altai orchestration quality
altai orchestration plan parse FILE
altai orchestration tasks list|show|add|update
altai orchestration graph show|eligible|blocked
altai orchestration garden [--fix-safe]
altai orchestration support-bundle OUTPUT
```

Every command supports JSON output. State-changing commands use workspace
authorization, existing policy gates, budgets, task graph validation, and the
same SQLite/filesystem stores as the desktop.

### 9.10 Existing agent-command parity

| Existing backend surface | Required CLI surface |
|---|---|
| start and send | `agent`, `run` |
| compact | `/compact` and `run --compact-*` configuration |
| approve, cancel, steer | terminal approval UI, `/cancel`, and interactive steering |
| list/get/truncate sessions | `chat list`, `show`, `resume`, `fork`, `delete`, `export` |
| durable event replay and cursor | automatic resume plus `chat show --events` |
| notifications and clarification tickets | `inbox` |
| background jobs | `jobs` |
| automations | `automation` |
| paper fetch | `paper fetch` |
| checkpoints | `checkpoint` |
| skills | `skills` |

This table is the minimum command-parity checklist for
`src-tauri/src/altai/agent/commands.rs`. New backend commands added during CLI
development must update the table and either gain a CLI route or be marked
visual-only with a reason.

### 9.11 Diagnostics

`altai doctor` checks:

- CLI and desktop versions;
- writable ALTAI data directories;
- workspace canonicalization and authorization;
- Git and required shell availability;
- provider configuration without sending a paid request by default;
- optional `--network` provider endpoint checks;
- MCP server configuration and executable discovery;
- SQLite integrity and schema versions;
- stale runtime leases;
- terminal capabilities;
- IsanAgent dependency revision.

The command must redact secrets and provide actionable remediation.

## 10. Desktop Compatibility and Migration

### 10.1 Preserve launcher behavior

The following continue to open ALTAI Desktop:

```bash
altai .
altai path/to/file.ts
altai --new-chat
altai --new-window
altai --explain file.ts
altai --refactor file.ts
altai --ask-project .
```

New scripts should use the unambiguous `altai open` form. Print a deprecation
notice only if a future release intends to repurpose bare paths.

### 10.2 Desktop executable rename

- rename the internal GUI binary to `altai-desktop`;
- update Tauri configuration and bundle metadata;
- update `current_exe`-based OS integration assumptions;
- keep product name, application identifier, data directories, file
  associations, and deep-link scheme unchanged;
- make context menus and OS launchers target the GUI binary directly;
- make the console router locate the GUI relative to itself on every platform.

### 10.3 Data migration

No session or credential migration should be required. If settings format must
change:

- read both old and new formats;
- write the new format atomically;
- retain a timestamped backup;
- record a schema version;
- add fixture-based migration tests for macOS, Linux, and Windows layouts.

## 11. Packaging and Installation

### 11.1 Release artifacts

Produce:

```text
altai_<version>_darwin_arm64.tar.gz
altai_<version>_darwin_x64.tar.gz
altai_<version>_linux_x64.tar.gz
altai_<version>_windows_x64.zip
SHA256SUMS
```

Also include the CLI in desktop distributions:

- macOS: bundle the CLI in `ALTAI.app/Contents/Resources/bin/altai` and provide
  an "Install Command Line Tool" action that creates a user-approved symlink;
- Linux deb/rpm: install the router at `/usr/bin/altai` and the GUI binary in
  the package's libexec directory;
- AppImage: expose an AppRun route and document the optional shell wrapper;
- Windows NSIS/MSI: install `altai.exe` and `altai-desktop.exe`, with an
  opt-in PATH entry;
- standalone archives: contain the console binary, licenses, completions, and
  man page, without WebKit or frontend assets.

### 11.2 Versioning

- CLI and desktop use the same ALTAI version.
- `altai version --verbose` prints ALTAI version, target triple, build commit,
  IsanAgent revision, and JSON schema version.
- a CLI/desktop data-schema incompatibility fails with a clear upgrade message.

### 11.3 Release pipeline

Update `.github/workflows/release.yml` to:

1. build and test `altai-cli` for every release target;
2. build the desktop GUI;
3. stage the correct binary layout for each installer;
4. generate completions and man pages from the Clap command tree;
5. create standalone archives;
6. calculate checksums;
7. smoke-test `altai version`, `altai help`, `altai doctor`, JSONL output, and
   `altai open --dry-run`;
8. attach all artifacts to the same GitHub release.

Pin the resolved IsanAgent commit for a release. The current automatic
`cargo update -p isanagent` behavior must produce and publish the resolved
revision so CLI and desktop cannot accidentally build against different
commits.

## 12. Testing Strategy

### 12.1 Unit tests

- Clap parsing, aliases, conflicts, and help snapshots.
- path canonicalization and desktop routing.
- config precedence and origin reporting.
- provider/model resolution from the shared catalog.
- secret redaction and backend behavior.
- permission mapping and non-TTY rejection.
- event-to-plain and event-to-JSONL rendering.
- terminal outcome to exit-code mapping.
- duration, ISO timestamp, attachment, and MCP validation.
- runtime lease acquisition, stale recovery, and contention.

### 12.2 Integration tests

Use a deterministic mock provider and temporary workspace:

- interactive message produces a persisted session;
- one-shot run streams valid JSONL and exits `0`;
- provider failure exits `6`;
- cancellation exits `7` and releases the lease;
- timeout exits `8`;
- approval without a TTY exits `4`;
- auto-edit changes only an authorized file;
- resumed desktop-created sessions are readable from CLI and vice versa;
- checkpoints restore expected files;
- background jobs survive process exit and are visible on restart;
- automation and inbox state are shared with desktop;
- MCP probe handles success, timeout, malformed protocol, and secret redaction;
- concurrent desktop/CLI ownership is rejected safely;
- orchestration commands operate on the same ledger and task graph.

### 12.3 Terminal tests

Use PTY-based tests on all platforms:

- TUI startup and clean shutdown;
- resize handling;
- `Ctrl+C` cancel and double-press exit;
- `Ctrl+D` exit;
- approval and clarification prompts;
- Unicode and narrow terminal rendering;
- `NO_COLOR`;
- line-mode fallback;
- terminal state restoration after panic.
- visual snapshot tests at 80x24, 100x30, and 160x48 for dark, light,
  256-color, and no-color themes;
- active/focus, approval, diff, model picker, tool, job, and narrow-layout
  snapshots;
- palette generation checks against the semantic ALTAI App token source.

### 12.4 Contract tests

- JSONL schema fixtures.
- event ordering and monotonically increasing sequence values.
- stdout/stderr separation.
- exit-code fixtures.
- session/event-journal compatibility fixtures.
- model catalog generation and stale-file check.
- CLI help and man-page snapshots.

### 12.5 CI matrix

Extend `.github/workflows/ci.yml`:

```text
Ubuntu: format, clippy, all core/CLI tests, PTY tests, packaging smoke test
macOS: compile, core/CLI tests, PTY tests, bundle path smoke test
Windows: compile, core/CLI tests, ConPTY tests, GUI/console subsystem checks
Frontend: existing checks plus model-catalog generation check
```

No live paid-provider test runs on pull requests. A manually triggered,
secret-backed nightly job may run one minimal request per supported provider.

## 13. Documentation

Add:

```text
docs/cli/README.md
docs/cli/commands.md
docs/cli/configuration.md
docs/cli/automation.md
docs/cli/jsonl-schema.md
docs/cli/security.md
docs/cli/migration.md
docs/cli/troubleshooting.md
docs/cli/CLI_ANYTHING_ANALYSIS.md
docs/cli/TEST.md
skills/altai-cli/SKILL.md
```

Update:

- root `README.md`;
- `INSTALL.md`;
- release notes template;
- `altai help`;
- generated man page;
- examples for GitHub Actions, shell pipelines, pre-commit use, and local
  OpenAI-compatible servers.

All examples must use fake keys and avoid bypass permissions unless the example
explicitly explains the risk.

## 14. Implementation Sequence

### Phase 0 — Contract freeze

- [ ] Approve this command tree, output schema, exit codes, permission behavior,
      config precedence, and compatibility policy.
- [ ] Add command/help snapshots and JSON schema fixtures before runtime work.
- [x] Record the desktop/CLI parity matrix.
- [x] Pin CLI-Anything and run its analysis/design workflow against ALTAI.
- [x] Add the resulting action/state/command gap analysis and the initial
      `docs/cli/TEST.md` plan.
- [ ] Approve the ALTAI terminal visual system, palette-generation source, and
      snapshots before theme implementation.

**Gate:** The public contract is reviewed; later phases cannot invent
incompatible command behavior.

### Phase 1 — Upstream reusable IsanAgent host

- [ ] Extract IsanAgent runtime construction into a public library API.
- [ ] Expose terminal channel, event stream, extensions, and shutdown.
- [ ] Keep the upstream `isanagent` binary behavior unchanged.
- [ ] Add upstream unit and integration coverage.
- [ ] Merge upstream and update ALTAI's dependency.

**Gate:** A minimal Rust test can start the host with a mock provider, send one
message, observe a terminal outcome, and shut it down without invoking
IsanAgent's binary.

### Phase 2 — ALTAI core extraction

- [ ] Add `altai-core`.
- [ ] Extract event sink, workspace services, attachments, configuration,
      secrets, and model resolution.
- [ ] Adapt the desktop runtime to the core without behavior changes.
- [ ] Add shared model catalog generation.

**Gate:** Existing desktop Rust and frontend tests pass, and core tests require
no Tauri runtime.

### Phase 3 — CLI skeleton and desktop router

- [ ] Add `altai-cli` with Clap.
- [ ] Split `altai` and `altai-desktop`.
- [ ] Implement help, version, completion, config, auth, models, doctor, and
      backward-compatible desktop routing.
- [ ] Update OS integration targets.

**Gate:** Every platform can run console help without opening a window and can
route `altai .` to the desktop.

### Phase 4 — Interactive agent

- [ ] Connect the ALTAI host to the reusable IsanAgent TUI.
- [ ] Port and prove the entire IsanAgent TUI feature baseline: transcript,
      model selector, session browser, tool/execution/sub-agent panes,
      background jobs, notifications, compaction, skills, retry, cancellation,
      and keyboard navigation.
- [ ] Add ALTAI branding, statuses, permission control, durable service
      adapters, journal replay, and ALTAI slash commands.
- [ ] Add the ALTAI terminal visual system through the upstream theme boundary:
      dark/light/auto/no-color palettes, responsive layouts, focus treatment,
      approval/diff surfaces, and visual snapshot coverage.
- [ ] Add line mode and robust signal/terminal cleanup.

**Gate:** A user can complete an edit-and-approval workflow, switch models,
resume a prior session, inspect tools/executions/subagents, manage a background
job, compact context, install/list skills, and reopen the same chat in the
desktop — all from the terminal.

### Phase 5 — One-shot and machine output

- [ ] Implement `altai run`.
- [ ] Add stdin, attachments, resume, timeout, cancellation, and output modes.
- [ ] Freeze JSONL schema version 1 and exit-code mapping.

**Gate:** A CI script can run ALTAI, parse only stdout as JSONL, and rely on the
documented exit status.

### Phase 6 — Management commands

- [ ] Add chat, jobs, inbox, automation, checkpoint, skills, paper, and MCP
      commands.
- [ ] Add read-only concurrency and state-changing lease enforcement.

**Gate:** Every non-visual agent-management action exposed by
`agent/commands.rs` has a CLI equivalent or a documented reason it is
desktop-only.

### Phase 7 — Orchestration commands

- [ ] Extract Tauri-free orchestration service methods.
- [ ] Implement status, lifecycle, readiness, quality, plans, tasks, graph,
      gardening, and support bundles.
- [ ] Add JSON output and partial-failure exit behavior.

**Gate:** A local orchestration workflow can be initialized, started,
inspected, paused, resumed, and stopped without opening the desktop.

### Phase 8 — Packaging and release

- [ ] Add standalone CLI archives.
- [ ] Bundle the CLI with every desktop installer.
- [ ] Add PATH/symlink installation flows.
- [ ] Generate checksums, completions, and man pages.
- [ ] Run install/uninstall smoke tests.

**Gate:** A clean machine on every supported OS can install, run, upgrade, and
remove the CLI without damaging desktop data.

### Phase 9 — Hardening and documentation

- [ ] Complete security review and threat model.
- [ ] Run fault-injection, SQLite contention, and interrupted-write tests.
- [ ] Verify accessibility and terminal compatibility.
- [ ] Finish user and automation documentation.
- [ ] Run release-candidate dogfooding against real repositories.
- [ ] Run the CLI-Anything refinement/validation review and resolve every
      accepted desktop-to-CLI capability gap before release.
- [ ] Generate and validate the shipped `skills/altai-cli/SKILL.md`.

**Gate:** All Definition of Done items below are satisfied.

## 15. Definition of Done

The CLI is complete only when all of the following are true:

- [ ] `altai agent` and `altai run` use the same IsanAgent engine and ALTAI
      workspace services as the desktop.
- [ ] `altai agent` preserves the complete IsanAgent TUI feature baseline and
      uses ALTAI adapters only for product-specific state and controls.
- [ ] The IsanAgent TUI uses the approved ALTAI terminal visual system, with
      dark/light/no-color modes and verified responsive snapshots.
- [ ] CLI-Anything analysis, `TEST.md`, validation evidence, and the generated
      `skills/altai-cli/SKILL.md` ship with the repository and release process.
- [ ] No agent-loop source is copied from IsanAgent.
- [ ] CLI and desktop can read each other's sessions, memory, checkpoints,
      jobs, notifications, automations, MCP configuration, and orchestration
      state.
- [ ] Runtime ownership prevents concurrent corruption.
- [ ] Interactive approvals and non-interactive failure behavior are tested.
- [ ] JSONL schema and exit codes are documented and protected by fixtures.
- [ ] Secrets never appear in argv by default, logs, JSONL, support bundles, or
      panic output.
- [ ] Legacy `altai <path>` desktop launching remains functional.
- [ ] The CLI runs without Tauri, WebKit, GTK, or frontend assets.
- [ ] macOS arm64/x64, Linux x64, and Windows x64 artifacts are published.
- [ ] Shell completions and man pages ship with releases.
- [ ] CI passes unit, integration, contract, PTY, and packaging smoke tests.
- [ ] Root installation and CLI documentation are updated.
- [ ] A clean install, upgrade, and uninstall have been verified on every
      supported OS.

## 16. Main Risks and Mitigations

| Risk | Mitigation |
|---|---|
| IsanAgent terminal host remains binary-only | Land the upstream library extraction before implementing the ALTAI runner. |
| Desktop and CLI runtime behavior drift | Share host construction and ALTAI core services; add parity contract tests. |
| Windows GUI binary cannot behave as a console app | Ship separate `altai` and `altai-desktop` executables. |
| Desktop settings are TypeScript-only | Move model metadata to a generated shared catalog and add a Rust preference loader. |
| Concurrent desktop and CLI processes corrupt a chat | Add chat-scoped cross-process leases and SQLite contention tests. |
| Machine-readable stdout is polluted | Centralize renderers and reserve stdout exclusively for the selected output contract. |
| Permission prompts deadlock in CI | Reject interactive approvals without a TTY and return exit code `4`. |
| Secrets leak through flags or diagnostics | Prefer secure prompt/env/store, use secret wrappers, and test redaction. |
| Tauri installers do not place a sidecar on PATH consistently | Define per-platform installer layouts and also publish standalone archives. |
| Branch dependency changes between builds | Publish the resolved IsanAgent revision and build CLI/desktop from one lockfile. |
| CLI-Anything generated Python code diverges from Rust services | Use CLI-Anything as an immutable methodology/validation dependency; never ship generated code as the runtime. |
| ALTAI web colors render inconsistently in terminals | Generate terminal palettes from semantic tokens and maintain truecolor, 256-color, and no-color snapshots. |

## 17. First Pull Request Boundary

The first ALTAI repository pull request should be deliberately non-featureful:

1. add the Cargo workspace structure;
2. add `altai-core` with event envelope, output schema types, configuration
   precedence types, and tests;
3. add the generated shared model catalog;
4. pin CLI-Anything, add the analysis/TEST artifacts, and establish the
   terminal palette token source;
5. add `altai-cli` with `help`, `version`, `completion`, and a dry-run desktop
   router;
6. do not start a real agent yet;
7. keep the existing desktop binary and behavior intact until the upstream
   IsanAgent host API is available.

This boundary validates build, dependency, command, and packaging decisions
without mixing them with the runtime extraction.
