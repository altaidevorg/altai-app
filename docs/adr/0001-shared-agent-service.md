# ADR 0001: Share one Rust agent service across Desktop, CLI, and VS Code

Date: 2026-07-30

Status: Accepted, amended for control-plane scope (2026-08-03)

## Context

ALTAI currently has one rich agent implementation in
`src-tauri/src/altai/agent/runtime.rs`. It owns run routing, concurrent chat
isolation, cancellation, steering, clarifications, replay, checkpoints,
notifications, background work, model failover, and the Desktop event stream.
Its public Tauri commands call `route_send`, `route_cancel`, `route_steer`,
session APIs, and replay APIs directly.

Some state is already reusable: `altai-core` owns workspace, policy, config,
and SQLite journal primitives. The CLI has a Tauri-independent one-shot host,
maps IsanAgent bus messages to JSONL in `run_output.rs`, and appends only a
small lifecycle subset to the same journal through `journal_sink.rs`. It is
not a replacement for the Desktop long-lived runtime.

Creating a second TypeScript agent loop or treating human-oriented CLI output
as a host API would fork permission decisions, durable session identity, event
ordering, and recovery behavior.

## Decision

Introduce `altai-agent-service`, a Tauri-independent Rust application-service
crate. It will become the sole owner of the long-lived IsanAgent lifecycle and
the durable workspace services used by all clients.

```text
Desktop Tauri adapter ─┐
CLI stdio adapter     ├─> altai-agent-service ─> IsanAgent / tools / MCP
VS Code host adapter  ┘             │
                                     ├─> .isanagent state + SQLite journal
                                     └─> shared secret-storage facade
```

The service API will cover send, cancel, steer, manual compaction, session
listing/history, replay cursor/events, clarification response, checkpoints,
and graceful shutdown. Tauri commands become thin authorization and
serialization adapters; the CLI stdio server becomes another adapter. There
is exactly one agent loop and one durable workspace/session source of truth.

The Desktop event envelope remains compatible during extraction:
`version`, `scope`, `chatId`, `runId`, `seq`, and the event payload accepted by
`src/modules/ai/lib/agentEventBridge.ts`. Event sequencing is service-owned;
the journal is authoritative for replay.

## Lifecycle and ownership

- In v1, a VS Code client starts one lazy host process per trusted, canonical
  workspace folder. Multi-root folders are isolated and selected explicitly.
- Foreground runs belong to that extension-host child process. On normal
  deactivation it receives a bounded graceful shutdown/cancel request; it must
  not be described as surviving full editor shutdown.
- Work that must outlive VS Code requires a separately designed daemon and is
  out of scope for v1.
- Existing Desktop storage identities such as `tauri:<chat_id>:` stay readable
  through an alias/migration boundary; no client gets a separate database.

## Amendment 2026-08-03: Control-plane scope

The statement above that "work that must outlive VS Code requires a separately
designed daemon and is out of scope for v1" is amended. That separately
designed daemon now exists as an approved direction: the user-scoped
`altai-control-plane` daemon (module CP-16 in
`docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md`). Durable work —
organizations, goals, projects, work items, attempts, routines, approvals,
budgets, and audit — is owned by that daemon and outlives any renderer,
including the VS Code extension host and the Desktop webview.

This amendment expands lifecycle and ownership scope only:

- `altai-agent-service` remains the sole owner of the long-lived IsanAgent
  lifecycle and the durable workspace services, exactly as decided above. The
  control plane requests execution through it; it does not replace it.
- The control-plane daemon is the single authoritative owner of work
  lifecycle mutations (parent plan §3.1). `altai-agent-service` and all
  renderers may request transitions; they may not perform authoritative
  transitions independently.
- The per-workspace VS Code host process rules in `Lifecycle and ownership`
  still apply to run execution. What changes is that renderer shutdown no
  longer bounds the lifetime of durable work, scheduled routines, or recovery
  state, because those live in the control-plane daemon.
- The Security boundary section is unchanged. Rust — not the webview or
  TypeScript — continues to own providers, filesystem, shell, MCP,
  checkpoints, credentials, and durable state. The control-plane daemon
  inherits this boundary; no secret or provider access moves into any client.

## Security boundary

- Rust, not the webview or TypeScript, accesses providers, filesystem, shell,
  MCP, checkpoints, and durable state.
- Provider keys never enter VS Code settings, workspace settings, webview
  state, protocol fixtures, logs, or telemetry. The service exposes only
  status where needed.
- The existing secret implementation must first be placed behind a
  Tauri-independent facade while preserving its behavior: mode-0600 local
  storage on macOS/Linux and Windows Credential Manager on Windows.
- Workspace Trust is enforced both in VS Code contribution visibility and in
  host/runtime authorization. An untrusted workspace cannot start an agent,
  shell, edit, MCP, workspace hook, or workspace-selected executable.

## Consequences

Positive consequences:

- Desktop, CLI, and VS Code use the same permissions, sessions, checkpoints,
  journal, skills, MCP configuration, and run identities.
- Recovery/replay semantics can be tested once against Desktop and stdio
  adapters.
- The VS Code extension stays a small editor-native client/process supervisor.

Costs and constraints:

- Runtime extraction is the highest-risk part of the program and must retain
  existing concurrent-fingerprint and stale-run protections.
- `altai-cli` cannot claim parity until the shared service replaces its
  one-shot host path and its journal captures the rich event vocabulary.
- Current CI validates Rust on macOS/Windows/Linux and runs full tests on
  Linux; it does not yet prove the shared service, VSIX packaging, or Remote
  placement. Those gates are explicit later work, not completed support.

## Rejected alternatives

1. A standalone TypeScript agent/provider implementation: rejected because it
   forks policy, credentials, recovery, and session persistence.
2. Parse `altai run --jsonl` as the extension protocol: rejected because it is
   a one-shot output contract, lacks persistent request/cancel/steer/replay
   semantics, and has incomplete journal parity.
3. Keep the full service in Tauri and expose Tauri IPC to VS Code: rejected
   because Remote extension placement and standalone CLI hosting require a
   Tauri-independent process boundary.

## Follow-up and evidence

TVS-03 introduces the event sink and workspace-service seam; TVS-04 moves the
long-lived routes without copying the runtime; TVS-05 exposes the service over
stdio. TVS-06 may start only after TVS-05 is accepted.

Evidence: `runtime.rs`, `commands.rs`, `altai-core` journal exports,
`altai-cli/src/run_output.rs`, `altai-cli/src/journal_sink.rs`, and the
existing CI/release workflows inspected on 2026-07-30.
