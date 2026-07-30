# ALTAI VS Code Extension Plan

Date: 2026-07-30

Status: Proposed

Scope: A first-party VS Code extension that exposes ALTAI's existing local
agent runtime inside VS Code without creating a second agent implementation.

## 1. Outcome

Ship `ALTAI for VS Code`: a local-first coding agent with the interaction
quality users expect from Claude Code, Codex, Cline, and Kilo, while retaining
ALTAI's own runtime, provider support, permission modes, checkpoints, skills,
MCP configuration, sessions, and orchestration data.

The first public release must provide:

- an ALTAI Activity Bar destination with a rich chat view;
- durable conversations that can also be opened by ALTAI Desktop and CLI;
- streaming reasoning, tool activity, usage, and run state;
- `ask`, `auto-edit`, and `plan` permission modes;
- explicit command and edit approvals;
- native VS Code diff review and checkpoint restore;
- editor/explorer commands for explaining, fixing, refactoring, and reviewing;
- `@file`, current selection, diagnostics, open editors, and Git diff context;
- model, agent, permission, and workspace selection;
- cancellation, steering, retry, and crash/reload recovery;
- macOS, Linux, Windows, Remote SSH, WSL, and Dev Container support;
- platform-specific VSIX and Marketplace packages.

`bypass` remains an advanced, separately unlocked mode. It is not a first-run
or workspace-controlled default.

## 2. Product decision

### 2.1 One engine, three clients

ALTAI Desktop, the terminal CLI, and VS Code must use the same Rust service and
the same IsanAgent reasoning loop:

```text
                              +----------------------+
ALTAI Desktop (Tauri)  ------>|                      |
ALTAI CLI/TUI          ------>|  ALTAI Agent Service |----> IsanAgent
VS Code extension      ------>|                      |----> tools / MCP / models
                              +----------+-----------+
                                         |
                                         +----> .isanagent state + SQLite journal
                                         +----> ALTAI secret backend
                                         +----> checkpoints / skills / config
```

The VS Code extension is an editor-native client and process supervisor. It
must not:

- reimplement the agent loop in TypeScript;
- call provider APIs from the webview;
- parse human-readable CLI output as an API;
- create a VS Code-only session database;
- make the webview responsible for file or shell access;
- send API keys, workspace files, or prompts to an ALTAI cloud service.

### 2.2 UI strategy

Use VS Code-native surfaces whenever they are a better fit, and a single
Webview View where a rich transcript/composer is necessary:

| Need | Surface |
| --- | --- |
| Chat transcript, composer, tool cards, plan, approval cards | Webview View |
| Review proposed or completed file changes | `vscode.diff` and text documents |
| Files changed, Work, Inbox, sessions | Tree Views, then contextual detail |
| Explain/fix/refactor/review | Commands and editor/explorer context menus |
| Model/agent/permission choice | View title actions or Quick Pick |
| Current run state | View badge and one restrained Status Bar item |
| Configuration | VS Code Settings and ALTAI secret-management commands |
| Logs and diagnostics | `OutputChannel` with redaction |

Do not port the full Tauri application into a webview. The desktop editor,
terminal, Git panel, notebook, and settings screens would duplicate VS Code
and create two competing IDEs inside one window.

### 2.3 Initial compatibility boundary

- Desktop Node extension host: supported.
- Remote SSH, WSL, and Dev Containers: supported by running the extension and
  Rust host where the workspace lives.
- Codespaces with a remote Node extension host: supported after the Linux host
  package is verified.
- Pure `vscode.dev` / `github.dev`: not supported initially because a browser
  extension cannot spawn the local Rust agent host.
- Virtual workspaces: read-only messaging should fail closed in v1; agent
  execution requires a local/remote filesystem workspace.
- VS Code is the release target. Cursor and other VS Code-compatible editors
  are smoke-tested later but are not allowed to constrain the v1 architecture.

## 3. Current repository findings

The repository already contains most of the hard product logic:

