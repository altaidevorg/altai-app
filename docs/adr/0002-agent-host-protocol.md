# ADR 0002: Use a versioned JSON-RPC stdio protocol for the agent host

Date: 2026-07-30

Status: Accepted, amended for control-plane scope (2026-08-03)

## Context

The VS Code client needs a machine-facing, reconnectable interface to the
shared Rust service. The current CLI exposes structured JSON and JSONL for
one-shot use, but stdout can represent final presentation output and does not
provide a persistent request surface. The Desktop renderer consumes a richer
versioned event envelope and can replay journaled run events.

The protocol must work when the extension runs beside a Remote SSH, WSL, or
Dev Container workspace; it must therefore use the Rust binary running on the
workspace extension host, not a browser or UI-side process.

## Decision

`altai-cli serve --stdio --protocol 1 --workspace <absolute-path>` will speak
JSON-RPC 2.0 with LSP-style `Content-Length` framing.

- Stdout is exclusively framed protocol bytes for the entire process lifetime.
- Stderr is redacted diagnostics only; it is never parsed for behavior.
- Every request has a JSON-RPC id and returns structured success/error data.
- `initialize` negotiates minimum/maximum protocol versions, host version,
  platform, and capabilities before requests are processed.
- Frames and JSON decoding have explicit header, payload, attachment, and
  nesting limits. The framing implementation is independent from stdio so it
  can be byte-buffer tested and fuzzed.
- Unknown optional fields are ignored/preserved as documented. An unsupported
  required version fails with a typed upgrade error; clients never branch on
  English error text.

Run notifications use `run/event`. Every run-scoped event contains non-empty
`chat_id`, non-empty `run_id`, and an increasing per-run `seq`. Payloads
converge on the Desktop protocol-v1 vocabulary, including `run_started`,
`thinking`, `agent_message`, tool events, `edit_diff`, `clarification`,
`usage`, execution/background/subagent/notification events, warnings, and
`run_terminated`.

The initial request names are:

```text
initialize                 workspace/status        config/get
models/list                agents/list             sessions/list
sessions/get               sessions/create         run/start
run/steer                  run/cancel              run/replay
clarification/respond      context/compact         checkpoints/list
checkpoints/restore        shutdown
```

The initial notifications are `run/event`, `workspace/changed`, `host/log`,
and `host/status`. A host advertises only capabilities it implements; before
TVS-05, the spike may advertise a deliberately small subset.

Cancellation is an explicit `run/cancel` request scoped by `chat_id` and
`run_id`, not child-process termination. Reconnect recovery uses the journal
and `run/replay` after the client detects a sequence gap.

## Amendment 2026-08-03: Control-plane domains

The protocol decided above covers run control. Its scope is broadened: the
same versioned JSON-RPC protocol will also carry the control-plane domains.
Module CP-15 of `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md` will
add these method groups in addition to the run-control methods listed above:

```text
organizations/*   goals/*       projects/*    agents/*
work/*            attempts/*    routines/*    approvals/*
budgets/*         activity/*    integrations/*  plugins/*
```

This amendment broadens scope only; every existing decision is preserved:

- The framing, version negotiation, typed-error, redaction, and capability
  rules decided above apply unchanged to the control-plane method groups.
  New groups are introduced through the same `initialize` version/capability
  negotiation; an unsupported required version still fails with a typed
  upgrade error.
- The run-control methods and `run/event` notification vocabulary of
  protocol v1 remain valid and are not renamed or removed.
- The protocol still transports neither provider keys nor implicit
  filesystem authority. Control-plane requests carry explicit identities
  (parent plan §3.3) and are authorized by the service, never by the client.
- Authoritative control-plane state transitions are performed only by the
  control-plane owner (parent plan §3.1). The protocol is how clients
  request those transitions, not a second owner.

## VS Code boundary

The extension declares `extensionKind: ["workspace"]`. It starts a host only
for a trusted canonical filesystem workspace and only on first ALTAI action.
Pure web and virtual workspaces cannot execute agents in v1. The webview sends
typed intentions to the extension host; it has no Node, filesystem, process,
or raw VS Code API capability.

## Consequences

- The protocol is a public compatibility boundary with Rust and TypeScript
  golden fixtures beginning in TVS-01.
- Logs cannot corrupt the protocol stream; fake credentials and authorization
  headers are included in later stderr-redaction tests.
- The protocol transports neither provider keys nor implicit VS Code filesystem
  authority. Context is explicitly bounded and authorized by the extension
  host/service.
- Process management (restart/backoff, handshake timeout, clean shutdown,
  architecture mismatch) remains the VS Code extension's responsibility.

## Rejected alternatives

1. Newline-delimited JSON: rejected because split/multiple frame handling,
   binary-sized attachments, and output corruption boundaries are weaker.
2. Tauri events or a local HTTP server: rejected because the first is not a
   reusable CLI/Remote boundary and the second adds listener/authentication
   exposure unnecessary for a parent-child local transport.
3. Parsing pretty/JSONL CLI output: rejected because presentation output is
   not a request protocol and cannot model long-lived control/replay safely.

## Follow-up

TVS-01 defines schemas, typed errors, fixtures, and independent framing tests.
TVS-02 proves split headers/bodies, multiple frames, EOF, invalid versions,
and ordered lifecycle events using the compiled CLI. TVS-05 replaces that
spike with the persistent shared service.
