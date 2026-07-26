# ALTAI Agent Operations Implementation Plan

> Status: proposed
>
> Created: 2026-07-26
>
> Goal: evolve ALTAI from a local project board with background runs into a
> local-first, durable, inspectable agent operations platform with Codex-class
> execution quality.

The Kilo Code + Z.AI GLM low-token execution profile for this roadmap is
defined in `docs/KILO_GLM_IMPLEMENTATION_EXECUTION_PLAN.md`.

## 1. Product outcome

ALTAI should let a user manage objectives instead of supervising individual
agent prompts.

A user must be able to:

1. Create work locally without connecting GitHub or another tracker.
2. Optionally attach GitHub, Linear, Jira, or another provider as a task source.
3. Let a durable orchestrator claim eligible work and run it in isolated
   workspaces.
4. Inspect plans, agent activity, tool calls, approvals, diffs, tests, costs,
   and delivery artifacts while work is running.
5. Steer, pause, resume, retry, cancel, or reassign any run.
6. Require deterministic quality and security gates before work can be handed
   off or applied.
7. Use the built-in ALTAI runtime, Codex App Server, or a future runner without
   changing the project-management model.
8. Recover correctly after renderer reloads, application restarts, runner
   crashes, transient provider failures, and interrupted Git operations.
9. Convert successful work into reusable project knowledge and playbooks only
   after user review.

GitHub is an optional integration. The local task store, scheduler, worktrees,
agent runs, review, evidence, and apply flow must remain available without it.

## 2. Delivery principles

### 2.1 No competing runtime

ALTAI already has a substantial Rust agent runtime, a sequenced SQLite event
journal, persisted assignments, permission gates, and Git worktree operations.
This plan extends those contracts instead of creating a second agent loop.

- The existing ALTAI/IsanAgent runtime becomes the first `RunnerAdapter`.
- Codex App Server is added as a separate adapter.
- Scheduler state is stored through the existing SQLite migration approach.
- Existing run events remain the source of truth for native agent execution.
- Existing worktree create, remove, and safe-apply operations remain the Git
  isolation boundary.

If a required runtime capability belongs in IsanAgent, follow the delivery rule
in `docs/ALTAI_CLAW_IMPLEMENTATION_PLAN.md`: implement it upstream first, pin
the merged revision, then add the ALTAI adapter and UI.

### 2.2 One authoritative coordinator

The Rust orchestration service is the only component allowed to:

- determine task eligibility;
- claim tasks;
- allocate concurrency;
- create attempts;
- schedule retries;
- own leases;
- decide terminal task state;
- reconcile external task sources;
- initiate cleanup.

React renders state and sends commands. It must not be required for scheduling
correctness and must not run the scheduler from a browser interval.

### 2.3 Configuration is layered

Effective configuration uses this precedence:

```text
managed safety requirements
        ↓ constrain
application defaults
        ↓ overridden by
repository WORKFLOW.md
        ↓ overridden by
task template
        ↓ overridden by
explicit per-run user choices
```

Lower layers may narrow permissions. They cannot override managed safety
requirements, workspace boundaries, credential isolation, or destructive-action
policies.

### 2.4 Evidence, not self-reported completion

An agent saying that work is complete is not a delivery gate. Completion is
derived from:

- the runner terminal event;
- a clean and inspectable workspace state;
- configured validation commands;
- required review results;
- required proof-of-work artifacts;
- the configured human or automated handoff policy.

### 2.5 Incremental rollout

Do not replace the current orchestration flow in one release.

1. Add the new backend behind `orchestration_v2`.
2. Run its eligibility logic in read-only shadow mode.
3. Enable it for newly started local queues only.
4. Import legacy intent and assignments on first opt-in.
5. Keep rollback to the current queue possible during the preview.
6. Remove the legacy claim loop only after recovery and soak gates pass.

## 3. Existing foundations to reuse

| Existing capability          | Location                                          | Planned use                               |
| ---------------------------- | ------------------------------------------------- | ----------------------------------------- |
| Sequenced SQLite run journal | `src-tauri/src/altai/agent/event_journal.rs`      | Runner event durability and replay        |
| Native agent runtime         | `src-tauri/src/altai/agent/runtime.rs`            | `NativeRunnerAdapter`                     |
| Current scheduler prototype  | `src-tauri/src/modules/orchestration.rs`          | State-machine behavior to migrate         |
| Repository workflow loader   | `src-tauri/src/modules/orchestration/workflow.rs` | Versioned `WORKFLOW.md` parser            |
| Agent assignment model       | `src/modules/github/store/assignmentsStore.ts`    | Compatibility projection during migration |
| Worktree isolation and apply | `src-tauri/src/modules/git/operations.rs`         | Workspace provider and delivery           |
| Project Board                | `src/modules/github/components/OverviewBoard.tsx` | Task control plane                        |
| Runtime run store            | `src/modules/ai/store/agentRunsStore.ts`          | Compatibility bridge to activity UI       |
| Remote execution roadmap     | `docs/REMOTE_ROADMAP.md`                          | Local/Docker/SSH/cloud executor work      |
| Claw implementation plan     | `docs/ALTAI_CLAW_IMPLEMENTATION_PLAN.md`          | Runtime ownership and upstream rules      |

