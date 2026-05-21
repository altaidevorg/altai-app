# Remote Execution + Mobile Companion — Roadmap

> Status: planning. No code yet. Each phase is a separate PR / session.

## What this roadmap covers

Three remote execution targets for the desktop app, plus a phone companion for
monitoring and approving tool calls on the go:

1. **SSH** — work on a remote Linux/Mac box like Cursor/VS Code Remote-SSH.
2. **Docker** — work inside a local or remote container via `docker exec`.
3. **Cloud worker** — agent runs server-side; desktop or mobile attaches as a viewer.
4. **Mobile app** — read-mostly monitor for cloud sessions; approve/deny tool calls,
   receive push notifications when an agent is blocked.

## North-star architecture

```
┌──────────────────┐        ┌──────────────────────┐        ┌──────────────────┐
│  Desktop altai   │        │  altai cloud worker  │        │  Mobile altai    │
│  (Tauri 2)       │        │  (Node/TS or Rust)   │        │  (Tauri mobile)  │
│                  │  WS    │                      │  WS    │                  │
│  Executor:       │ ←────→ │  - session state     │ ←────→ │  - login         │
│   Local          │        │  - agent loop        │        │  - sessions list │
│   SSH            │        │  - tool approval     │        │  - timeline view │
│   Docker         │        │    queue             │        │  - approve/deny  │
│   Cloud (proxy)  │        │  - push fan-out      │        │  - push receiver │
└─────────┬────────┘        └──────────┬───────────┘        └──────────────────┘
          │                            │
          │  local/ssh/docker          │  spawns workers
          ↓                            ↓
   ┌────────────────┐           ┌───────────────────┐
   │ User machine   │           │  Worker fleet     │
   │ Remote SSH host│           │  (k8s / fly.io)   │
   │ Container      │           └───────────────────┘
   └────────────────┘
```

**Core idea**: introduce an `Executor` abstraction inside the desktop app so the
agent runtime doesn't care *where* `read_file` or `bash_run` actually executes.
The same abstraction lets the cloud worker reuse the agent loop wholesale.

## Why this order

A1 (executor abstraction) is the cheapest, lowest-risk refactor — it doesn't ship
any new product but everything else depends on it. Skipping it and writing the
SSH backend first locks SSH-specific assumptions into the agent loop.

B1 (cloud worker) and C1 (mobile shell) can run **in parallel** once A1 is done,
since they don't share code. That's the only place parallel work pays off.

---

## Phase A — Local refactor: Executor abstraction

### A1 — Extract `Executor` interface (1 session)

**Goal**: every call currently going through `native.*` in
[src/modules/ai/lib/native.ts](../src/modules/ai/lib/native.ts) routes through an
`Executor` interface so swapping local for remote is a single dependency
injection.

**Acceptance criteria**

- New `src/modules/ai/lib/executors/types.ts` defines `Executor` with all
  current methods (`readFile`, `writeFile`, `readDir`, `runCommand`,
  `shellSessionOpen`, etc).
- `LocalExecutor` implements the interface and just calls the existing `invoke`s.
- `native.ts` re-exports a `currentExecutor()` getter; all callers route through it.
- No behavior change. All existing tests pass. App launches, agent runs locally.

**Files touched**

- New: `src/modules/ai/lib/executors/types.ts`, `executors/local.ts`,
  `executors/index.ts`.
- Modified: `src/modules/ai/lib/native.ts` (becomes a thin re-export),
  `src/modules/ai/tools/*` (no behavior change — verify each caller).

**Risk / not-in-scope**

- No new functionality. If the diff balloons because the interface doesn't fit a
  command, the interface is wrong — fix the interface, don't widen the diff.
- Git operations stay attached to local for now (A2 handles git over SSH).

---

## Phase A2 — SSH executor (2-3 sessions)

**Goal**: `WorkspaceEnv = { kind: "ssh", host, user, keyPath? }` makes every
filesystem/shell/grep call run on the remote host.

**Acceptance criteria**

- New Rust commands `ssh_open_session`, `ssh_close_session`, plus `fs_*` /
  `shell_*` variants that route over the established SSH channel.
- Connection manager: one connection per host, multiplexed channels (sftp for
  files, exec for commands).
- UI: new "Connect to SSH host" entry, host config in settings, status indicator
  in the status bar.
- `WorkspaceEnv` switch reflects in `currentWorkspaceEnv()`; tool calls go
  through SSH transparently.
- Agent can edit a file on the remote, run `cargo build`, see output streamed back.

**Files touched**

