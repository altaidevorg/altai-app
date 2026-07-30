# ALTAI VS Code Extension — Terra Execution Plan

Date: 2026-07-30

Status: Ready for implementation

Executor: GPT-5.6 Terra

Source architecture:
`docs/plans/2026-07-30-vscode-extension.md`

## 1. Purpose

This is the implementation runbook for Terra. It converts the product and
architecture plan into reviewable, sequential pull requests.

Terra must execute one task ID at a time. Do not give all task IDs to one
unbounded run. Start a fresh Terra run after each accepted PR so it receives
the current repository state instead of relying on a stale long conversation.

The critical path is:

```text
TVS-00 baseline
  -> TVS-01 protocol
  -> TVS-02 stdio spike
  -> TVS-03 service/event seam
  -> TVS-04 long-lived runtime extraction
  -> TVS-05 production stdio host
  -> TVS-06 extension shell
  -> TVS-07 chat MVP
  -> TVS-08 editor context
  -> TVS-09 approvals/diff
  -> TVS-10 Work/Inbox/recovery
  -> TVS-11 packaging/remote/security
  -> TVS-12 beta release
```

## 2. Terra operating contract

Every Terra run must:

1. Read this file and the source architecture plan completely.
2. Read every repository file named by the assigned task before editing.
3. Run `git status --short` before editing and preserve all unrelated user
   changes and untracked files.
4. Implement only the assigned task ID.
5. Use `apply_patch` for manual source edits.
6. Avoid a second agent loop, a second session database, and parsing
   human-oriented CLI output.
7. Add tests with the implementation, not in a later cleanup task.
8. Run the task's focused verification first, then its required regression
   gate.
9. Run `git diff --check` and review `git diff --stat` before handoff.
10. Report changed files, tests, remaining risks, and any acceptance criterion
    that is not proven.

Terra must not:

- use `git add -A`, destructive reset/checkout commands, or overwrite dirty
  files;
- change protocol fixtures without explaining compatibility impact;
- place credentials in TypeScript settings, webview state, logs, fixtures, or
  test snapshots;
- make a workspace setting select an executable or enable `bypass`;
- copy `src-tauri/src/altai/agent/runtime.rs` into a second runtime;
- claim Remote, Marketplace, accessibility, or security support without the
  corresponding test or release evidence.

Commit rule: one task ID is one PR. A task may use two commits only when the
first is a pure mechanical move and the second is behavior/tests. Never mix
unrelated formatting or refactoring.

## 3. Baseline commands