## 4. Target architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│ Task Sources                                                    │
│ Local · GitHub · Linear · Jira · API                            │
└──────────────────────────────┬──────────────────────────────────┘
                               ↓ normalized tasks
┌─────────────────────────────────────────────────────────────────┐
│ Durable Orchestrator                                            │
│ Eligibility · DAG · leases · retries · budgets · reconciliation │
└───────────────┬──────────────────────┬──────────────────────────┘
                │                      │
                ↓                      ↓
┌───────────────────────────┐  ┌──────────────────────────────────┐
│ Policy and Hook Engine    │  │ Event Ledger and Projections     │
│ permissions · network     │  │ activity · metrics · audit       │
│ commands · approvals      │  │ evidence · task/read models      │
└───────────────┬───────────┘  └──────────────────────────────────┘
                ↓
┌─────────────────────────────────────────────────────────────────┐
│ Runner Adapters                                                 │
│ Native ALTAI · Codex App Server · future external CLI runners   │
└──────────────────────────────┬──────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────────┐
│ Workspace Executors                                             │
│ Local worktree · Docker · SSH · future cloud worker             │
└──────────────────────────────┬──────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────────┐
│ Verification and Delivery                                       │
│ tests · review · browser QA · evidence · apply · draft PR       │
└─────────────────────────────────────────────────────────────────┘
```

### 4.1 Core Rust boundaries

Create these backend modules under `src-tauri/src/modules/orchestration/`:

```text
orchestration/
├── commands.rs
├── config.rs
├── coordinator.rs
├── domain.rs
├── ledger.rs
├── migrations.rs
├── projections.rs
├── recovery.rs
├── scheduler.rs
├── policy/
│   ├── mod.rs
│   ├── approvals.rs
│   ├── hooks.rs
│   ├── network.rs
│   └── rules.rs
├── runners/
│   ├── mod.rs
│   ├── native.rs
│   └── codex_app_server.rs
├── sources/
│   ├── mod.rs
│   ├── local.rs
│   ├── github.rs
│   ├── linear.rs
│   └── jira.rs
├── workspaces/
│   ├── mod.rs
│   ├── local_worktree.rs
│   ├── docker.rs
│   └── ssh.rs
├── verification/
│   ├── mod.rs
│   ├── commands.rs
│   ├── review.rs
│   └── browser.rs
└── workflow/
    ├── mod.rs
    ├── schema_v1.rs
    ├── schema_v2.rs
    └── template.rs
```

The exact split may change during implementation, but ownership must remain
clear: sources normalize work, the coordinator owns state, runners execute,
workspace providers isolate, policies authorize, and verification proves the
result.

### 4.2 Adapter contracts

#### Task source

```text
TaskSourceAdapter
├── list_candidates(cursor)
├── get_task(native_id)
├── reconcile(task)
├── post_status(task, status)
├── post_comment(task, body)
└── capabilities()
```

Provider writes are capability-checked and optional. `LocalTaskSource` is always
available and requires no authentication.

#### Runner

```text
RunnerAdapter
├── capabilities()
├── start_attempt(spec)
├── resume_attempt(identity, input)
├── steer_attempt(identity, message)
├── cancel_attempt(identity)
├── inspect_attempt(identity)
└── shutdown()
```

Runner events are normalized into a versioned ALTAI event envelope. Provider-
specific payloads may be retained as bounded diagnostic metadata, but UI and
scheduler logic consume normalized event kinds.

#### Workspace executor

```text
WorkspaceExecutor
├── prepare(task, attempt)
├── run(command, policy)
├── inspect_files()
├── collect_diff()
├── collect_artifact()
├── cleanup(policy)
└── health()
```

This contract must converge with the `Executor` abstraction planned in
`docs/REMOTE_ROADMAP.md`; do not create two remote execution interfaces.

## 5. Durable domain model

Extend the current SQLite migration framework. Keep the append-only agent run
journal and add orchestration-specific tables/projections.

### 5.1 Required entities

#### Project

- stable workspace identity;
- repository root identity;
- selected workflow revision;
- selected task source adapters;
- running/paused/stopped intent;
- created and updated timestamps.

#### Task

- stable ALTAI task ID;
- source kind and source-native reference;
- title and description;
- acceptance criteria;
- priority and labels;
- state;
- dispatch eligibility;
- dependency summary;
- assigned agent profile;
- created and updated timestamps;
- optimistic source revision.

#### Dependency

- upstream task ID;
- downstream task ID;
- dependency kind: `blocks`, `requires`, or `related`;
- source-native relationship when available.

#### Attempt

- stable attempt ID and task ID;
- attempt number;
- runner kind and runner identity;
- workspace/executor identity;
- agent profile and immutable effective configuration;
- lease owner, generation, and expiry;
- thread/turn/session identifiers;
- state and terminal outcome;
- retry kind and next retry time;
- token, duration, and cost totals;
- created, started, heartbeat, and terminal timestamps.

#### Approval

- attempt ID and originating event;
- risk class;
- requested action summary;
- bounded structured action payload;
- policy decision and decision source;
- expiry;
- user-visible status.

#### Artifact

- attempt/task ID;
- type: `diff`, `test`, `lint`, `build`, `ci`, `review`, `screenshot`,
  `video`, `log`, `report`, or `other`;
- content-addressed storage key or external URL;
- MIME type, size, checksum, and redaction status;
- producer and creation timestamp.

#### Worker

- executor kind and host identity;
- capabilities;
- health and last heartbeat;
- capacity and active allocation count;
- environment revision.

### 5.2 Event model

All orchestration mutations append a versioned event and update projections in
one transaction.

Minimum event families:

- `project.started`, `project.paused`, `project.stopped`;
- `task.discovered`, `task.updated`, `task.claimed`, `task.blocked`;
- `attempt.created`, `attempt.started`, `attempt.heartbeat`;
- `attempt.input_required`, `attempt.approval_required`;
- `attempt.steered`, `attempt.cancel_requested`;
- `attempt.completed`, `attempt.failed`, `attempt.cancelled`,
  `attempt.stalled`;
- `verification.started`, `verification.finished`;
- `review.started`, `review.finished`;
- `artifact.created`;
- `delivery.ready`, `delivery.applied`, `delivery.published`,
  `delivery.failed`;
- `workspace.created`, `workspace.reused`, `workspace.cleaned`;
- `policy.allowed`, `policy.denied`, `policy.approval_required`;
- `budget.warning`, `budget.exhausted`;
- `source.reconcile_failed`, `source.write_failed`.

Every event contains:

- schema version;
- event ID;
- workspace/project/task/attempt IDs as applicable;
- monotonically ordered sequence within its aggregate;
- UTC timestamp;
- actor: user, orchestrator, runner, hook, or source adapter;
- bounded JSON payload;
- correlation and causation IDs.

### 5.3 State machine

Task states:

```text
Draft
  → Queued
  → Planning
  → AwaitingPlanApproval
  → Running
  → AwaitingInput
  → AwaitingApproval
  → Verifying
  → Reviewing
  → ReadyForHandoff
  → Done