- `src-tauri/src/altai/agent/runtime.rs` owns routing, concurrent chats,
  cancellation, steering, approvals, replay, notifications, automations,
  checkpoints, MCP, model failover, and the rich event stream.
- `src-tauri/crates/altai-core` is already Tauri-independent and contains
  workspace, policy, config, compaction, event, palette, and journal
  primitives.
- `src-tauri/crates/altai-cli` already has interactive and one-shot commands,
  structured output, permission mapping, attachments, and a host adapter.
- `src-tauri/crates/altai-cli/src/run_output.rs` maps IsanAgent bus messages to
  structured JSONL events.
- `src/modules/ai/lib/agentEventBridge.ts` defines the richer desktop
  protocol-v1 event vocabulary and validation behavior.
- `src/modules/ai/store/agentRunsStore.ts` contains a useful per-run reducer
  for run state, tool checks, changes, subagents, results, and usage.
- `src/components/ai-elements` contains transcript, markdown, reasoning, tool,
  code, and todo presentation components that can inform a shared UI package.

The main gaps are architectural:

1. The reusable CLI host is currently centered on TUI/one-shot execution.
2. The full long-lived `route_send/cancel/steer/replay` service is coupled to
   Tauri state and event emission.
3. CLI journal parity is still missing some rich bus events.
4. CLI release packaging and installed-binary CI are incomplete.
5. Credential access still needs a Tauri-independent secret-storage service.
   The current backend is a mode-0600 local file on macOS/Linux and Windows
   Credential Manager on Windows.
6. IsanAgent is temporarily path-pinned while its host API work is upstreamed.

These are prerequisites for a robust extension, not optional cleanup.

## 4. Target architecture

### 4.1 Repository layout

Introduce a small workspace without moving the existing application:

```text
extensions/
  vscode/
    package.json
    src/
      extension.ts
      commands/
      context/
      host/
      protocol/
      state/
      views/
      webview/
    media/
    test/
packages/
  agent-protocol/
  agent-ui/
src-tauri/crates/
  altai-core/
  altai-agent-service/
  altai-cli/
```

- `altai-agent-service`: long-lived, Tauri-independent application service
  extracted from the desktop runtime.
- `agent-protocol`: generated/validated TypeScript protocol types and golden
  fixtures.
- `agent-ui`: only the reusable transcript/composer/tool components that can
  operate behind a host adapter. It must not import Tauri or VS Code APIs.
- `extensions/vscode`: VS Code lifecycle, process management, editor context,
  native diffs, webview bridge, and packaging.

If extracting `agent-ui` blocks the first vertical slice, the extension may
start with a minimal dedicated webview. Protocol and runtime reuse are
mandatory; visual component reuse is not.

### 4.2 Agent host process

Add a machine-facing command:

```text
altai-cli serve --stdio --protocol 1 --workspace <absolute-path>
```

Use JSON-RPC 2.0 with LSP-style `Content-Length` framing:

- stdout contains protocol frames only;
- logs and crash diagnostics go to stderr;
- every request has a request ID;
- every run event contains `chat_id`, `run_id`, and monotonic `seq`;
- `initialize` negotiates protocol range, host version, platform, and
  capabilities;
- unknown event fields are tolerated; unknown required protocol versions fail
  with a useful upgrade message;
- cancellation is a first-class request, not a killed child process;
- the service performs journal catch-up after reconnect.

Minimum request surface:

```text
initialize
workspace/status
config/get
models/list
agents/list
sessions/list
sessions/get
sessions/create
run/start
run/steer
run/cancel
run/replay
clarification/respond
context/compact
checkpoints/list
checkpoints/restore
shutdown
```

Minimum notifications:

```text
run/event
workspace/changed
host/log
host/status
```

The event payload should converge on the desktop protocol-v1 vocabulary:
`run_started`, `thinking`, `tool_call_start`, `tool_call_end`, `edit_diff`,
`clarification`, `usage`, subagent events, execution events, warnings, and
`run_terminated`.

### 4.3 Process lifecycle