Terra uses these commands unless a task provides a narrower override:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli -- -D warnings
pnpm exec tsc --noEmit
pnpm lint
pnpm test
pnpm build
git diff --check
```

After new packages exist, add them explicitly rather than replacing focused
commands with an ambiguous workspace-wide command.

## 4. Task ledger

| ID | Deliverable | Hard dependency | Suggested commit |
| --- | --- | --- | --- |
| TVS-00 | Baseline, ADRs, capability/test matrix | none | `docs(vscode): lock extension architecture and gates` |
| TVS-01 | Versioned Rust/TypeScript protocol contract | TVS-00 | `feat(protocol): add agent host protocol v1` |
| TVS-02 | Framed stdio vertical-slice spike | TVS-01 | `feat(cli): prove framed stdio agent stream` |
| TVS-03 | Shared event sink and service shell | TVS-02 | `refactor(agent): introduce tauri-independent service seam` |
| TVS-04 | Long-lived run/session routing extraction | TVS-03 | `refactor(agent): move runtime routing into shared service` |
| TVS-05 | Production `altai-cli serve --stdio` | TVS-04 | `feat(cli): expose persistent agent host protocol` |
| TVS-06 | VS Code extension shell and host manager | TVS-05 | `feat(vscode): add extension shell and host lifecycle` |
| TVS-07 | Streaming Chat MVP | TVS-06 | `feat(vscode): add durable streaming chat` |
| TVS-08 | Editor context and commands | TVS-07 | `feat(vscode): add editor-native context actions` |
| TVS-09 | Native diff, approvals, checkpoints | TVS-08 | `feat(vscode): add safe edit approval workflow` |
| TVS-10 | Replay, Work, Inbox, durable projections | TVS-09 | `feat(vscode): add work inbox and recovery` |
| TVS-11 | Platform packaging, Remote, security gates | TVS-10 | `build(vscode): add platform VSIX release matrix` |
| TVS-12 | Public beta hardening and documentation | TVS-11 | `release(vscode): prepare public beta` |

## 5. TVS-00 — Baseline and architecture lock

### Goal

Record the current behavior and decisions before production refactoring.

### Files

Create:

```text
docs/adr/0001-shared-agent-service.md
docs/adr/0002-agent-host-protocol.md
docs/vscode/CAPABILITY_MATRIX.md
docs/vscode/TEST.md
```

Update only if necessary:

```text
docs/plans/2026-07-30-vscode-extension.md
docs/plans/2026-07-30-vscode-extension-terra-execution.md
```

### Required decisions

- Rust service is the only agent engine.
- JSON-RPC 2.0 uses LSP-style `Content-Length` framing.
- stdout is protocol-only; stderr is redacted diagnostics.
- one host process per trusted canonical workspace in v1.
- foreground runs do not pretend to survive editor shutdown.
- VS Code extension is `extensionKind: ["workspace"]`.
- pure web and virtual workspaces cannot execute agents in v1.
- provider keys stay out of the webview and VS Code settings.
- Desktop protocol-v1 event vocabulary is the convergence target.

The capability matrix must map every intended v1 feature to its current
Desktop command/event and desired VS Code surface.

### Acceptance

- No production source changes.
- Every later task has a named test row in `docs/vscode/TEST.md`.
- Open questions have an owner and a blocking/non-blocking label.

### Verify

```bash
git diff --check -- docs
rg -n "TBD|TODO|open question" docs/adr docs/vscode
```

### Terra prompt

```text
Implement TVS-00 from docs/plans/2026-07-30-vscode-extension-terra-execution.md.
This is documentation-only. Inspect the current runtime, CLI, event bridge,
CI, release workflow, and both VS Code plans. Lock the decisions, build an
evidence-backed Desktop-to-VS-Code capability matrix, and create a test matrix.
Do not edit production source or resolve unrelated dirty files.
```

## 6. TVS-01 — Protocol v1

### Goal

Create one versioned, testable protocol contract shared by Rust and the VS Code
client before either side depends on implementation details.

### Files

Create:

```text
src-tauri/crates/altai-protocol/Cargo.toml
src-tauri/crates/altai-protocol/src/lib.rs
src-tauri/crates/altai-protocol/src/frame.rs
src-tauri/crates/altai-protocol/src/message.rs
src-tauri/crates/altai-protocol/tests/fixtures.rs
shared/agent-protocol/v1/schema.json
shared/agent-protocol/v1/fixtures/*.json
packages/agent-protocol/package.json
packages/agent-protocol/tsconfig.json
packages/agent-protocol/src/index.ts
packages/agent-protocol/src/schema.ts
packages/agent-protocol/src/__tests__/fixtures.test.ts
```

Update:

```text
src-tauri/Cargo.toml
pnpm-workspace.yaml
pnpm-lock.yaml
```

### Contract

Requests:

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

Notifications:

```text
run/event
workspace/changed
host/log
host/status
```

All run events carry `chat_id`, `run_id`, and monotonic `seq`. Initialize
returns protocol min/max, host version, platform, and capability flags.

Keep framing independent from stdin/stdout so it can be fuzzed with byte
buffers. Enforce header, frame, attachment, and JSON nesting limits.

### Acceptance

- Rust and TypeScript accept every valid golden fixture.
- Both reject malformed headers, oversized frames, missing IDs, invalid
  versions, and malformed run identity.
- Unknown optional fields survive or are ignored as documented.
- The contract includes structured error codes; no client behavior depends on
  English error text.
- No provider key or prompt content appears in example fixtures.

### Verify

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml -p altai-protocol
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-protocol -- -D warnings
pnpm --filter @altai/agent-protocol test
pnpm --filter @altai/agent-protocol exec tsc --noEmit
git diff --check
```

### Terra prompt

```text
Implement TVS-01 only. Treat the protocol as a public compatibility boundary.
Add the Rust crate, TypeScript package, schema, and cross-language golden
fixtures. Do not start a process or add VS Code UI. Preserve existing
EventEnvelope/journal compatibility and document every intentional mapping.
Run the exact TVS-01 verification commands.
```

## 7. TVS-02 — Stdio vertical slice

### Goal

Prove the complete byte path before extracting the large desktop runtime:
framed request -> existing IsanAgent scripted one-shot host -> ordered framed
events -> terminal result.

### Files

Create:

```text
src-tauri/crates/altai-cli/src/serve/mod.rs
src-tauri/crates/altai-cli/src/serve/connection.rs
src-tauri/crates/altai-cli/src/serve/spike.rs
src-tauri/crates/altai-cli/tests/serve_stdio.rs
```

Update:

```text
src-tauri/crates/altai-cli/src/main.rs
src-tauri/crates/altai-cli/Cargo.toml
docs/vscode/TEST.md
```

### Scope

- Add `altai-cli serve --stdio --protocol 1`.
- Until TVS-05, support only `initialize`, one `run/start`, `run/cancel`, and
  `shutdown`.
- Use the existing scripted provider for deterministic integration tests.
- Production invocation without a configured real provider must fail with a
  structured error, never silently use the scripted provider.
- stdout must remain valid after warnings are written to stderr.

### Acceptance

- Integration test spawns the real compiled CLI process.
- It observes ordered `run_started`, at least one assistant/tool event, and one
  `run_terminated`.
- Cancel produces one terminal cancellation and no later run events.
- Invalid protocol version returns a structured error and clean shutdown.
- Split headers, split bodies, multiple frames per read, and EOF are tested.

### Verify

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p altai-protocol -p altai-cli serve
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-protocol -p altai-cli -- -D warnings
cargo run --manifest-path src-tauri/Cargo.toml -p altai-cli -- serve --help
git diff --check
```

### Terra prompt

```text
Implement TVS-02 only. Build the smallest real stdio vertical slice on the
existing IsanAgent one-shot host. Use altai-protocol framing and spawn the
compiled CLI in integration tests. This is a disposable spike boundary, not a
second agent runtime: keep its host adapter thin and mark unsupported methods
with structured capability errors. Do not add extension UI.
```

## 8. TVS-03 — Shared service and event seam

### Goal

Remove direct Tauri event emission from core agent behavior without moving the
entire runtime in one unsafe patch.

### Files

Create:

```text
src-tauri/crates/altai-agent-service/Cargo.toml
src-tauri/crates/altai-agent-service/src/lib.rs
src-tauri/crates/altai-agent-service/src/event.rs
src-tauri/crates/altai-agent-service/src/sink.rs
src-tauri/crates/altai-agent-service/src/workspace_services.rs
src-tauri/crates/altai-agent-service/tests/event_contract.rs
src-tauri/src/altai/agent/tauri_sink.rs
```

Update:

```text
src-tauri/Cargo.toml
src-tauri/src/altai/agent/mod.rs
src-tauri/src/altai/agent/runtime.rs
src-tauri/src/altai/agent/tauri_channel.rs
src-tauri/src/lib.rs
```

### Extraction boundary

- Move shared event DTOs and event sequencing into the service crate.
- Introduce `AgentEventSink` with async-safe, bounded delivery semantics.
- Implement `TauriEventSink` in the desktop crate.
- Centralize opening/classifying the memory DB and event journal in
  `WorkspaceServices`.
- Keep Tauri commands and most routing in place for this task.
- Preserve the event envelope consumed by
  `src/modules/ai/lib/agentEventBridge.ts`.

Do not duplicate structs to avoid imports. Move and re-export temporarily where
needed.

### Acceptance

- Desktop event fixtures are byte/JSON equivalent before and after extraction.
- Sink failure cannot mark an unperformed action successful.
- Journal sequencing remains authoritative and restart classification occurs
  exactly once per workspace service.
- `altai-agent-service` has no `tauri` dependency.
- Desktop tests and CLI tests still pass.

### Verify

```bash
if cargo tree --manifest-path src-tauri/Cargo.toml -p altai-agent-service | rg -q '(^|[-_ ])tauri([ -]|$)'; then
  echo "altai-agent-service must not depend on Tauri" >&2
  exit 1
fi
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-core -p altai-cli
cargo test --manifest-path src-tauri/Cargo.toml -p altai --lib altai::agent::runtime
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai -- -D warnings
pnpm test -- agentEventBridge
git diff --check
```

### Terra prompt

```text
Implement TVS-03 only. Introduce a Tauri-independent altai-agent-service event
and workspace-service seam, then adapt Desktop through TauriEventSink. Preserve
the frontend protocol byte-for-byte and use move/re-export steps instead of
copying runtime types. Do not move route_send or add new user-facing behavior
yet. Prove the new crate has no Tauri dependency.
```

## 9. TVS-04 — Long-lived runtime extraction

### Goal

Move the reusable run/session lifecycle out of the Tauri crate so Desktop and
stdio host invoke the same methods.

### Primary source

```text
src-tauri/src/altai/agent/runtime.rs
src-tauri/src/altai/agent/commands.rs
```

### Target files

```text
src-tauri/crates/altai-agent-service/src/service.rs
src-tauri/crates/altai-agent-service/src/runtime.rs
src-tauri/crates/altai-agent-service/src/routing.rs
src-tauri/crates/altai-agent-service/src/sessions.rs
src-tauri/crates/altai-agent-service/src/replay.rs
src-tauri/crates/altai-agent-service/src/clarification.rs
src-tauri/crates/altai-agent-service/tests/lifecycle.rs
src-tauri/src/altai/agent/runtime.rs
src-tauri/src/altai/agent/commands.rs
```

### Required service API

```text
send
cancel
steer
manual_compaction
list_sessions
get_session_messages
truncate_session
latest_replay_cursor
replay_run_events
respond_to_clarification
shutdown
```

Use a host-neutral channel identity. Existing `tauri:<chat_id>:` storage keys
must remain readable; add migration/alias behavior rather than orphaning
Desktop history.

### Acceptance

- Tauri commands are thin argument authorization/serialization wrappers.
- Desktop and service contract tests exercise the same method bodies.
- Concurrent chats with different model/persona/permission fingerprints remain
  isolated.
- Cancel and steer reject stale `run_id` values.
- Restart/replay returns monotonic events without duplication.
- Existing Desktop session IDs remain visible.
- There is still one IsanAgent runtime implementation.

### Verify

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service
cargo test --manifest-path src-tauri/Cargo.toml -p altai --lib altai::agent::runtime
cargo test --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai -p altai-cli -- -D warnings
pnpm test -- agentEventBridge chatStore agentRunsStore
git diff --check
```

### Review split

If this exceeds roughly 1,500 changed lines excluding mechanical moves, Terra
must stop after a compilable sub-boundary and propose `TVS-04A` / `TVS-04B`.
The first sub-PR should move send/cancel/steer/replay; the second should move
session mutation and clarification response. Neither sub-PR may leave two
active implementations.

### Terra prompt

```text
Implement TVS-04 only. Move the long-lived routing and session lifecycle from
the Tauri runtime into altai-agent-service, keeping Tauri commands as thin
wrappers. Preserve durable IDs, event ordering, permission behavior, and
concurrent model isolation. Do not mechanically copy the runtime. If the safe
diff exceeds the plan's review threshold, finish a compilable TVS-04A boundary
and report the exact TVS-04B remainder instead of producing a giant PR.
```

## 10. TVS-05 — Production stdio host

### Goal

Replace the TVS-02 limited spike with the persistent shared service.

### Files

Update:

```text
src-tauri/crates/altai-cli/src/serve/*
src-tauri/crates/altai-cli/src/main.rs
src-tauri/crates/altai-cli/Cargo.toml
src-tauri/crates/altai-cli/tests/serve_stdio.rs
src-tauri/crates/altai-protocol/*
docs/cli/IMPLEMENTATION_STATUS.md
docs/cli/TEST.md
docs/vscode/TEST.md
```

### Required behavior

- One initialized service per canonical workspace process.
- All v1 requests advertised during `initialize` are implemented or omitted
  from capabilities.
- Multiple chats and sequential/concurrent runs work.
- Approval/clarification can pause and resume without process restart.
- stdout is protocol-only for the process lifetime.
- SIGINT/termination requests cancel active foreground work and flush terminal
  journal events.
- Backpressure is bounded; lifecycle and approval events cannot be silently
  dropped.

### Acceptance

- The compiled CLI integration test runs two isolated chats.
- Replay repairs a deliberately disconnected client.
- A duplicate request ID and stale run command return typed errors.
- stderr redaction test includes fake API keys and authorization headers.
- Existing `altai run`, `agent`, and journal commands do not regress.

### Verify

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p altai-protocol -p altai-agent-service -p altai-cli
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-protocol -p altai-agent-service -p altai-cli -- -D warnings
cargo build --manifest-path src-tauri/Cargo.toml -p altai-cli --release
src-tauri/target/release/altai-cli version --verbose
src-tauri/target/release/altai-cli serve --help
git diff --check
```

### Terra prompt

```text
Implement TVS-05 only. Replace the limited stdio spike with the shared
altai-agent-service and expose exactly the capabilities that work. Add
multi-chat, reconnect/replay, stale-ID, redaction, backpressure, and shutdown
tests against the compiled CLI. Preserve every existing CLI command contract.
```

## 11. TVS-06 — Extension shell and host lifecycle

### Goal

Add a VS Code extension that activates lazily and reliably supervises the Rust
host, without building the full chat UI yet.

### Files

Create:

```text
extensions/vscode/package.json
extensions/vscode/tsconfig.json
extensions/vscode/esbuild.mjs
extensions/vscode/.vscodeignore
extensions/vscode/src/extension.ts
extensions/vscode/src/commands/registerCommands.ts
extensions/vscode/src/host/HostManager.ts
extensions/vscode/src/host/HostResolver.ts
extensions/vscode/src/protocol/RpcClient.ts
extensions/vscode/src/workspace/WorkspaceRegistry.ts
extensions/vscode/src/views/ChatViewProvider.ts
extensions/vscode/src/test/*
extensions/vscode/media/*
```

Update:

```text
pnpm-workspace.yaml
pnpm-lock.yaml
package.json
.github/workflows/ci.yml
```

### Contribution contract

- `extensionKind: ["workspace"]`.
- limited untrusted-workspace support.
- virtual workspace execution unsupported.
- one ALTAI Activity Bar container with Chat placeholder.
- commands: Open Chat, New Chat, Run Doctor, Open Logs.
- no `onStartupFinished` activation that starts Rust.
- `HostResolver` prefers the bundled exact target binary. Executable override
  is user/global scope only.
- one lazy process per selected canonical workspace.

### Acceptance

- Activation does not spawn Rust.
- First ALTAI action performs handshake and starts exactly one process.
- Two workspace folders receive two isolated managers.
- Incompatible/missing/crashed host produces a user-facing recovery action.
- deactivation performs bounded graceful shutdown.
- Restricted Mode cannot start a host or read workspace-controlled executable
  settings.

### Verify

```bash
pnpm --filter @altai/vscode exec tsc --noEmit
pnpm --filter @altai/vscode test
pnpm --filter @altai/vscode build
pnpm --filter @altai/vscode test:integration
pnpm lint
git diff --check
```

### Terra prompt

```text
Implement TVS-06 only. Scaffold the workspace VS Code extension, typed RPC
client, workspace registry, lazy HostManager, placeholder Chat view, commands,
trust gates, tests, and CI. Do not build transcript/composer UI. Prove
activation starts no child process and that first use starts one exact
workspace host.
```

## 12. TVS-07 — Streaming Chat MVP

### Goal

Deliver a useful plan/read agent conversation with durable sessions and
reload-safe streaming.

### Files

Create/update under:

```text
extensions/vscode/src/state/*
extensions/vscode/src/views/ChatViewProvider.ts
extensions/vscode/src/webview/*
extensions/vscode/media/*
packages/agent-ui/*
```

Only create `packages/agent-ui` after proving that extracted components have
no Tauri imports. A small extension-local UI is preferable to a premature
desktop-wide component migration.

### Scope

- session create/list/resume;
- prompt composer;
- streaming assistant/reasoning/tool/usage/outcome;
- model, agent, and `plan`/`ask` selectors;
- stop, retry, steer, and manual compact;
- visible context/token summary;
- monotonic reducer with sequence-gap replay;
- strict CSP, nonce, message validation, theme tokens, keyboard navigation,
  ARIA live announcements, reduced motion.

Do not add file mutation UI in this task. Use plan mode as the primary E2E.

### Acceptance

- Reloading the webview reconstructs the transcript from durable session data.
- Reloading the extension host replays the active run without duplicate events.
- Unknown optional event fields do not crash the view.
- Markdown cannot inject scripts, commands, or unrestricted remote content.
- The webview cannot access Node, filesystem, process, or raw VS Code APIs.
- Dark, light, high-contrast, keyboard, and screen-reader smoke tests pass.

### Verify

```bash
pnpm --filter @altai/agent-protocol test
pnpm --filter @altai/vscode exec tsc --noEmit
pnpm --filter @altai/vscode test
pnpm --filter @altai/vscode build
pnpm --filter @altai/vscode test:integration -- --grep "chat|replay|accessibility"
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-cli
git diff --check
```

### Terra prompt

```text
Implement TVS-07 only. Build the accessible streaming Chat MVP on the existing
RPC client and protocol. Start with plan/read flows, durable sessions, and
sequence-gap replay. Use a minimal local webview UI unless a Tauri-free shared
component boundary is already obvious. Enforce CSP and typed messages. Do not
implement edit approval yet.
```

## 13. TVS-08 — Editor-native context and commands

### Goal

Make ALTAI feel integrated with VS Code rather than a generic sidebar.

### Files

Create/update:

```text
extensions/vscode/src/context/ContextCollector.ts
extensions/vscode/src/context/limits.ts
extensions/vscode/src/commands/registerCommands.ts
extensions/vscode/src/git/GitContext.ts
extensions/vscode/src/webview/*
extensions/vscode/package.json
extensions/vscode/src/test/*
```

### Commands

```text
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
```

### Context

- current selection plus URI and line range;
- current or explicitly selected files;
- diagnostics;
- visible/open editors;
- staged/unstaged Git diff;
- file-picker images/PDFs;
- bounded folder summary delegated to Rust.

Every item is a visible, removable composer chip. Enforce item and total byte
limits before RPC. Do not claim general integrated-terminal scrollback.

### Acceptance

- Editor/Explorer menu visibility is context-sensitive.
- Multi-root commands require or retain an explicit target workspace.
- Untitled, remote, binary, deleted, symlinked, and oversized files fail
  clearly.
- The active editor is never silently attached unless the command semantics
  say so.
- Prompt injection-like filenames/content cannot alter protocol framing.

### Verify

```bash
pnpm --filter @altai/vscode test -- ContextCollector commands GitContext
pnpm --filter @altai/vscode exec tsc --noEmit
pnpm --filter @altai/vscode test:integration -- --grep "selection|context|multi-root"
pnpm --filter @altai/vscode build
git diff --check
```

### Terra prompt

```text
Implement TVS-08 only. Add editor/explorer commands and an explicit bounded
ContextCollector for selection, files, diagnostics, visible editors, Git diff,
images, and PDFs. Keep context visible as removable chips. Handle multi-root,
remote URIs, binary/oversized content, and Restricted Mode. Do not add terminal
scrollback claims or edit approval.
```

## 14. TVS-09 — Native diff, approvals, and checkpoints

### Goal

Safely complete real editing tasks through the same permission policy as
Desktop.

### Files

Create/update:

```text
extensions/vscode/src/diff/DiffContentProvider.ts
extensions/vscode/src/diff/DiffController.ts
extensions/vscode/src/approvals/ApprovalController.ts
extensions/vscode/src/checkpoints/CheckpointController.ts
extensions/vscode/src/state/*
extensions/vscode/src/webview/*
extensions/vscode/package.json
src-tauri/crates/altai-agent-service/src/clarification.rs
src-tauri/crates/altai-agent-service/src/checkpoint.rs
src-tauri/crates/altai-cli/src/serve/*
```

### Flow

Runtime event -> registered original/proposed read-only URIs -> `vscode.diff`
-> explicit response -> `clarification/respond` -> next sequenced runtime
event.

Closing a diff tab is never approval. Use document versions/content hashes to
detect stale review. `bypass` requires a separate user-level unlock and warning.

### Acceptance

- Deny produces no filesystem mutation.
- Approve applies only the still-current reviewed operation.
- A file changed after review invalidates the approval.
- Checkpoints restore modify/create/delete cases.
- Secret paths, traversal, symlink escapes, and roots outside the selected
  workspace remain blocked.
- Restricted Mode and plan mode cannot mutate.
- Duplicate approval replies are idempotent or rejected with a typed error.

### Verify

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-cli approval
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-cli checkpoint
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-cli security
pnpm --filter @altai/vscode test -- DiffController ApprovalController CheckpointController
pnpm --filter @altai/vscode test:integration -- --grep "diff|approval|checkpoint|trust"
pnpm --filter @altai/vscode exec tsc --noEmit
git diff --check
```

### Terra prompt

```text
Implement TVS-09 only. Add native vscode.diff review, explicit
clarification/respond approvals, stale-content protection, checkpoint restore,
and trust/path/security tests. Closing a diff must never approve. Reuse the
Rust permission and secret-path policy; do not add a TypeScript mutation path.
```

## 15. TVS-10 — Recovery, Work, and Inbox

### Goal

Expose active and historical agent work without duplicating run identity.

### Files

Create/update:

```text
extensions/vscode/src/work/*
extensions/vscode/src/inbox/*
extensions/vscode/src/state/*
extensions/vscode/src/views/*
extensions/vscode/package.json
src-tauri/crates/altai-agent-service/src/work.rs
src-tauri/crates/altai-agent-service/src/notifications.rs
src-tauri/crates/altai-agent-service/src/automations.rs
src-tauri/crates/altai-cli/src/serve/*
src/modules/ai/lib/workView.ts
src/modules/ai/lib/workView.test.ts
```

Use the semantics in
`docs/plans/2026-07-29-unified-agent-work-surface.md`.

### Scope

- Work Tree View: active, attention, review, history.
- Inbox Tree View: action, review, update.
- deep-link to owning session/run/diff.
- background jobs, subagents, notifications, and clarification projections.
- active-run recovery after extension-host reload.
- automation browse and run-now only.

Do not claim that work survives full VS Code exit until a daemon exists.

### Acceptance

- One canonical `run_id` across Chat, Work, and Inbox.
- Notification/ticket duplicates collapse into one projection.
- Resolving Inbox does not delete Work history.
- Sequence replay after extension reload has no duplicates.
- A foreground child-process limitation is clearly labeled.
- Desktop and VS Code show compatible state for the same workspace.

### Verify

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-cli
pnpm test -- workView
pnpm --filter @altai/vscode test -- work inbox recovery
pnpm --filter @altai/vscode test:integration -- --grep "work|inbox|reload"
pnpm --filter @altai/vscode exec tsc --noEmit
git diff --check
```

### Terra prompt

```text
Implement TVS-10 only. Add native Work and Inbox projections using the unified
work-surface semantics. Preserve one canonical run identity, deduplicate
notifications/tickets, deep-link into Chat/diff, and recover through journal
replay. Do not advertise editor-exit persistence or implement a daemon.
```

## 16. TVS-11 — Packaging, Remote, and security gates

### Goal

Produce installable target-specific VSIX packages and prove where the Rust host
runs.

### Files

Create/update:

```text
.github/workflows/vscode-ci.yml
.github/workflows/vscode-release.yml
extensions/vscode/scripts/package-host.mjs
extensions/vscode/scripts/verify-package.mjs
extensions/vscode/.vscodeignore
extensions/vscode/package.json
extensions/vscode/README.md
extensions/vscode/SECURITY.md
docs/vscode/TEST.md
docs/vscode/PACKAGING.md
docs/vscode/REMOTE.md
```

### Initial target matrix

```text
darwin-arm64
darwin-x64
linux-x64
linux-arm64
win32-x64
win32-arm64
```

Alpine targets are added only after musl behavior is tested. Pure `web` is not
published in v1.

### Required gates

- build the Rust host for each target;
- place exactly one matching host binary in each VSIX;
- generate checksums, SBOM, license notices, and provenance;
- verify executable permissions and Windows suffix handling;
- test local macOS/Windows/Linux;
- smoke WSL, Remote SSH, and Dev Container extension placement;
- verify no source maps, fixtures with secrets, other platform binaries, or
  developer files leak into VSIX;
- document signing and publisher ownership.

### Acceptance

- `vsce ls` and archive inspection match an allowlist.
- Marketplace target names match the extension-host platform.
- Remote workspace starts the remote binary, not a local UI-side binary.
- incompatible architecture produces a recovery message, not an exec-format
  crash loop.
- Restricted Mode, CSP, redaction, secret scan, and dependency audit pass.

### Verify

```bash
pnpm --filter @altai/vscode package:all
pnpm --filter @altai/vscode verify:packages
pnpm --filter @altai/vscode test:integration
cargo test --manifest-path src-tauri/Cargo.toml -p altai-agent-service -p altai-cli
git diff --check
```

CI evidence is required for targets that cannot be executed locally.

### Terra prompt

```text
Implement TVS-11 only. Add platform-specific VSIX packaging, host-binary
allowlist verification, checksums/SBOM/provenance, local and Remote CI, and
security gates for the declared target matrix. Package exactly one target
binary per VSIX. Do not publish web or Alpine targets without real tests.
```

## 17. TVS-12 — Public beta

### Goal

Finish product documentation, quality evidence, and release automation without
adding new architecture.

### Deliver

- clean-machine install to first plan run;
- update and rollback verification;
- accessibility audit: keyboard, screen reader, high contrast, reduced motion;
- performance measurements: activation, host readiness, first event, memory;
- provider/auth onboarding that keeps keys out of webview/settings;
- troubleshooting for missing/incompatible/crashed host;
- privacy and telemetry statement;
- Marketplace listing assets and changelog;
- pre-release channel and rollback procedure;
- optional Open VSX only after Marketplace package verification.

### Beta blockers

- any unapproved file mutation;
- dropped/duplicated replay event;
- plaintext key in logs/state/package;
- unhandled host mismatch;
- inaccessible approval choice;
- missing license/provenance;
- unsupported persistence claim.

### Verify

Run the complete CI matrix plus:

```bash
pnpm --filter @altai/vscode verify:packages
pnpm --filter @altai/vscode test:integration
cargo test --manifest-path src-tauri/Cargo.toml --workspace
pnpm exec tsc --noEmit
pnpm lint
pnpm test
pnpm build
git diff --check
```

### Terra prompt

```text
Implement TVS-12 only. Prepare the verified public beta: documentation,
onboarding, accessibility/performance evidence, package inspection, release
automation, pre-release channel, and rollback. Do not add Chat Participant,
inline completion, or other new product scope. Treat every listed beta blocker
as a release stop.
```

## 18. Deferred Terra tasks

Create separate plans after beta:

- `@altai` Chat Participant using the same service/session IDs;
- Language Model Tools;
- inline completion with an explicit latency and cancellation budget;
- diagnostic code actions;
- notebook/experiment integration;
- MCP and skills management;
- daemon-backed scheduled/background work that survives editor exit;
- Cursor/Open VSX compatibility.

None may introduce a second provider or permission implementation.

## 19. Terra handoff template

Terra ends every task with:

```text
Task: TVS-XX
Outcome: completed | partial | blocked

Changed:
- file: reason

Acceptance:
- [x] proven criterion
- [ ] unproven criterion and why

Verification:
- command -> pass/fail

Compatibility:
- protocol/schema impact
- Desktop impact
- CLI impact
- VS Code impact

Risks / next task:
- concise remaining risk

Suggested next task:
- TVS-YY, only if all hard dependencies are accepted
```

If blocked, Terra must leave the repository compiling and state the smallest
missing decision. It must not continue into the next task to work around an
unaccepted architecture change.