- Rust: `src-tauri/src/modules/ssh/{client.rs,session.rs,fs.rs,shell.rs}.
  Crates: `russh`, `russh-sftp`, or `async-ssh2-tokio`.
- TS: `executors/ssh.ts`, settings UI for host management, env switcher in toolbar.

**Risks**

- `bash_background` semantics: need a long-lived shell channel, not one-shot
  `exec`. Decide: tmux-based session vs. raw channel.
- Auth: only key-based at first, no passwords; password prompt is a separate
  follow-up.
- Grep performance: shipping the remote file tree to the local grep is wrong;
  use ripgrep over SSH instead.
- Path canonicalization: the existing `fs_canonicalize` lives in Rust on the
  local box. Need a remote equivalent or skip canonicalization for SSH paths
  (treat as already-canonical).

---

## Phase A3 — Docker executor (1-2 sessions)

**Goal**: `WorkspaceEnv = { kind: "docker", container, workdir }` lets the agent
work inside a container.

**Acceptance criteria**

- Connect to a running container by name/id; refuse if not running.
- `fs_*` uses `docker cp` (or bind-mounted volume read when available) and
  `docker exec` for commands.
- Same UI affordances as SSH (status bar, settings).
- Compose project detection: if the workspace has a `docker-compose.yml`, surface
  the service list as connect targets.

**Files touched**

- Rust: `src-tauri/src/modules/docker/*` using `bollard` crate.
- TS: `executors/docker.ts`, container picker.

**Risks**

- File watching inside containers is unreliable on macOS — fall back to polling
  for the explorer when in Docker mode.
- Permissions inside the container vs. host UID mismatch.

---

## Phase B — Cloud worker server (separate repo)

### B1 — Worker server skeleton (3-5 sessions)

**Goal**: a standalone service that runs the agent loop and exposes a WebSocket
API for desktop and mobile clients to subscribe.

**Acceptance criteria**

- New repo `altai-cloud-worker` (Node/TS, Bun runtime, or Rust + axum).
  Lean toward Bun + Hono — fast iteration, smaller surface than a full Node stack.
- Endpoints:
  - `POST /sessions` — create session (returns session id + WS URL).
  - `WS /sessions/:id` — bidirectional event stream (agent → events, client → user
    messages and approval responses).
  - `POST /sessions/:id/approve` — approve a pending tool call (also accepted over WS).
- Persistence: SQLite for session metadata, file storage for transcripts and
  artifacts. Postgres later when we need multi-tenant.
- Auth: JWT, single-user mode at first (one token per install). Multi-user is
  a follow-up.
- The agent loop is the *same* code as desktop — wrap `runAgentStream` from
  [src/modules/ai/lib/agent.ts](../src/modules/ai/lib/agent.ts) behind the
  Executor interface so the worker uses the same logic.
- Tool execution on the worker uses `LocalExecutor` against a sandbox dir.

**Files touched**

- New repo, structure TBD but probably:
  ```
  altai-cloud-worker/
    src/
      sessions/        # session lifecycle
      transport/       # WS server, message schema
      executor/        # LocalExecutor reused from desktop (vendored or shared pkg)
      auth/
    test/
  ```

**Risks**

- **Sharing code with desktop**: pulling agent logic out of the Tauri app into a
  shared `@altai/agent-core` package is the right move; this is the moment to
  do it. If we skip it, B1 forks the agent loop and they drift.
- **Tool approval over WS**: the desktop already pauses on approval; reusing
  that on the worker requires the approval promise to resolve from a remote
  event instead of a local UI click. Plan the event schema carefully.
- **Cost**: worker is paying for the model — pricing/billing is out of scope here
  but is on the critical path before public launch.

### B2 — Streaming, attachments, multi-window viewing (2 sessions)

After B1 works for one client, add:
- Multiple clients attached to the same session (desktop + mobile both watching).
- Attachment upload (images, PDFs) over the WS protocol.
- Resume after disconnect: client reconnects, gets the missed events replay.

---

## Phase A4 — Cloud executor (desktop → worker) (2 sessions)

**Goal**: a desktop user picks "Run on cloud" for a session, the agent loop runs
on the worker, the desktop UI is just a viewer.

**Acceptance criteria**

- `WorkspaceEnv = { kind: "cloud", workerURL, sessionId, token }`.
- New `CloudExecutor` proxies tool calls to the worker over WS; on the worker
  side, the actual `LocalExecutor` (running in a sandbox or attached SSH host)
  does the work.
- Tool approval still surfaces in the desktop UI — the approval click sends a WS
  message back to the worker.
- Session resume: closing the desktop, reopening it, picking the session id
  brings the user back to the live stream.

**Risks**

- Latency: typing a chat message and waiting two RTTs for the first token.
  Mitigate by streaming tokens as soon as the worker has them.
- Trust boundary: tool calls executing on someone else's machine. Be explicit in
  the UI about which executor is active — color the status bar.

---

## Phase C — Mobile companion

### C1 — Mobile shell (4-6 sessions)

**Goal**: an iOS + Android app you can install, log in, see your running cloud
sessions, view the timeline, approve/deny tool calls.

**Acceptance criteria**

- App launches, login screen, OTP or QR-pair from desktop.
- Sessions list (active + recent).
- Per-session timeline view: messages, tool calls, file diffs (read-only).
- Tool approval card with **Approve** / **Deny** buttons.
- Push notification arrives when a session is blocked on approval.
- Settings: log out, switch worker URL.

**Stack choice** — pick one:

| Option | Pros | Cons |
|---|---|---|
| **Tauri 2 mobile** | Reuse some React UI from desktop; one codebase | Tauri mobile is young; native plugin gaps; tooling rough |
| **Expo / React Native** | Mature, fast iteration, plenty of plugins | Separate codebase from desktop; UI parity is on you |
| **SwiftUI + Kotlin Compose** | Best native feel, push notifications easy | Two codebases, no React reuse, slowest velocity |

Recommendation: **Expo / React Native** for the first release.
Rationale: monitor-first means a small surface; you don't need to reuse desktop
UI; mature push-notification + Expo Application Services pipeline saves weeks
over Tauri mobile right now. Revisit Tauri mobile in 6-12 months.

**Files touched**

- New repo `altai-mobile`. Lean on Expo Router for navigation, NativeWind for
  Tailwind parity, expo-secure-store for the worker token, expo-notifications
  for push.

**Risks**

- App Store review for any agent UI that runs commands — but mobile is monitor
  only, so this is small.
- Network on cellular: WS reconnection logic matters.

### C2 — Push notifications (1-2 sessions)

- APNs for iOS, FCM for Android.
- Worker fans out a push when a session enters `awaiting_approval`.
- Tap notification → app deep-links to the approval card.

---

## Cross-cutting concerns

### Shared packages

Once B1 starts, extract from the desktop into shared workspace packages:
- `@altai/agent-core` — `runAgentStream`, tool definitions, prompts.
- `@altai/protocol` — WS message schema (Zod schemas), session types.
- `@altai/executor` — `Executor` interface + `LocalExecutor`. SSH/Docker
  variants live in their consumer (desktop only, for now).

Right now the repo is a single Tauri app; converting to a `pnpm-workspace.yaml`
monorepo with `packages/*` is a one-time refactor that happens at B1.

### Security & secrets

- API keys live on the worker, not on the mobile. Mobile only stores its
  worker JWT.
- SSH keys: store in the OS keychain via `tauri-plugin-store-keychain` (don't
  read from `~/.ssh` directly into JS).
- All WS traffic must be TLS; reject `ws://` outside `localhost`.
- Tool approval payload signed by the worker so mobile can verify what it's
  approving.

### Telemetry / observability

Once a worker exists, you'll want:
- Structured logs (pino) shipped to whatever log sink (Axiom / Logflare / your
  own).
- Per-session metrics: total tokens, wall time, tool counts.
- An admin view (later) that shows fleet health.

### Pricing & rate limiting

Out of scope for engineering roadmap. But the moment B1 ships internally,
decide:
- Bring-your-own-key vs. metered access.
- Per-session token cap.
- Concurrent session cap per account.

---

## Estimated total effort

| Phase | Sessions |
|---|---:|
| A1 Executor interface | 1 |
| A2 SSH | 2-3 |
| A3 Docker | 1-2 |
| B1 Worker server | 3-5 |
| A4 Cloud executor | 2 |
| B2 Multi-client streaming | 2 |
| C1 Mobile shell | 4-6 |
| C2 Push | 1-2 |
| Shared-package refactor (at B1) | 1 |
| **Total** | **17-24 sessions** |

This is a 2-3 month roadmap at one session/day, or ~6 weeks of focused full-time
work. The phases are intentionally checkpoint-shippable: shipping A1+A2 alone
(local + SSH) is already a real product step.

## What to do *next*

Decide whether to start with:
1. **A1** — cheapest, unblocks everything else. Recommended.
2. **C1 prototype** in parallel — only viable if someone else can sit on the
   desktop work while the mobile codebase incubates against mock data.

Avoid starting B1 before A1: it'll fork the agent loop and you'll pay for the
merge later.