For v1, run one lazy host process per trusted workspace folder:

- start it only when the ALTAI view or command is first used;
- use an exact canonical workspace root;
- queue requests until `initialize` completes;
- apply bounded restart with backoff;
- on extension reload, inspect the journal and replay the active run;
- on normal deactivation, request graceful shutdown and cancel an in-window
  foreground run after a short deadline;
- never silently detach a process advertised as a background run.

Durable work that must survive VS Code closing belongs in a later
`altai daemon` milestone. A child process tied to the extension host must not
pretend to provide that guarantee.

For multi-root workspaces, each folder has an isolated host, state, policy, and
view context. The user must select the target folder before a run starts.

### 4.4 VS Code extension host

The extension is a workspace extension (`extensionKind: ["workspace"]`) so the
Rust process executes beside local or remote workspace files.

Core services:

- `HostManager`: resolves and supervises the matching Rust binary.
- `RpcClient`: framing, schema validation, request cancellation, reconnect.
- `WorkspaceRegistry`: canonical roots and multi-root selection.
- `SessionStore`: view projections only; durable truth remains in Rust.
- `RunReducer`: consumes the shared event contract and detects sequence gaps.
- `ContextCollector`: selection, current file, explicit files, diagnostics,
  visible/open editors, and Git diff.
- `DiffController`: opens read-only original/proposed documents through
  `vscode.diff` and routes approval responses.
- `ChatViewProvider`: webview lifecycle and strictly validated messages.
- `CommandRegistry`: command palette, editor title, editor context, and
  explorer context actions.

The webview never receives a filesystem capability or raw VS Code API handle.
It sends typed intentions such as `sendPrompt`, `cancelRun`, `openDiff`, and
`approveChoice`; the extension host authorizes and executes them.

### 4.5 Context model

MVP context sources:

- current selection with file URI and line range;
- current file or explicitly selected Explorer files;
- diagnostics in the current file or chosen workspace scope;
- currently visible editors;
- unstaged/staged Git diff through the built-in Git extension API when
  available, with a CLI fallback;
- images or PDFs selected through a VS Code file picker;
- bounded folder summaries produced by the Rust host.

All context must be explicit and visible as removable composer chips. Apply
size caps before a prompt crosses the extension-to-host boundary.

Do not promise arbitrary integrated-terminal scrollback in v1. VS Code's
stable extension surface does not expose a general terminal buffer. The ALTAI
agent can still run and observe its own shell tools; users can paste terminal
output as selected context.

## 5. User experience

### 5.1 Activity Bar view

One ALTAI container with no more than three views:

1. **Chat** — default, custom webview.
2. **Work** — native Tree View for active, attention, review, and history.
3. **Inbox** — native Tree View for approvals, clarifications, and results.

Chat header:

- session switcher/new chat;
- active agent and model;
- permission mode;
- compact context/token indicator;
- run state and stop action.

Composer:

- multiline input;
- `@` file/context picker;
- `/` command picker;
- visible context chips;
- attach selection/file/diff/diagnostics;
- send, queue, steer, and stop states;
- accessible keyboard behavior and screen-reader announcements.

Transcript:

- user and assistant messages;
- collapsible reasoning;
- tool start/progress/result cards;
- todos/plan;
- usage and run outcome;
- edit or shell approval cards;
- changed-file summary with native diff links;
- retry/continue affordance for recoverable outcomes.

### 5.2 Commands

Initial public commands:

```text
ALTAI: Open Chat
ALTAI: New Chat
ALTAI: Ask About Selection
ALTAI: Explain Selection
ALTAI: Fix Selection
ALTAI: Refactor Selection
ALTAI: Review File
ALTAI: Review Git Changes
ALTAI: Add File to Context
ALTAI: Stop Current Run
ALTAI: Change Model
ALTAI: Change Agent
ALTAI: Change Permission Mode
ALTAI: Restore Checkpoint
ALTAI: Open Logs
ALTAI: Run Doctor
```