```

Side states:

```text
Blocked · Retrying · Paused · Cancelled · Failed · Abandoned
```

Rules:

- Only the coordinator changes authoritative task or attempt state.
- An attempt completion does not directly produce `Done`.
- `Done` requires all configured completion gates.
- A task with unresolved blocking dependencies cannot be claimed.
- Retries create a new attempt identity while optionally reusing the same
  workspace and runner thread according to policy.
- A source task becoming ineligible stops or pauses its active attempt according
  to the configured source policy.
- Terminal events are idempotent and first-terminal-wins.

### 5.4 Leases and duplicate prevention

- Claim and lease creation occur in one `BEGIN IMMEDIATE` transaction.
- A lease has an owner ID, generation, and expiry.
- Heartbeats renew only the current generation.
- Recovery may reclaim an expired lease only after inspecting the runner and
  workspace.
- Every dispatch carries an idempotency key derived from task ID, attempt ID,
  runner, and generation.
- Source writes use provider-native idempotency where available and an ALTAI
  outbox otherwise.
- No renderer state participates in duplicate prevention.

## 6. WORKFLOW.md version 2

The current file remains valid as legacy v1. A missing `version` means v1.
Version 2 is explicit and strictly parsed.

Illustrative target:

```yaml
---
version: 2

orchestration:
  max_concurrent: 4
  max_attempts: 4
  poll_interval_seconds: 15
  stall_timeout_seconds: 300
  active_states: [todo, in_progress]
  terminal_states: [done, cancelled]

runner:
  default: native
  allow: [native, codex-app-server]

agents:
  planner:
    model_id: null
    reasoning: medium
    permissions: plan
    tools: [read, search, git-read]
  worker:
    model_id: null
    reasoning: high
    permissions: auto-edit
  reviewer:
    model_id: null
    reasoning: high
    permissions: plan

environment:
  executor: local-worktree
  install: pnpm install
  start: pnpm dev
  terminals:
    - name: app
      command: pnpm dev
  healthcheck: http://127.0.0.1:1420

hooks:
  after_create: pnpm install
  before_run: git status --porcelain
  after_run: pnpm prettier --check .
  timeout_seconds: 60

quality:
  commands:
    - npm run lint
    - npm test -- --run
    - npm run build
  require_clean_worktree: true
  require_review: true
  require_plan_approval: false
  browser:
    enabled: false
    routes: []

budgets:
  max_task_minutes: 120
  max_attempt_tokens: 200000
  max_task_cost_usd: null
  warn_at_percent: 80

routing:
  planner: planner
  implementation: worker
  review: reviewer

handoff:
  target: human-review
  auto_apply: false
  auto_publish_draft_pr: false
---
Complete the objective and satisfy every acceptance criterion.

