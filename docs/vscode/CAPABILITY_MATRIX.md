# ALTAI Desktop-to-VS-Code capability matrix

Baseline inspected: 2026-07-30. “Current Desktop evidence” names the source
of truth, not a claim that the VS Code behavior already exists.

| v1 capability | Current Desktop evidence | Target VS Code surface | Delivery / gate |
| --- | --- | --- | --- |
| Durable chat sessions | `agent_list_sessions` and `agent_get_session_messages`; runtime uses durable `tauri:<chat_id>:` identities | Chat session switcher; view-only `SessionStore` | TVS-04/05/07; shared session contract |
| Start and stream a run | `route_send`; `Event` / versioned `AgentEventEnvelope`; bridge validates protocol-v1 events | Chat Webview transcript and one status indicator | TVS-01/05/07; ordered run integration test |
| Reasoning and assistant output | `Thinking`, `AgentMessage` events and `agentEventBridge.ts` schemas | Collapsible reasoning and assistant message cards | TVS-07; Webview/reducer tests |
| Tool activity | `ToolCallStart` / `ToolCallEnd` events | Streaming tool cards | TVS-07; fixture and UI tests |
| Usage and run outcomes | `Usage`, `RunWarning`, `RunTerminated`; run reducer | Usage/outcome summary | TVS-07; sequence/outcome tests |
| Cancel and steer | `route_cancel` and `route_steer` require exact run identity | Stop and steer actions | TVS-04/05/07; stale-ID and terminal-order tests |
| Replay after reload | `replay_run_events` and replay cursor use the SQLite journal | Host reconnect and transcript recovery | TVS-04/05/07; reload/no-duplicates integration test |
| Plan / ask / auto-edit policy | Runtime builds IsanAgent permission configuration; current command forwards `permission_mode` | Permission Quick Pick and visible mode | TVS-07/09; service policy tests; `bypass` deferred/unlocked |
| Clarifications and edit proposals | `Clarification`, `EditDiff`, and existing chat-reply flow; legacy ID approval explicitly rejects | Inbox/chat approval card plus native diff | TVS-05/09; explicit `clarification/respond` tests |
| File diff review | Runtime emits before/after diff data | Read-only content URIs and `vscode.diff` | TVS-09; stale-content and deny-no-mutation tests |
| Checkpoints | Runtime owns checkpoints and related events | Checkpoint list/restore command | TVS-05/09; create/modify/delete restore tests |
| Editor/explorer context | Desktop accepts images/documents; workspace-aware runtime | Commands plus removable context chips: selection, file, diagnostics, editors, Git diff, image/PDF | TVS-08; limits, multi-root, URI/security tests |
| Git review | Runtime tools/workspace context; no VS Code-specific adapter yet | “Review Git Changes” command; built-in Git API then bounded fallback | TVS-08; staged/unstaged and unavailable-Git tests |
| Work / Inbox | Runtime emits background, execution, subagent, and notification events | Native Tree Views; deep-link to run/session/diff | TVS-10; canonical-identity/recovery tests |
| MCP, skills, automations | Runtime owns MCP and automations; Desktop has workspace workflow concepts | Reuse Rust policy; automation browse/run-now only | TVS-05/10; no workspace-controlled MCP start; management UI deferred |
| Provider credentials | Desktop secret module; runtime redaction tests; current CLI takes configured one-shot host input | Rust-auth/status commands only; no settings/webview secret entry | TVS-03/05/12; redaction/secret-boundary tests |
| Logs and doctor | CLI has `doctor`; runtime/CLI report diagnostics to stderr | Redacted OutputChannel; Run Doctor command | TVS-06/11; missing/incompatible/crashed-host tests |
| Workspace Trust | Desktop authorizes workspace at command boundary | Restricted Mode gate in contributions and HostManager | TVS-06/09/11; no host/process/edit/MCP in untrusted workspace |
| Multi-root and Remote | Runtime has workspace service separation but no VS Code manager | One canonical host per selected folder; workspace extension runs host remotely | TVS-06/08/11; multi-root and Remote placement tests |
| Platform packaging | Existing CI compiles Rust on macOS/Windows/Linux; release builds Desktop targets only (macOS arm64/x64, Linux x64, Windows x64) | Target-specific VSIX with exactly one bundled host binary | TVS-11; archive allowlist and CI evidence |

## Deliberately not v1 capabilities

- Pure `vscode.dev` / `github.dev` execution and virtual-workspace agent runs.
- General integrated-terminal scrollback capture.
- A foreground run surviving full VS Code shutdown; that needs a daemon.
- Chat Participant, Language Model Tools, inline completion, notebook support,
  and compatibility guarantees for Cursor/Open VSX.

## Baseline gaps that must not be hidden

The CLI’s `run_output.rs` produces structured JSONL for a one-shot host, while
`journal_sink.rs` persists only lifecycle start/terminal events. It does not
yet provide the full persistent protocol or rich event journal parity. The
repository CI has a CLI installed-style smoke test on Linux and cross-platform
Rust compile checks, but no VSIX/Remote integration workflow. These are
implementation gates rather than evidence of shipped extension support.