The editor and Explorer menus should expose only the context-relevant subset.
No command may bypass the same Rust permission policy used by chat.

### 5.3 Diff and approval flow

1. The runtime emits `edit_diff` or a clarification with diff metadata.
2. The extension registers original/proposed read-only content URIs.
3. `vscode.diff` opens the native editor diff.
4. The Chat and Inbox surfaces show Approve, Deny, and the runtime-supported
   alternatives.
5. The answer goes through `clarification/respond`.
6. The runtime applies or rejects the edit and emits the next sequenced event.
7. A checkpoint is listed before any approved mutation.

The extension must never infer approval from closing a diff tab.

### 5.4 Later VS Code AI integration

After the standalone ALTAI experience is stable:

- add an `@altai` Chat Participant as a second entry point into the same
  service;
- optionally expose selected ALTAI capabilities as Language Model Tools;
- add inline completion through `InlineCompletionItemProvider` as a separate,
  latency-budgeted subsystem;
- add code actions for diagnostics;
- add notebook-cell context and experiment views.

These do not block v1 and must not fork session or permission semantics.

## 6. Security and privacy

### 6.1 Workspace Trust

Declare limited support for untrusted workspaces:

- session browsing and static help may remain available;
- agent execution, shell, edits, MCP startup, workspace hooks, and binary-path
  overrides are disabled until the workspace is trusted;
- both `when` clauses and runtime checks enforce the boundary;
- security-sensitive settings are listed under `restrictedConfigurations`.

Workspace settings may select a checked-in ALTAI agent profile, but may not:

- choose an arbitrary executable path;
- inject host environment variables;
- turn on `bypass`;
- weaken secret-file or symlink checks;
- install or start an MCP server without the applicable permission flow.

### 6.2 Credentials

Preserve ALTAI's invariant that provider keys do not enter the webview.

Preferred implementation:

- extract the existing secret implementation behind a Tauri-independent Rust
  trait while preserving its actual platform behavior: mode-0600 local storage
  on macOS/Linux and Windows Credential Manager on Windows;
- the extension can query connected/missing status only;
- API-key entry is performed by an interactive Rust `altai auth` flow or
  provider OAuth/device flow;
- secrets are never written to VS Code settings, workspace files, logs,
  protocol events, telemetry, or webview state.

VS Code `SecretStorage` is an acceptable fallback only after an explicit
security decision, because it would place the plaintext key in the extension
host process even though it remains encrypted at rest.

### 6.3 Webview and process boundary

- strict Content Security Policy with a per-render nonce;
- `localResourceRoots` restricted to extension assets;
- no remote scripts, inline event handlers, or arbitrary command URIs;
- sanitize model-produced Markdown and links;
- validate every webview message and every Rust protocol frame;
- redact secrets, authorization headers, and sensitive paths from logs;
- keep protocol payload and attachment limits;
- ship signed/checksummed first-party Rust binaries;
- executable override is user/global scope only and shows a persistent
  non-default-host indicator.

No prompt/content telemetry by default. If product analytics are added later,
they are opt-in and limited to coarse feature events.

## 7. Implementation sequence

### M0 — Contracts and spike

Goal: prove one real prompt can stream from IsanAgent into an Extension
Development Host.

Deliver:

- capability matrix from Desktop to VS Code;
- protocol v1 schema, handshake, error model, and golden fixtures;
- `altai-cli serve --stdio` spike with a scripted provider;
- minimal VS Code command and Output Channel client;
- architecture decision records for process lifecycle, credentials, binary
  distribution, remote extension placement, and webview choice.

Exit gate:

- a VS Code integration test receives ordered `run_started`, assistant/tool
  events, and `run_terminated`;
- stdout corruption and incompatible protocol versions fail deterministically.

### M1 — Extract the reusable agent service

Goal: remove Tauri as a requirement for a long-lived agent session.

Deliver:

- `altai-agent-service` crate;
- adapters for Desktop event emission and stdio JSON-RPC;
- shared send, cancel, steer, replay, compaction, session, and clarification
  APIs;