Use the repository instructions as the source of truth. Record assumptions,
verification evidence, and remaining risks.
```

Requirements:

- typed schema with unknown-field rejection;
- explicit schema migrations and compatibility tests;
- strict template variables for task, attempt, labels, blockers, and
  acceptance criteria;
- secret references, never inline secret values;
- UI editor and raw Markdown editing;
- validation diagnostics with line/field context;
- live reload for future dispatches;
- immutable effective-config snapshot on every attempt;
- visible diff between repository config and per-run overrides.

## 7. Implementation workstreams

The sequence below is dependency-ordered. Each numbered item should be one
focused PR unless its acceptance gate requires an inseparable atomic change.

### Milestone A — Reliable local orchestration

#### A0. Contracts and feature boundary

1. Write versioned Rust domain types and transition table.
2. Define normalized task, attempt, event, approval, artifact, and runner DTOs.
3. Add `orchestration_v2` feature flag and read-only shadow mode.
4. Add architecture dependency tests so UI cannot call runner internals and
   sources cannot mutate scheduler state.

Acceptance:

- transition table has exhaustive tests;
- invalid transitions fail closed;
- v2 can inspect current local todos without claiming them;
- current v1 behavior is unchanged when the flag is disabled.

#### A1. SQLite orchestration ledger

1. Add migrations, schema-version rejection, private-file permissions, and
   bounded queries.
2. Add append-only orchestration events and transactional projections.
3. Add task/attempt/lease/approval/artifact/outbox tables.
4. Add cursor pagination and workspace authorization to all IPC list commands.
5. Add snapshot export suitable for deterministic tests and support bundles.

Acceptance:

- migration is idempotent and rejects a future schema;
- conflicting duplicate events are rejected;
- identical duplicate events are accepted as no-ops;
- first terminal event wins;
- cross-workspace reads and writes fail closed;
- database corruption produces an operator-visible error and never silently
  starts a second empty scheduler.

#### A2. Rust coordinator service

1. Move ticking, claims, backoff, and terminal reconciliation out of React.
2. Start one coordinator actor per authorized workspace.
3. Add wakeups for source changes, runner events, workflow changes, retry
   deadlines, and explicit UI commands.
4. Add pause, graceful stop, forced stop, and app-shutdown semantics.
5. Emit Tauri events from committed ledger events.

Acceptance:

- closing the Project Board does not affect execution;
- renderer reload does not affect execution;
- pausing stops new claims but not active attempts;
- stopping requests cancellation and reaches a deterministic terminal state;
- no polling interval in React is required for correctness.

#### A3. Recovery and legacy migration

1. Import legacy running/paused intent from `altai-orchestration.json`.
2. Link existing orchestrator assignments to imported attempts.
3. Inspect incomplete native runs through the current runtime recovery API.
4. Reconcile worktrees before reclaiming leases.
5. Preserve retry counts and effective workflow snapshots.
6. Mark ambiguous state `NeedsAttention` instead of guessing.

Acceptance:

- crash after claim but before dispatch does not duplicate a run;
- crash after dispatch but before persistence reconnects to the existing run;
- crash after runner completion but before projection finalizes replays once;
- stale workspaces are reported, not silently deleted;
- migration can be rerun safely.

### Milestone B — Runner and policy platform

#### B1. RunnerAdapter abstraction

1. Define runner capabilities: streaming, resume, steering, approvals, dynamic
   tools, usage, rate limits, and structured output.
2. Implement `NativeRunnerAdapter` around the existing ALTAI runtime.
3. Normalize existing run events into orchestration events without duplicating
   persistence.
4. Route cancellation and clarification through the owning runtime only.
5. Add a deterministic `MockRunnerAdapter`.

Acceptance:

- current native agent behavior passes through the adapter without feature
  loss;
- runner identity is immutable per attempt;
- a runner cannot emit events for another task/workspace;
- mock runner can script success, failure, stall, approval, and malformed
  events.

#### B2. Policy and approval engine

1. Add action risk classification.
2. Combine managed requirements, app settings, workflow config, agent profile,
   and per-run overrides.
3. Implement allow, deny, ask, and auto-review decisions.
4. Persist approval requests and decisions.
5. Add expiry and unattended-run policy.
6. Preserve current global bypass safety gate.

Acceptance:

- policy evaluation is deterministic and unit-tested;
- child/subagent policy cannot exceed parent authority;
- unavailable human approval cannot stall forever;
- approval decisions show the exact source layer that produced them.

#### B3. Lifecycle hooks

1. Implement `session_start`, `before_tool`, `after_tool`, `before_edit`,
   `after_edit`, `before_apply`, `after_run`, `on_error`, and
   `before_cleanup`.
2. Use structured JSON input/output and explicit blocking decisions.
3. Add timeouts, output limits, cwd enforcement, and secret redaction.
4. Support project hooks and locked managed hooks.
5. Build a read-only hook inspector in Settings.

Acceptance:

- a blocking hook prevents the action;
- a failed observability hook does not crash orchestration;
- hook commands cannot escape the assigned workspace;
- managed hooks cannot be disabled by repository configuration.

#### B4. Codex App Server adapter

1. Detect and validate the installed `codex` binary and app-server protocol.
2. Generate or vendor the targeted protocol schema; do not hand-maintain
   guessed JSON-RPC payloads.
3. Implement process lifecycle, initialization, thread start/resume, turn
   start, streaming updates, cancellation, and shutdown.
4. Preserve thread identity across continuation turns.
5. Map approvals, input requests, usage, rate limits, file changes, and tool
   calls into normalized events.
6. Keep protocol stdout separate from diagnostics.
7. Add read, silence, turn, and process-exit timeouts.
8. Advertise only supported client tools.

Acceptance:

- incompatible protocol versions fail before a task is claimed;
- a continuation uses the same thread and a new turn;
- malformed/unsupported messages are bounded and operator-visible;
- process exit and silence timeout schedule the correct retry kind;
- tracker credentials are not inherited by the child process;
- adapter conformance tests run against fixtures and an optional installed
  Codex smoke test.

### Milestone C — Operations UX

#### C1. Activity stream and read model

1. Create paginated task and attempt read APIs.
2. Subscribe to committed events with replay-from-sequence support.
3. Add reconnect handling and gap detection.
4. Render human-readable summaries from normalized events.
5. Preserve raw bounded diagnostics behind an advanced disclosure.

Acceptance:

- UI can reload and resume from its last event sequence;
- missing sequences trigger replay, not silent gaps;
- activity display is derived from backend state and is not required for
  correctness.

#### C2. Run Inspector

Add tabs:

- Overview;
- Plan;
- Activity;
- Changes;
- Verification;
- Approvals;
- Evidence;
- Cost and usage;
- Diagnostics.

Actions:

- send follow-up;
- steer;
- approve/deny;
- pause task;
- cancel attempt;
- retry with same or changed configuration;
- reassign agent profile;
- open worktree;
- compare attempt configurations;
- export support bundle.

Acceptance:

- every action is capability- and state-gated;
- stale actions fail with a current-state response;
- the inspector works for native and Codex runners through the same UI model.

#### C3. Board evolution

1. Add configurable columns mapped to task states.
2. Show blocked dependencies, approvals, verification, and review status.
3. Add priority, labels, owner profile, budget, and source filters.
4. Add bulk pause/cancel/retry/reassign with confirmation and bounded scope.
5. Add saved local views.
6. Keep GitHub-only actions hidden or disabled when disconnected.

Acceptance:

- local boards remain fully functional without integrations;
- dragging a card proposes a valid state transition and cannot bypass gates;
- remote source failures do not block local board use.

### Milestone D — Verification and proof of work

#### D1. Command quality gates

1. Parse required checks from `WORKFLOW.md`.
2. Execute checks through `WorkspaceExecutor`.
3. Stream bounded output and preserve full log artifacts.
4. Classify pass, fail, timeout, skipped, and unavailable.
5. Add retry policy per check.
6. Prevent `ReadyForHandoff` while required checks fail.

Acceptance:

- check results are reproducible and linked to workspace commit/diff identity;
- a later edit invalidates prior check evidence;
- timeout kills child processes and releases capacity.

#### D2. Automated reviewer

1. Add a read-only reviewer profile.
2. Supply base/head diff, task criteria, test evidence, and relevant repository
   instructions.
3. Require structured findings with severity, file, line, evidence, and
   suggested validation.
4. Deduplicate findings across review iterations.
5. Allow a bounded worker-reviewer correction loop.

Acceptance:

- reviewer cannot edit;
- unresolved blocking findings prevent handoff;
- style-only findings cannot block unless configured;
- every corrected finding links to the correcting attempt/turn.

#### D3. Evidence store

1. Add content-addressed artifact storage in ALTAI application data.
2. Enforce per-artifact and per-task size limits.
3. Redact secrets before persistence.
4. Store checksums and producer identity.
5. Add retention, pin, export, and cleanup policies.
6. Produce a final proof-of-work summary.

Acceptance:

- evidence survives app restart;
- missing/corrupt artifacts are detected by checksum;
- cleanup never follows untrusted paths;
- exported bundles contain no raw credentials.

#### D4. Safe delivery

1. Require clean base/worktree and current evidence revision.
2. Preview commits and diff before apply.
3. Keep current conflict-abort semantics.
4. Add explicit post-apply verification.
5. Support draft PR publication through a source capability.
6. Keep auto-apply and auto-merge disabled by default.

Acceptance:

- delivery is idempotent;
- failed apply leaves the target unchanged;
- source publication failure does not lose local work;
- the task reaches `Done` only under the configured handoff policy.

### Milestone E — Reproducible environments and browser QA

#### E1. Environment profile

1. Parse install, start, terminal, cache, healthcheck, and environment revision.
2. Run setup only in the isolated workspace.
3. Cache safe dependency state by repository/environment revision.
4. Track long-lived processes and terminate them on cleanup.
5. Surface environment health and setup logs.

Acceptance:

- setup is idempotent;
- a worktree can boot the application independently;
- stale cache keys cannot cross repositories;
- background processes cannot outlive cleanup unnoticed.

#### E2. Browser verification

1. Add an opt-in browser QA adapter.
2. Start the app from its environment profile.
3. Wait for the configured healthcheck.
4. Drive declared routes/journeys.
5. Capture screenshots, console errors, network failures, and optional video.
6. Compare required snapshots with explicit tolerance and review workflow.

Acceptance:

- browser work is isolated per attempt;
- visual artifacts identify commit, route, viewport, and timestamp;
- a browser failure is distinguishable from an application assertion failure;
- no external site credentials are exposed without explicit configuration.

#### E3. Docker executor

Implement the Docker phase in `docs/REMOTE_ROADMAP.md` against the shared
`WorkspaceExecutor` contract.

Acceptance:

- the same task/runner works in local worktree and Docker;
- host/container path mapping is explicit;
- executor capability differences are visible before dispatch.

### Milestone F — Task graphs and agent teams

#### F1. Dependencies and scheduler DAG

1. Add dependency CRUD and cycle detection.
2. Import provider-native blockers when available.
3. Dispatch by priority, dependency readiness, age, and configured fairness.
4. Show critical path and blocked reason.
5. Recompute eligibility transactionally after upstream completion.

Acceptance:

- cycles are rejected with a useful path;
- blocked tasks never consume runner capacity;
- completing one task atomically unblocks eligible dependents.

#### F2. Planning and decomposition

1. Add a planner run type with read-only tools.
2. Produce a structured plan and proposed task graph.
3. Require user approval before creating or changing multiple tasks by default.
4. Preserve the approved plan as an artifact and execution baseline.
5. Track deviations and decisions.

Acceptance:

- plan approval is version-specific;
- editing a plan invalidates the previous approval;
- task creation is atomic or reports a resumable partial failure.

#### F3. Agent profiles

1. Add repository profiles under `.altai/agents/`.
2. Support name, description, prompt, model, reasoning, permissions, tools,
   skills, MCP servers, budgets, and file scope.
3. Add user and managed scopes with visible precedence.
4. Add manual selection and capability-based inference.
5. Validate unavailable models/tools before dispatch.

Acceptance:

- a profile cannot broaden managed permissions;
- inferred selection is recorded and explainable;
- missing dependencies fail before workspace side effects.

#### F4. Team coordinator and mailbox

1. Add parent/child task and attempt relationships.
2. Add bounded agent-to-coordinator messages.
3. Add direct user steering of a child attempt.
4. Add shared task-list claim semantics.
5. Add conflict detection for overlapping file ownership.
6. Require explicit approval before expensive fan-out by default.

Acceptance:

- child contexts are isolated;
- child results return as structured summaries/artifacts;
- message delivery is durable and exactly-once at the recipient boundary;
- parallel writes never share one worktree.

#### F5. Integration coordinator

1. Detect overlapping diffs and dependency ordering.
2. Merge completed work into an integration worktree.
3. Run combined verification.
4. Route conflicts to a dedicated resolution attempt.
5. Preserve each child commit and evidence lineage.

Acceptance:

- successful child tasks cannot silently overwrite each other;
- combined tests run against the actual integrated revision;
- unresolved conflicts produce `NeedsAttention`, not repeated blind retries.

### Milestone G — Repository intelligence and learning

#### G1. Agent Readiness scan

Score and explain:

- repository instructions and navigation;
- architecture documentation;
- test/build discoverability;
- environment bootability;
- dependency and schema documentation;
- security and reliability guidance;
- stale or conflicting instructions;
- worktree compatibility;
- browser observability.

Acceptance:

- every score links to evidence;
- scans are local and read-only by default;
- recommendations never modify the repository without approval.

#### G2. Context-pack builder

1. Treat `AGENTS.md` as a concise map.
2. Index repository-owned architecture, product, reliability, security, and
   execution-plan documents.
3. Select a bounded task-specific context pack.
4. Record every included source and revision.
5. Detect stale links and oversized instruction surfaces.

Acceptance:

- the same inputs produce the same context manifest;
- context limits are enforced before dispatch;
- hidden external knowledge is not claimed as repository truth.

#### G3. Execution plans and decision logs

1. Support lightweight and checked-in execution plans.
2. Record progress, decisions, assumptions, and verification evidence.
3. Link tasks and attempts to plan revisions.
4. Add plan freshness and completion checks.

Acceptance:

- long tasks can resume from repository artifacts without replaying an entire
  transcript;
- decisions remain reviewable after session cleanup.

#### G4. Session analysis and playbooks

1. Analyze successful, failed, expensive, and abandoned attempts.
2. Compare paths, retries, tool use, and missing context.
3. Propose a reusable playbook, hook, documentation change, or quality rule.
4. Require user review before saving any learning.
5. Version playbooks under `.altai/playbooks/`.

Acceptance:

- generated learning cites the runs that motivated it;
- secrets and raw sensitive logs are excluded;
- workflow changes are proposed as diffs and never self-merge.

#### G5. Continuous repository gardening

Add opt-in scheduled tasks for:

- stale documentation;
- architecture violations;
- flaky tests;
- dead code;
- dependency drift;
- repeated agent failure patterns;
- evidence retention and stale worktrees.

Acceptance:

- gardening produces small reviewable tasks or PRs;
- schedules honor budgets and quiet hours;
- cleanup is recoverable where practical.

### Milestone H — Cost, routing, and evaluation

#### H1. Usage and budget accounting

1. Normalize tokens, duration, rate limits, and provider costs when available.
2. Aggregate per attempt, task, project, runner, model, and date.
3. Add warnings and hard budget stops.
4. Separate model usage from local compute estimates.
5. Show unavailable/estimated values honestly.

Acceptance:

- absolute usage updates are not double-counted;
- retries retain separate cost lineage;
- a budget stop reaches a deterministic task state.

#### H2. Smart routing

1. Route by task type, required capabilities, risk, budget, and latency class.
2. Start with explicit deterministic rules.
3. Add an optional classifier only after rule-based routing is observable.
4. Record why a runner/model/profile was chosen.
5. Support user pinning.

Acceptance:

- routing never selects an unavailable or unauthorized target;
- automatic fallback cannot silently weaken quality or permission policy;
- selection is replayable from the effective configuration.

#### H3. Evaluation and Replay Lab

Build a local developer surface for:

- scripted mock runner scenarios;
- recorded event replay;
- crash injection at every transition;
- approval and input timeouts;
- malformed runner events;
- source rate limits and outages;
- Git conflicts;
- workspace setup failures;
- 1,000-task scheduler load;
- deterministic UI fixtures.

Acceptance:

- core orchestration tests require no paid model calls;
- a recorded production support bundle can be sanitized and replayed;
- CI runs failure matrices with deterministic seeds.

#### H4. Quality dashboard

Track:

- task success without retry;
- retry and abandonment rates;
- duplicate dispatch count;
- recovery success;
- median time to first activity and handoff;
- verification failure rate;
- reviewer finding recurrence;
- apply/publish failure;
- token/cost per completed task;
- human approvals and steering frequency;
- stale workspace count.

Acceptance:

- metrics derive from committed events;
- dashboards do not affect orchestration correctness;
- privacy and retention controls apply to analytics.

### Milestone I — External sources, remote workers, and collaboration

#### I1. Source adapter framework

1. Complete `LocalTaskSource`.
2. Refactor current GitHub code behind `GitHubTaskSource`.
3. Add capability negotiation and an outbox for remote mutations.
4. Add public read-only GitHub loading as a separate anonymous capability.
5. Add Linear and Jira only after local/GitHub conformance tests pass.

Acceptance:

- source adapters pass one shared conformance suite;
- disconnected or degraded providers do not block local work;
- remote writes are explicit and idempotent.

#### I2. Credential broker and provider-native tools

1. Keep credentials in the OS keychain or managed host environment.
2. Expose narrow host-side tools to runners.
3. Bind tools to the current source/task context.
4. Redact credentials and sensitive headers from events.
5. Add revoke and audit UI.

Acceptance:

- runner child processes do not receive raw tracker credentials;
- a tool cannot mutate a different task/repository;
- revocation affects new calls immediately.

#### I3. SSH and remote worker pool

Implement the SSH and cloud-worker milestones from `docs/REMOTE_ROADMAP.md`
through `WorkspaceExecutor`.

Add:

- worker health and capacity;
- per-host concurrency;
- environment revision and drift detection;
- sticky retry placement;
- explicit failover semantics;
- remote cleanup visibility.

Acceptance:

- local and remote runs use the same orchestration contract;
- loss of a host cannot duplicate an active attempt;
- capacity exhaustion waits instead of silently changing execution mode.

#### I4. Notifications and collaboration

1. Add task comments and user-to-agent follow-ups.
2. Add desktop notifications for approval, input, failure, and handoff.
3. Reuse safe Slack/email/channel work only after the gates in the Claw plan.
4. Add shareable read-only support bundles before live shared sessions.
5. Add mobile/cloud approval only after authenticated remote orchestration is
   available.

Acceptance:

- notifications link to the exact task/attempt;
- external replies are authenticated and origin-bound;
- one reply cannot resume multiple runtimes.

## 8. Security requirements

These are release gates, not later hardening:

1. Normalize and authorize every workspace path in Rust.
2. Never trust model-provided task IDs, destinations, cwd, repository identity,
   or credentials.
3. Keep tracker and integration credentials out of runner environments.
4. Redact known secret patterns before logs/events/artifacts are persisted.
5. Use bounded payloads, output, line sizes, artifact sizes, and query limits.
6. Separate read-only, workspace-write, network, source-write, apply, publish,
   merge, and destructive capabilities.
7. Require explicit policy for outbound network access.
8. Make higher-risk actions reviewable and auditable.
9. Bind approvals to the exact attempt, action hash, and expiry.
10. Invalidate approval when the action changes.
11. Treat symlinks and path traversal as hostile at workspace boundaries.
12. Do not allow repository config to disable managed requirements.
13. Keep auto-apply, auto-push, and auto-merge off by default.
14. Export a redacted audit trail without exposing raw prompts or credentials
    unless the user explicitly includes them.

## 9. Test strategy

### 9.1 Unit and property tests

- state transition table;
- lease acquisition, expiry, and generation;
- retry/backoff with deterministic clocks;
- config parsing and precedence;
- strict prompt/template rendering;
- policy decisions;
- hook decisions and timeouts;
- routing;
- dependency cycle detection;
- token aggregation;
- redaction and path normalization.

Use property tests for transition invariants, idempotency, and event replay.

### 9.2 Integration tests

- SQLite migration and concurrent first-open;
- coordinator plus mock task source and mock runner;
- native runner conformance;
- Codex protocol fixture conformance;
- worktree create/run/retry/apply/cleanup;
- crash and restart at every durable transition;
- approval and input continuation;
- source outbox retry;
- evidence checksum and retention;
- combined multi-agent integration.

### 9.3 UI tests

- board state and filters;
- Run Inspector event replay;
- approval stale-state handling;
- workflow editor validation;
- evidence and diff rendering;
- keyboard and screen-reader operation;
- offline/disconnected source behavior.

### 9.4 Soak and chaos tests

Minimum release scenarios:

1. 1,000 queued tasks, eight concurrent workers, zero duplicate attempts.
2. Kill and restart ALTAI after every state transition.
3. Repeated runner silence, crash, malformed event, and rate-limit signals.
4. Repeated renderer reload while work continues.
5. Remote source unavailable for one hour while local work continues.
6. Workspace disk full, permission denied, and stale lock.
7. Conflicting apply and dirty target workspace.
8. Twenty-four-hour run with bounded logs and stable memory usage.

## 10. Rollout milestones

### Preview 1 — Durable local autopilot

Includes A0–A3 and B1 with the native runner.

Exit gate:

- no React scheduler;
- restart recovery passes;
- zero duplicate dispatch in soak tests;
- local board remains GitHub-independent.

### Preview 2 — Inspectable and governed execution

Includes B2–B4 and C1–C3.

Exit gate:

- Codex App Server opt-in runner works;
- approvals and steering are durable;
- complete activity timeline is available.

### Preview 3 — Verifiable delivery

Includes D1–D4 and E1–E2.

Exit gate:

- configured checks and review block handoff correctly;
- proof-of-work bundle is generated;
- UI tasks can produce browser evidence.

### Preview 4 — Multi-agent project execution

Includes F1–F5.

Exit gate:

- task dependencies and decomposition are safe;
- parallel agents never share a worktree;
- integration verification runs on the combined result.

### Preview 5 — Learning and scale

Includes G, H, and I in independently releasable slices.

Exit gate:

- learning is reviewable and opt-in;
- budgets and routing are explainable;
- source/worker adapters pass conformance;
- remote execution does not weaken local security invariants.

## 11. Success metrics

North-star metrics:

- duplicate dispatch rate: exactly zero;
- crash recovery success: greater than 99.9% in deterministic chaos runs;
- required verification bypasses: zero;
- tasks completed without manual steering;
- median human attention minutes per completed task;
- successful apply/publish rate;
- retry rate by runner/model/task type;
- recurring reviewer finding rate;
- cost and time per accepted task;
- stale workspace and leaked process count;
- local-only task completion rate without integrations.

Do not optimize raw PR count at the expense of accepted-task rate, regression
rate, security, or human review burden.

## 12. PR and review policy

- Prefer one contract or vertical slice per PR.
- Every backend behavior PR includes Rust tests.
- Every persisted schema PR includes migration, future-schema, corruption, and
  concurrency tests.
- Every runner PR includes conformance fixtures.
- Every security-sensitive PR includes abuse cases.
- Every UI PR includes loading, empty, error, stale-state, and keyboard paths.
- No milestone merges with ignored failing tests or undocumented recovery
  semantics.
- Update this plan after each merged slice with status, decisions, and deviations.

Expected size: approximately 25–35 focused PRs across the previews. Several UI,
adapter, and evaluation slices can proceed in parallel only after their shared
contracts are merged.

## 13. Explicit non-goals for the first previews

- ALTAI does not execute while the desktop process is closed until the remote
  worker milestone exists.
- No default autonomous merge to protected branches.
- No arbitrary unreviewed code downloaded and executed as a hook.
- No silent model/provider fallback that changes permissions or quality level.
- No requirement to connect GitHub for local project management.
- No self-modifying workflow or knowledge without a user-reviewed diff.
- No simultaneous legacy and v2 coordinator claiming the same queue.

## 14. Principal risks and mitigations

| Risk                                      | Mitigation                                                  |
| ----------------------------------------- | ----------------------------------------------------------- |
| Two schedulers claim the same task        | feature-gated single ownership, leases, idempotency keys    |
| Codex protocol drift                      | generated versioned schemas and adapter conformance tests   |
| Renderer becomes a hidden coordinator     | Rust service owns all decisions; UI is a projection         |
| Agent completion is mistaken for delivery | verification, review, and handoff gates                     |
| Parallel agents conflict                  | one worktree per attempt and integration coordinator        |
| Credentials leak to runners or logs       | host-side broker, narrow tools, redaction                   |
| Workflow complexity becomes unusable      | safe defaults, progressive UI, raw advanced editor          |
| Costs grow unexpectedly                   | per-attempt/task budgets, warnings, deterministic routing   |
| Repository guidance rots                  | readiness scan, doc checks, reviewed gardening              |
| Remote host failure duplicates work       | leases, sticky ownership, explicit failover attempts        |
| Evidence consumes disk                    | content addressing, quotas, retention, pin/cleanup controls |
| Scope stalls delivery                     | preview gates and independently releasable vertical slices  |

## 15. Recommended first implementation sequence

Start with these six PRs:

1. **O1 — Domain contract and exhaustive state-transition tests**
2. **O2 — SQLite orchestration ledger and migrations**
3. **O3 — Mock runner, local task source, and deterministic coordinator tests**
4. **O4 — Rust coordinator actor and Tauri event subscription**
5. **O5 — Legacy intent/assignment recovery migration**
6. **O6 — Native runner adapter and v2 local-board opt-in**

This sequence produces a reliable local foundation before adding Codex App
Server, richer UI, multi-agent behavior, integrations, or remote execution.

## 16. Primary references

- OpenAI Symphony specification:
  <https://github.com/openai/symphony/blob/main/SPEC.md>
- OpenAI Symphony engineering overview:
  <https://openai.com/index/open-source-codex-orchestration-symphony/>
- OpenAI harness engineering:
  <https://openai.com/index/harness-engineering/>
- OpenAI Codex safety controls:
  <https://openai.com/index/running-codex-safely/>
- GitHub Copilot custom agents and hooks:
  <https://docs.github.com/en/copilot/how-tos/copilot-sdk/features/custom-agents>
- Google Jules API session/activity model:
  <https://developers.google.com/jules/api>
- Cursor Background Agents:
  <https://docs.cursor.com/background-agent>
- Claude Code agent teams:
  <https://code.claude.com/docs/en/agent-teams>
- Devin advanced orchestration and playbooks:
  <https://docs.devin.ai/work-with-devin/advanced-capabilities>