- Tauri desktop migrated to the service without observable behavior changes;
- rich journal parity for all run-scoped events;
- Tauri-independent secret-storage facade;
- removal of the temporary IsanAgent path pin when upstream permits.

Exit gate:

- the same contract suite runs against Desktop adapter and stdio adapter;
- existing desktop agent tests pass;
- CLI and Desktop can open the same durable session.

### M2 — Extension shell and host management

Goal: make install, activation, trust, and process lifecycle reliable.

Deliver:

- `extensions/vscode` package and lazy activation;
- Activity Bar container, Chat placeholder, settings, commands, context keys;
- platform/architecture host resolution;
- host handshake, restart/backoff, Output Channel, doctor command;
- multi-root selection;
- untrusted and virtual workspace gates;
- remote extension configuration.

Exit gate:

- activation does not spawn a host;
- first command starts exactly one host for the selected workspace;
- reload/reconnect and malformed-frame tests pass.

### M3 — Chat MVP

Goal: complete a useful read/plan conversation inside VS Code.

Deliver:

- themed, accessible Chat Webview View;
- new/list/resume session;
- streaming assistant, reasoning, tool, usage, and outcome events;
- composer, file/selection/diagnostic/Git context;
- model, agent, and permission selectors;
- cancel, retry, steering, and manual compaction;
- protocol sequence-gap recovery from the journal.

Exit gate:

- a plan-mode repository task survives webview reload without duplicate or
  missing transcript events;
- no webview message can directly access the filesystem or process API.

### M4 — Editing, approvals, and recovery

Goal: safely complete real coding tasks.

Deliver:

- `ask` and `auto-edit`;
- native diff review;
- shell and edit clarifications;
- checkpoint list/restore;
- changed-file and verification summaries;
- selection/editor/explorer commands;
- save/external-change conflict behavior;
- optional bypass unlock with explicit warnings.

Exit gate:

- deny produces no mutation;
- approve applies only the reviewed operation;
- checkpoints restore create/modify/delete cases;
- symlink, secret-file, traversal, workspace-root, and untrusted-workspace
  security tests pass.

### M5 — Work, Inbox, and durable operations

Goal: expose ALTAI's work model beyond the focused chat.

Deliver:

- native Work and Inbox Tree Views based on the unified work-surface model;
- background job and subagent projections;
- approval/clarification deep links;
- notifications and status badges;
- automation browsing and run-now;
- daemon design for work that genuinely survives editor closure.

Exit gate:

- one run has one canonical identity across Chat, Work, and Inbox;
- resolving Inbox never deletes the underlying run;
- active work is recovered after extension-host reload.

### M6 — Packaging and public beta

Goal: publish a supportable extension.

Deliver:

- Rust host artifacts for supported Marketplace targets;
- platform-specific VSIX build and Marketplace publishing;
- SBOM, checksums, license notices, and release provenance;
- macOS, Windows, Linux, WSL, Remote SSH, and Dev Container CI;
- offline/error/update flows;
- accessibility and performance audit;
- privacy, security, troubleshooting, and enterprise deployment docs;
- optional Open VSX publishing after Marketplace validation.

Exit gate:

- clean-machine install to first successful plan run;
- upgrade and rollback tests;
- Marketplace package contains only the intended platform binary;
- no high-severity security or accessibility findings.

### M7 — Competitive follow-ups

- `@altai` Chat Participant;
- Language Model Tools;
- inline completion;
- diagnostic code actions;
- notebook and experiment integration;
- MCP/skills management UI;
- orchestration graphs and multi-agent assignment;
- daemon-backed scheduled/background work;
- compatibility testing for Cursor and other VS Code forks.

## 8. Test strategy

### Rust

- service unit tests for routing, policy, credentials, lifecycle, and replay;
- JSON-RPC framing/fuzz tests and payload limits;
- journal crash/restart tests;
- real workspace confinement and symlink tests;
- scripted-provider deterministic agent tests.

### TypeScript

- protocol decoder and forward-compatibility tests;
- run reducer and sequence-gap tests;
- context size/redaction tests;
- multi-root routing tests;
- webview message validation and CSP snapshot tests;
- command/menu/context-key tests.

### Extension integration

Use `@vscode/test-electron` with a real compiled Rust test host:

- activation and lazy start;
- send/cancel/steer/replay;
- session resume;
- native diff and approval;
- Workspace Trust transition;
- extension-host reload;
- multi-root;
- theme, keyboard, and screen-reader flows.

Run cross-platform CI on Windows, macOS, and Linux. Add WSL/Remote/Container
smoke coverage before beta.

## 9. Release metrics

Measure locally and report only if the user opts in:

- extension activation time;
- time to host readiness;
- time to first streamed event;
- run completion/cancel/failure categories;
- approval round-trip success;
- reconnect/replay success;
- host crash rate;
- peak extension-host and Rust-host memory.

Suggested beta targets:

- extension activation under 100 ms without starting Rust;
- warm host handshake under 300 ms;
- no dropped or duplicated run events across reload;
- no plaintext secrets in logs or webview storage;
- zero unapproved file mutations in the security suite.

## 10. Staffing and rough schedule

Assuming two engineers who can work across Rust and TypeScript:

| Work | Estimate |
| --- | --- |
| M0 contracts/spike | 1 week |
| M1 reusable service | 2–3 weeks |
| M2 extension shell | 1 week |
| M3 chat MVP | 2 weeks |
| M4 edits/approvals | 2 weeks |
| M5 Work/Inbox | 1–2 weeks |
| M6 beta hardening | 2 weeks |

Expected first private MVP: 6–8 weeks.

Expected public beta: 10–12 weeks.

A solo implementation is more realistically 14–18 weeks. The dominant
uncertainty is the runtime extraction and cross-platform host packaging, not
the React chat UI.

## 11. Recommended first vertical slice

Do not begin with the polished sidebar.

Build this slice first:

1. extract a minimal `AgentService::start_run/cancel/subscribe`;
2. expose it through `altai-cli serve --stdio`;
3. stream protocol-v1 events to an Output Channel;
4. open a basic Chat Webview and render those events;
5. reload the Extension Development Host and replay from SQLite;
6. run the same prompt through Desktop and verify shared session visibility.

If this slice works, the project has validated its hardest architectural
claims. Everything after it is incremental product work.

## 12. Go/no-go checklist

Start implementation when:

- protocol v1 and ownership are accepted;
- Desktop runtime extraction scope is accepted;
- supported OS/architecture matrix is chosen;
- API-key entry policy is chosen;
- foreground-only process semantics for v1 are accepted;
- Marketplace publisher and signing ownership are identified.

Do not publish beta until:

- Workspace Trust and secret-path tests pass;
- binary provenance and update behavior are documented;
- reload/replay is deterministic;
- diff approval is race-safe;
- accessibility QA covers keyboard, high contrast, reduced motion, and screen
  reader announcements;
- the extension can diagnose an incompatible or missing host without asking
  users to inspect developer tools.

## 13. Relevant VS Code constraints

- Extension hosts and remote placement:
  <https://code.visualstudio.com/api/advanced-topics/extension-host>
- Supporting Remote Development:
  <https://code.visualstudio.com/api/advanced-topics/remote-extensions>
- Workspace Trust:
  <https://code.visualstudio.com/api/extension-guides/workspace-trust>
- Webview UX guidance:
  <https://code.visualstudio.com/api/ux-guidelines/webviews>
- Chat Participant API:
  <https://code.visualstudio.com/api/extension-guides/ai/chat>
- Language Model Tool API:
  <https://code.visualstudio.com/api/extension-guides/ai/tools>
- Platform-specific VSIX publishing:
  <https://code.visualstudio.com/api/working-with-extensions/publishing-extension>
- Extension integration testing:
  <https://code.visualstudio.com/api/working-with-extensions/continuous-integration>
