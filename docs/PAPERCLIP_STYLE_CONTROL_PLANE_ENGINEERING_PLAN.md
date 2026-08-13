# ALTAI Paperclip-Style Control Plane Engineering Plan

> **Execution order:** `docs/control-plane-execution/WORK_OS_PROGRAM_BACKLOG.md`
> is the canonical project-management queue. This document remains authoritative
> for architecture and scope.

> Status: proposed canonical implementation plan
>
> Date: 2026-08-03
>
> Scope: Desktop, IDE extension, Studio, CLI, and application plugins

## 1. Decision

ALTAI will not embed, fork, or repackage Paperclip. It will adopt the control-plane
semantics that make Paperclip reliable while preserving ALTAI's native execution
advantages.

The product boundary is:

```text
ALTAI Control Plane decides what should run, why, when, for whom, and under which policy.
IsanAgent executes one authorized attempt and reports exactly what happened.
```

This plan corrects any existing implementation that violates that boundary, even if the
implementation is already functional. Compatibility is preserved through migrations and
adapters, not by retaining competing owners.

If accepted, this document governs control-plane ownership and implementation order. The
existing Agent Operations plan remains useful for detailed quality, delivery, and testing
requirements, but conflicting ownership or lifecycle decisions must be amended to match
this plan.

For implementation through a fresh-context GLM 5.1 coding agent, use
`docs/GLM_5_1_CONTROL_PLANE_EXECUTION_PLAYBOOK.md`. It defines task packets, risk tiers,
context bootstrap, verification, review, and stop conditions; it does not supersede this
architecture.

## 2. Product Outcome

ALTAI should behave like a local-first agent operations control plane:

1. A user defines an organization/workspace, goals, projects, agents, and work.
2. Every work item can explain its ancestry and intended outcome.
3. Assignment, comments, approvals, routines, and dependency changes create durable wake
   requests.
4. A single coordinator atomically checks policy, budget, dependencies, workspace, and
   ownership before dispatch.
5. IsanAgent receives one immutable attempt specification, performs the work, and emits a
   durable event stream.
6. The control plane evaluates the result, verification, review, cost, and next action.
7. Desktop, IDE, Studio, CLI, and plugins see the same identities and projections.
8. Scheduled work can continue without a renderer and, when enabled, without an open UI.
9. GitHub and future trackers are integrations, not the core source of truth.

## 3. Non-Negotiable Architecture Rules

### 3.1 One control-plane owner

Exactly one service owns:

- work-item lifecycle;
- assignment and checkout;
- dependency eligibility;
- wake request coalescing;
- routine scheduling;
- retries and recovery;
- budget hard stops;
- authoritative terminal disposition;
- external-source reconciliation.

React, Tauri commands, IDE webviews, plugins, and IsanAgent may request a transition. They
may not perform an authoritative transition independently.

### 3.2 IsanAgent remains the primary execution runtime

IsanAgent continues to own:

- provider/model execution;
- one run's conversation/session context;
- tool invocation;
- permission enforcement for filesystem, shell, MCP, and edits;
- checkpoints and run-level recovery;
- steering and cancellation of an active run;
- run-internal subagents;
- compaction and model failover;
- sequenced run events and transcript replay.

IsanAgent must not own:

- project tasks;
- organization or project state;
- durable task assignment;
- cross-run task dependencies;
- project routines;
- company/project/agent budget policy;
- GitHub synchronization;
- business approvals;
- the canonical work inbox.

### 3.3 One identity per concept

The following IDs are distinct and must never be substituted for one another:

```text
organization_id
goal_id
project_id
workspace_id
agent_instance_id
agent_profile_id + agent_profile_revision_id
work_item_id
attempt_id
run_id
session_id
routine_id + routine_revision_id + routine_run_id
approval_id
external_object_id
```

Every bridge stores explicit mappings. It must not infer identity from a title, chat name,
GitHub number, filesystem path, or array position.

### 3.4 Parentage is not dependency

- `parent_work_item_id` explains decomposition and goal ancestry.
- `work_dependencies` controls execution eligibility.
- A child does not automatically block its parent.
- If a parent must wait for a child, a dependency edge must exist.

### 3.5 Configuration and state are separate

Version-controlled repository configuration remains in `ALTAI.md`, `WORKFLOW.md`, and
`.altai/` profile files. Runtime state, leases, costs, comments, approvals, and event
positions live in the control-plane database.

### 3.6 No second scheduler during migration

Shadow evaluation may compare eligibility decisions, but only one scheduler may claim or
dispatch. Feature flags must select a single writer.

### 3.7 IsanAgent cron is host-selectable, not removed

The IsanAgent `cron` tool remains a supported IsanAgent capability. ALTAI does not require
its removal or weaken standalone and agent-dorm deployments. The embedding host selects one
schedule backend for a runtime:

| Host mode | Agent-visible intent | Backend owner |
| --- | --- | --- |
| IsanAgent standalone/local | `cron` | IsanAgent `CronActor` in local mode |
| IsanAgent agent-dorm / multi-tenant edge | `cron` | IsanAgent multi-tenant-edge cron scheduler |
| ALTAI legacy compatibility | `cron` | existing IsanAgent local cron during bounded migration only |
| ALTAI managed control plane | `cron` compatibility contract, optionally richer Routine affordances | ALTAI `routines.*` command port and control-plane scheduler |

“Suppress cron in ALTAI” means suppressing registration/startup of the native cron backend,
not deleting the tool capability from IsanAgent. In managed mode the agent may still call a
tool named `cron`; ALTAI provides its implementation and bridges the intent to a Routine.
Exactly one backend is registered for a run/workspace, and the selected backend is visible
in diagnostics and the attempt specification.

## 4. Corrected Domain Vocabulary

### 4.1 Durable work objects

| Object | Meaning | Lifetime |
| --- | --- | --- |
| Organization | Isolation, policy, agent, and budget boundary | Durable |
| Goal | Desired outcome and ancestry | Durable |
| Project | Work, repository/workspace, policy, and delivery context | Durable |
| WorkItem | User- or agent-visible unit of project work | Durable |
| Dependency | Execution blocker between work items | Durable |
| WorkComment | Human/agent/system communication on work | Durable |
| Attempt | One coordinator-authorized execution attempt | Durable |
| Run | IsanAgent execution corresponding to an attempt | Durable |
| Routine | Versioned recurring or event-triggered work definition | Durable |
| WakeRequest | Idempotent request to evaluate/run an agent | Durable |
| Approval | Governance decision with explicit scope and payload | Durable |

### 4.2 Run-internal objects

These are not project-management tasks:

- `todo_write` items are `RunPlanItem` records.
- IsanAgent subagents are `SubagentRun` records under one parent run.
- background shell/process jobs are run execution resources.
- a chat session is execution context, not task ownership.

If a subagent needs independently owned, scheduled, reviewable work, the parent must ask
the control plane to create a child `WorkItem`. That child receives its own attempt and run.

### 4.3 Agent profile versus agent instance

`AgentProfile` is reusable configuration:

- instructions;
- model/reasoning defaults;
- tools and skills;
- MCP servers;
- permissions;
- file scopes;
- default attempt budgets.

`AgentInstance` is a durable worker identity:

- name, role, title, and capabilities description;
- organization and project memberships;
- `reports_to_agent_id`;
- active profile revision;
- status and pause reason;
- aggregate budget policy;
- current assignments, sessions, and runs.

Multiple agent instances may use one profile. Changing a profile creates a revision; an
active attempt keeps the immutable revision it started with.

## 5. State Model

### 5.1 Work status

The human- and integration-facing lifecycle is deliberately small:

```text
backlog -> todo -> in_progress -> in_review -> done
               \-> blocked
               \-> cancelled
```

Canonical values:

- `backlog`
- `todo`
- `in_progress`
- `in_review`
- `blocked`
- `done`
- `cancelled`

### 5.2 Execution phase

Execution detail is stored separately:

- `none`
- `queued`
- `planning`
- `awaiting_plan_approval`
- `running`
- `awaiting_input`
- `awaiting_approval`
- `verifying`
- `reviewing`
- `retrying`
- `paused`
- `failed`
- `needs_attention`
- `terminal`

This separation prevents external tracker status mappings from erasing ALTAI's richer
execution lifecycle.

### 5.3 Attempt state

```text
created -> claimed -> dispatched -> running -> finalizing -> terminal
```

Terminal outcomes:

- `succeeded`
- `failed`
- `cancelled`
- `timed_out`
- `budget_stopped`
- `policy_denied`
- `lost`

Task completion is not inferred from a successful model response. Verification, review,
delivery policy, and required artifacts determine the work item's final disposition.

## 6. Target Runtime Topology

```text
Desktop ---------+
IDE extension ---+
Studio ----------+---- versioned client protocol ----+
CLI -------------+                                    |
Trusted plugins -+                                    v
                                             altai-control-plane
                                      identities / work / routines / audit
                                                |             |
                                                |             +--> plugin workers
                                                v
                                        altai-agent-service
                                                |
                                             IsanAgent
                                                |
                                  workspace / tools / MCP / providers
```

### 6.1 Process ownership

The final topology uses one user-scoped `altai-control-plane` daemon:

- Desktop connects to or starts it.
- IDE/Studio connects to the existing instance.
- CLI connects to the same instance.
- A process lock prevents a second coordinator.
- Unix uses a user-scoped domain socket; Windows uses a named pipe.
- The protocol is transport-neutral and can later support authenticated loopback or remote
  HTTP/WebSocket without changing domain contracts.

The control-plane crate must also run in-process in deterministic tests. Production
Desktop orchestration must not depend on a React component being mounted.

### 6.2 Persistence

Use two explicit persistence planes:

1. User-scoped control database in application data:
   - organizations, goals, projects, agents, work, routines, budgets, activity;
   - SQLite WAL with one writer owned by the control-plane process.
2. Existing workspace/run journals:
   - detailed IsanAgent event streams, transcripts, checkpoints, and replay;
   - referenced by stable `run_id` and a control-plane `run_locator`.

Do not move all run journals in the first migration. First establish stable identity and
ownership; storage consolidation can be a later decision.

## 7. Module Dependency Map

```text
CP-00 Architecture fence
  -> CP-01 Shared domain contracts
      -> CP-02 Control database
          -> CP-03 Work domain
          -> CP-04 Organizations / goals / projects
          -> CP-05 Agent registry
          -> CP-06 Hierarchy / dependencies / comments
              -> CP-07 Assignment / checkout / wake queue
                  -> CP-08 IsanAgent attempt adapter
                  -> CP-09 Workspace / delivery
                  -> CP-10 Approval / governance
                  -> CP-11 Usage / cost / budgets
                  -> CP-12 Routines / scheduler
                      -> CP-13 Liveness / recovery
                          -> CP-14 External objects / GitHub
                          -> CP-15 Public protocol
                              -> CP-16 Daemon lifecycle
                              -> CP-17 Read models / UI
                              -> CP-18 Desktop / IDE / Studio / CLI adapters
                              -> CP-19 Application plugin runtime
                                  -> CP-20 Migration / cutover
                                      -> CP-21 Soak / release gates
```

Some modules can overlap after their dependency contracts are merged. No module may invent
temporary identities or state transitions to bypass an unfinished dependency.

## 8. Module-by-Module Engineering Plan

## CP-00 — Architecture Fence and ADR Amendments

### Goal

Make the ownership boundary enforceable before adding features.

### Work

1. Add an ADR for the control-plane/execution-plane split.
2. Amend ADR 0001: durable work may outlive VS Code/Desktop through the control-plane
   daemon.
3. Amend ADR 0002: the protocol will carry control-plane domains in addition to run
   control.
4. Mark the current Agent Operations plan's ownership sections as superseded where they
   conflict.
5. Add dependency rules preventing `altai-agent-service` from importing project/control
   domain modules.
6. Add dependency rules preventing React stores from importing SQLite/Tauri orchestration
   implementation details.

### Exit gate

- The team can answer who owns every mutation without reference to a UI component.
- Architecture tests fail if execution imports control-plane persistence or UI owns a
  scheduler.

## CP-01 — Shared Domain Contracts

### Goal

Define stable Rust and TypeScript contracts before database/UI work.

### Work

Create product-neutral types for:

- IDs and revisions;
- Organization, Goal, Project, ProjectWorkspace;
- AgentProfile, AgentProfileRevision, AgentInstance;
- WorkItem, WorkDependency, WorkComment;
- Attempt, RunBinding, Lease;
- WakeRequest;
- Routine, RoutineRevision, RoutineTrigger, RoutineRun;
- ScheduleBackendMode and RoutineCommandPort request/response contracts;
- Approval;
- BudgetPolicy, CostEvent;
- ExternalObjectLink;
- ActivityEvent and actor identity.

Every mutation command includes:

- actor;
- idempotency key where retryable;
- expected revision where concurrent edits are possible;
- correlation/causation IDs;
- timestamp supplied by the service clock.

### Files

- new `src-tauri/crates/altai-control-protocol/`
- extend `packages/host-contract/`
- golden JSON fixtures under `shared/control-protocol/v1/`

### Exit gate

- Rust and TypeScript validate the same fixtures.
- Invalid IDs, unknown required versions, oversized payloads, and stale revisions fail with
  typed errors.

## CP-02 — Control Database and Repository Layer

### Goal

Create the durable user-scoped source of truth.

### Work

1. Add `altai-control-plane` crate.
2. Add versioned, transactional SQLite migrations.
3. Add repositories with transaction-scoped methods; services must not issue ad hoc SQL.
4. Add append-only `control_events` with per-aggregate sequencing.
5. Add idempotency records and future-schema refusal.
6. Add backup/restore and corruption diagnostics before enabling writes.

### Initial tables

```text
organizations
goals
projects
project_workspaces
agent_profiles
agent_profile_revisions
agent_instances
project_agent_memberships
work_items
work_dependencies
work_comments
attempts
attempt_run_bindings
leases
wake_requests
approvals
routines
routine_revisions
routine_triggers
routine_runs
budget_policies
cost_events
external_objects
activity_events
control_events
idempotency_records
run_locators
```

### Exit gate

- Concurrent first-open is safe.
- Migrations are replay-safe and reject newer schemas.
- An event append plus projection update commits atomically.

## CP-03 — Canonical Work Domain

### Goal

Replace assignment/session/task ambiguity with `WorkItem` as the canonical project work
object.

### Work

1. Implement work create/read/update/archive services.
2. Enforce the split between `WorkStatus` and `ExecutionPhase`.
3. Add priorities, work modes, acceptance criteria, assignee, creator, and responsible user.
4. Add optimistic concurrency via `revision`.
5. Add immutable source/origin attribution.
6. Define work modes: `standard`, `plan`, `ask`, and future typed modes.
7. Prevent terminal rollback except through an explicit reopen policy.

### Existing code changes

- `orchestration/domain.rs`: migrate the current combined task state into the two-axis
  model.
- `orchestration/ledger.rs`: retain compatibility reads while the new repository becomes
  authoritative.
- `assignmentsStore.ts`: stop creating durable IDs; request work creation from the service.

### Exit gate

- Creating work from local board, GitHub, or API produces the same domain shape.
- A session deletion cannot delete a work item.
- A work item can exist before any attempt or chat session.

## CP-04 — Organizations, Goals, Projects, and Workspaces

### Goal

Add the context and ancestry that keeps autonomous work aligned.

### Work

1. Create one default local organization during migration.
2. Support nested goals with cycle detection and owner assignment.
3. Support projects linked to one or more goals.
4. Support multiple repository/workspace bindings per project.
5. Separate project workspace identity from canonical filesystem paths.
6. Resolve run context as organization -> goal ancestry -> project -> work item.
7. Keep company/CEO vocabulary optional in templates, not mandatory in the domain.

### Exit gate

- Every dispatched work item has a complete, bounded goal/project context pack.
- Moving a local checkout does not create a duplicate project.
- Cross-organization reads and writes fail closed.

## CP-05 — Agent Registry and Org Structure

### Goal

Turn file-based personas into reusable profiles and add durable agent identities.

### Work

1. Import built-in and `.altai/agents/` profiles as versioned profile sources.
2. Create AgentInstance CRUD, status, pause/resume/terminate, and reporting lines.
3. Add org-chart cycle detection.
4. Add capabilities descriptions for assignment/routing.
5. Bind each attempt to an immutable effective profile revision.
6. Keep managed restrictions cumulative; project/user layers cannot broaden them.
7. Separate agent availability from active run status.

### Existing code changes

- `orchestration/profiles.rs`: becomes profile resolution/import logic, not agent identity.
- `orchestration/team.rs`: task hierarchy and agent org chart become separate durable
  services.

### Exit gate

- Two agent instances can use the same profile without sharing sessions or budgets.
- A profile edit does not change an in-flight attempt.
- Paused/terminated agents cannot receive new dispatches.

## CP-06 — Work Hierarchy, Dependencies, and Comments

### Goal

Make decomposition, coordination, and communication first-class and durable.

### Work

1. Add parent/sub-work relationships with cycle prevention.
2. Persist blocker edges separately from parentage.
3. Add comments with human, agent, and system attribution.
4. Link comments to creating run/attempt when applicable.
5. Add bounded mention parsing and wake-on-comment policy.
6. Define child-to-parent reporting channels.
7. Route lateral agent coordination through a new assigned work item, not arbitrary writes
   into sibling threads.
8. Persist task mailboxes or replace them with comment/wake primitives; do not keep
   correctness-critical in-memory mailboxes.

### Exit gate

- Parentage alone never makes a task ineligible.
- An unresolved blocker always prevents checkout.
- Agent messages survive process restart and have exactly-once visible attribution.

## CP-07 — Assignment, Atomic Checkout, Leases, and Wake Queue

### Goal

Implement Paperclip-style dispatch correctness around ALTAI's runner.

### Work

1. Assignment creates or coalesces a `WakeRequest`; it does not directly start a run.
2. Add wake sources: assignment, comment, mention, routine, approval result, retry,
   recovery, manual.
3. Claim wake requests transactionally.
4. Check agent status, dependency state, policy, budget, and workspace readiness before
   attempt creation.
5. Acquire task checkout and attempt lease atomically.
6. Distinguish ownership lock from active execution binding.
7. Compare-and-clear locks during finalization.
8. Add bounded retry/backoff and dead-letter/recovery outcomes.
9. Coalesce repeated wakes without losing trigger evidence.

### Exit gate

- Concurrent assignment/comment/schedule events create at most one active attempt per
  exclusive work item.
- A stale terminal run cannot retain checkout ownership.
- A real live owner returns a typed conflict; clients do not retry blindly.

## CP-08 — IsanAgent Attempt Adapter and Boundary Correction

### Goal

Make IsanAgent a clean attempt executor controlled by the control plane.

### Attempt input

```text
attempt_id
work_item_id
agent_instance_id
profile_revision_id
session policy
workspace authorization
prompt/context pack
permission policy
tool/skill/MCP allowlist
scheduling backend
budget envelope
quality requirements
correlation metadata
```

### Work

1. Add `AttemptExecutor` to `altai-agent-service` with start, steer, cancel, inspect, and
   replay operations.
2. Bind one IsanAgent run to one attempt via a durable `RunBinding`.
3. Map all IsanAgent events into the shared event vocabulary without losing raw bounded
   diagnostics.
4. Make run completion a signal to control-plane finalization, not direct task completion.
5. Route steering, clarifications, and tool approvals to the exact run lease.
6. Prevent IsanAgent from mutating project state except through scoped control-plane tools.
7. Preserve IsanAgent sessions, checkpoints, compaction, failover, tools, and subagents.

### Cron correction

IsanAgent's `cron` tool is retained. Its current implementations are valid for standalone
local and agent-dorm/multi-tenant-edge use. The correction applies only when IsanAgent is
embedded in an ALTAI-managed control-plane runtime.

#### Backend selection

Add an explicit host-selected `ScheduleBackendMode`:

```text
NativeLocal
NativeMultiTenantEdge
AltaiManagedRoutines
LegacyAltaiLocalCron       # migration only
```

The selected mode is immutable for an attempt. In `AltaiManagedRoutines` mode:

1. Do not start/register the workspace-native `CronActor` as the authoritative scheduler.
2. Register an ALTAI-provided implementation for the agent-visible `cron` contract.
3. Bridge `add`, `remove`, and `list` to typed control-plane `routines.create`,
   `routines.disable/delete`, and `routines.list` commands.
4. Bind organization, project, workspace, agent, attempt, and actor from trusted execution
   context; never accept model-supplied scope IDs as authority.
5. Return the canonical `routine_id` and next-fire summary while preserving a compatibility
   handle for callers expecting a cron job ID.
6. Have the control-plane scheduler create `RoutineRun` plus the configured WorkItem and/or
   WakeRequest; do not inject an unaudited message directly into a chat.

#### Tool registration seam

ALTAI currently registers IsanAgent `CronTool` unconditionally in the shared instance
builder and then augments the registry. Replace this with one schedule-tool factory/provider
chosen before registration. Do not register a second tool under the same name: the current
ToolRegistry overwrites its map entry but appends duplicate catalog definitions.

Preferred implementation order:

1. Add an ALTAI-side `ScheduleToolProvider`/`RoutineCommandPort` seam to
   `altai-agent-service`.
2. Implement `AltaiRoutineCronBridgeTool` against that port using IsanAgent's public Tool
   contract.
3. Make Desktop/IDE/Studio managed attempts select `AltaiManagedRoutines`.
4. Leave IsanAgent standalone host selection between Local and MultiTenantEdge unchanged.
5. Optionally upstream a clean ToolRegistry `replace/remove` API later; it is not required
   for the first ALTAI bridge if registration is conditional.

Concrete ALTAI seams:

- `altai-agent-service/src/host.rs`: replace the mandatory raw `cron_node` assumption in
  `WorkspaceBundle` with a typed schedule-tool binding/backend selection.
- `altai-agent-service/src/instance_builder.rs`: conditionally register either IsanAgent
  `CronTool` or `AltaiRoutineCronBridgeTool`, never both.
- Desktop and stdio host adapters: create `CronActor` only for explicit native/legacy mode;
  managed mode injects a control-plane Routine client.
- current `agent_automation_*` Tauri commands/runtime helpers: become compatibility adapters
  to Routine commands before their eventual removal from the public UI contract.
- `altai-control-protocol`: carry trusted schedule scope, idempotency, approval requirement,
  and canonical/legacy ID mapping.

#### Contract translation

| IsanAgent cron input | ALTAI Routine field |
| --- | --- |
| `at` | one-shot `RoutineTrigger::At` with explicit timezone/instant |
| `every_seconds` | interval trigger, subject to ALTAI minimum interval and budget policy |
| `cron_expr` | cron trigger plus explicit timezone; never infer local/UTC silently |
| `message` | versioned Work/Wake template instructions |
| trusted current chat/run | source RunBinding and optional parent WorkItem reference |
| generated/returned job ID | compatibility mapping to canonical `routine_id` |

The bridge uses `attempt_id + tool_call_id` as the idempotency key so model/tool retries do
not create duplicate routines. Recurring or cross-project routine creation requires the
`routines.create` capability and may require policy approval. An agent cannot grant itself
future tools, broader workspace scope, a larger budget, or a different target agent through
the cron arguments.

#### Migration path

1. Import existing ALTAI-owned IsanAgent cron records into versioned Routine definitions.
2. Persist `legacy_cron_job_id <-> routine_id` mappings for remove/list compatibility.
3. Keep the legacy native provider available behind a single-writer compatibility flag for
   one stable release; it is never active together with managed scheduling.
4. Redirect legacy Desktop automation commands and UI to Routine commands/projections.
5. Retire ALTAI's direct native-cron ownership after import acceptance, without deleting
   IsanAgent's native or agent-dorm cron implementations.

### Background correction

- Run-scoped background processes remain IsanAgent execution resources.
- Durable work continuation requires a wake, monitor, blocker, child work item, or managed
  runtime service in the control plane.
- A local PID or detached shell is never sufficient liveness evidence.

### Exit gate

- IsanAgent can execute an attempt without knowing organization/project scheduler rules.
- No IsanAgent cron tick can independently create authoritative project work.
- Standalone Local and MultiTenantEdge cron conformance remains unchanged.
- Managed `cron add` retry creates exactly one Routine and returns the same compatibility
  mapping.
- A managed run exposes exactly one schedule backend and one `cron` tool definition.
- Run success cannot bypass verification/review/handoff policy.

## CP-09 — Workspace Resolution, Isolation, and Delivery

### Goal

Preserve ALTAI's stronger worktree/delivery behavior under control-plane ownership.

### Work

1. Persist ProjectWorkspace and ExecutionWorkspace identities.
2. Resolve workspace mode: primary, new isolated, reuse existing.
3. Record preparation/finalization/cleanup operations.
4. Prevent dependent work from waking before upstream workspace finalization.
5. Keep one worktree per isolated attempt unless an explicit reuse policy exists.
6. Preserve explicit Apply and Draft PR delivery actions.
7. Attach diffs, tests, screenshots, commits, and PRs as artifacts/work products.
8. Track commit and artifact lineage across child integration.

### Exit gate

- Parallel agents never accidentally share a mutable workspace.
- Delivery remains explicit and recoverable.
- A dirty/conflicted target never receives a partial apply.

## CP-10 — Approvals and Governance

### Goal

Separate runtime permissions from business/control-plane approvals.

### Approval classes

- tool/edit/shell approval: enforced by IsanAgent during one run;
- plan approval: controls work decomposition or implementation continuation;
- delivery approval: controls apply/publish/merge;
- agent configuration/hire approval: controls new durable agent authority;
- budget override approval;
- plugin capability expansion approval;
- managed policy exception approval.

### Work

1. Persist approval payload hash, scope, requester, policy source, and decision.
2. Link approvals to work, attempt, agent, project, or plugin.
3. Wake the exact responsible agent/work item after resolution.
4. Revision config changes and support rollback.
5. Never expose secret values inside approval payloads.

### Exit gate

- Approving one action cannot authorize a changed payload.
- Expired/stale approvals fail closed.
- Every high-impact action has an actor-attributed audit event.

## CP-11 — Usage, Cost Ledger, and Budgets

### Goal

Add portfolio-level cost control without losing per-attempt safeguards.

### Work

1. Normalize provider usage into append-only CostEvents.
2. Attribute cost to organization, project, goal, agent, work item, attempt, provider,
   model, and biller where known.
3. Keep estimated and confirmed cost status separate.
4. Add monthly and lifetime BudgetPolicies.
5. Support organization, project, agent, and work-item scopes.
6. Enforce hard stop before dispatch and during run usage updates.
7. Create budget incidents and explicit resolution/override flow.

### Exit gate

- Dispatch and budget reservation are atomic.
- One agent cannot exceed its hard stop because another client had stale budget data.
- UI totals reconcile from CostEvents, not mutable counters alone.

## CP-12 — Routines and Scheduler

### Goal

Replace chat-owned automations with versioned, auditable project work routines.

### Routine model

- immutable revision snapshot;
- task template and target project/goal/parent;
- assignee agent;
- variables and environment references;
- triggers: once, cron, webhook, event, manual;
- concurrency: allow, skip-if-active, coalesce-if-active;
- catch-up: skip-missed, run-once, bounded replay;
- activity gate and quiet hours;
- run history and linked work item.

### Work

1. Scheduler claims due triggers from the control DB.
2. Each trigger creates or reuses a RoutineRun with an idempotency fingerprint.
3. RoutineRun creates a WorkItem, then a WakeRequest.
4. Routine revisions are immutable; edits create a new revision.
5. Webhook triggers include signature verification and replay protection.
6. Manual run uses the same path as scheduled execution.
7. Implement `RoutineCommandPort` for trusted UI/API clients and the managed IsanAgent
   cron bridge.
8. Store command origin as human, API/plugin, imported legacy cron, or agent tool call with
   attempt/run/tool-call causation.
9. Apply capability, approval, interval, scope, concurrency, and budget policy before a
   bridge-created Routine becomes enabled.
10. Expose a deterministic preview containing normalized trigger, timezone, next fires,
    target Work/Wake template, and required approval.

### Exit gate

- Replaying a trigger cannot create duplicate active work.
- Missed schedules follow explicit catch-up policy.
- Routine history remains understandable after the definition changes.
- A Routine created through the `cron` bridge is indistinguishable in lifecycle quality
  from one created through UI/CLI, while its agent/tool origin remains auditable.

## CP-13 — Liveness, Monitors, and Recovery

### Goal

Ensure every non-terminal agent-owned task has a durable next action.

### Valid liveness paths

- active attempt/run;
- queued wake;
- unresolved blocker with healthy upstream work;
- pending approval/interaction with a responsible actor;
- durable one-shot monitor with bounded attempts;
- assigned human owner;
- explicit RecoveryAction.

### Work

1. Evaluate liveness at attempt finalization and periodic recovery sweeps.
2. Detect orphaned runs, expired leases, stale checkouts, missing disposition, and invalid
   background waits.
3. Add bounded normal-model continuation before escalation.
4. Create RecoveryActions with owner, evidence, proposed repair, and resolution.
5. Never infer completion from prose comments.
6. Make recovery idempotent by source-state fingerprint.

### Exit gate

- The system can answer "what moves this forward?" for every non-terminal work item.
- Identical stale evidence cannot generate an infinite recovery loop.
- Process loss does not produce duplicate child work or duplicate delivery.

## CP-14 — External Objects and GitHub Adapter

### Goal

Keep ALTAI's GitHub strengths while making GitHub an optional normalized integration.

### Work

1. Add ExternalObjectLink and plugin/adapter-owned sync state.
2. Implement GitHub issue, PR, and Project mappings through the TaskSource contract.
3. Add per-project source-of-truth policy: ALTAI, GitHub, or explicit field-level mirror.
4. Add webhook ingestion for daemon mode and bounded polling fallback for local mode.
5. Add idempotent comment/status sync with loop prevention.
6. Preserve device-flow auth for local personal use.
7. Add GitHub App installation auth only when multi-user/hosted operation requires it.
8. Keep tokens behind Rust and provide agents scoped broker operations, never raw tokens.
9. Preserve explicit draft PR creation and delivery lineage.

### Exit gate

- GitHub can be disconnected without disabling local project management.
- Offline reconciliation does not duplicate comments, PRs, or status transitions.
- Sync conflicts are surfaced; last-writer-wins is never silently assumed.

## CP-15 — Public Control-Plane Protocol

### Goal

Expose one capability-negotiated API to every ALTAI surface.

### Method groups

```text
organizations/*
goals/*
projects/*
agents/*
work/*
attempts/*
routines/*
approvals/*
budgets/*
activity/*
integrations/*
plugins/*
```

### Work

1. Reuse JSON-RPC framing where appropriate; keep transport separate from methods.
2. Add cursor pagination and bounded filters.
3. Add event subscriptions with sequence cursors and replay.
4. Return capability availability per host and deployment mode.
5. Use typed error codes, never English string branching.
6. Add actor/session authentication to all mutations.

### Exit gate

- No Desktop-only orchestration capability remains in the product contract.
- IDE and Desktop receive identical work entities and events.
- Sequence gaps are repaired through replay.

## CP-16 — Control-Plane Daemon and Lifecycle

### Goal

Provide one durable local coordinator for all clients.

### Work

1. Add `altai-daemon` binary and supervisor commands.
2. Implement user-scoped single-instance lock.
3. Add local socket/named-pipe transport and per-install authentication secret.
4. Add lazy start, health check, version negotiation, graceful shutdown, and bounded restart
   backoff.
5. Add opt-in launch-at-login for scheduled background operation.
6. Make "UI closed execution" an explicit setting and visible status.
7. Never expose a network listener by default.

### Exit gate

- Desktop, IDE, and CLI attach to one coordinator simultaneously.
- Closing a renderer does not stop active control-plane work.
- Disabling background operation produces a clear paused state, not silent schedule loss.

## CP-17 — Read Models and Product UI

### Goal

Make every UI a projection of canonical control-plane state and provide one coherent
operating model across Desktop, IDE, Studio, CLI, and plugins.

### Product rule

The control plane is not presented as twenty unrelated administration pages. Desktop keeps
the editor-centric shell; its existing **Project Management** destination evolves into an
**Operations** workspace with stable secondary navigation. Studio exposes the same objects
as a wider management surface. IDE keeps a compact project lens, work queue, inbox, and run
inspector. All of them deep-link to the same canonical IDs.

The complete information architecture, screen contracts, workflows, and current-screen
migration are defined in Section 9.

### Work

1. Replace frontend joins across assignments, background jobs, automations, notifications,
   and runs with server-created read models.
2. Keep Work and Inbox semantics from the Unified Agent Work Surface plan.
3. Bind every detail view to explicit IDs, not the globally active chat.
4. Use optimistic UI only with revision-aware rollback.
5. Preserve keyboard and screen-reader requirements.

### Delivery slices

| Slice | Screens | Backend prerequisite |
| --- | --- | --- |
| CP-17A | Operations shell, context switcher, daemon health | CP-04, CP-15, CP-16 |
| CP-17B | Dashboard, My Work, Inbox | CP-03, CP-07, CP-10, CP-13 |
| CP-17C | Work board/list, Work detail, task graph | CP-03, CP-06, CP-07 |
| CP-17D | Run detail, transcript, evidence, delivery | CP-08 through CP-10 |
| CP-17E | Goals, projects, workspaces, agents/org chart | CP-04, CP-05 |
| CP-17F | Routines, routine runs, recovery state | CP-12, CP-13 |
| CP-17G | Approvals, budgets/costs, activity/audit | CP-10, CP-11 |
| CP-17H | Integrations and external sync health | CP-14 |

Each slice must ship its query projection, command handlers, event-driven invalidation,
loading/empty/error/offline states, permission matrix, keyboard behavior, and UI tests in
the same release. A screen backed by placeholder local state does not count as delivered.

### Existing code changes

- `assignmentsStore`: compatibility/cache only, then remove persistence.
- `automationStore`: Routine projection only.
- orchestration React controller: subscription and commands only.
- GitHub Overview: renders normalized work/external links instead of owning status.

### Exit gate

- Refreshing or closing a UI cannot alter scheduling correctness.
- All badge/count selectors derive from one read-model stream.
- Work remains accessible even when its session is archived.

## CP-18 — Desktop, IDE, Studio, and CLI Adapters

### Goal

Use one shared service without flattening host-specific capabilities.

### Work

1. Desktop adapter: Tauri navigation, local editor, terminal, diff, and native Git UI.
2. IDE adapter: editor selection, workspace trust, diff/open-file, extension-host daemon
   client.
3. Studio adapter: project/agent/routine management and read-only or approved execution
   controls.
4. CLI adapter: machine-readable CRUD, wake, observe, replay, and administration commands.
5. Capability-gate host-only actions while keeping domain objects identical.

### Exit gate

- A work item created in any host is immediately visible in every attached host.
- Host-specific UI actions do not create host-specific task identities.
- Untrusted IDE workspaces cannot dispatch or mutate workspace-scoped work.

## CP-19 — Application Plugin Runtime

### Goal

Add a second plugin tier without confusing it with agent content bundles.

### Plugin classes

1. `AgentContentPlugin`: skills, agent profiles, commands, hooks, MCP definitions.
2. `ApplicationPlugin`: connector/automation/UI/tool extensions running out of process.

### Work

1. Define versioned manifest and capability vocabulary.
2. Run application plugin workers out of process with JSON-RPC.
3. Add bounded lifecycle, restart, logs, health, jobs, webhooks, and plugin-owned state.
4. Require idempotent event/webhook/job handlers.
5. Add scoped secret references; workers receive a value only for an authorized operation.
6. Start with schema-driven pages/actions.
7. Use sandboxed UI for untrusted custom UI; same-origin ESM is allowed only for explicitly
   trusted/signed plugins.
8. Require approval for capability expansion on upgrade.

### Initial capabilities

```text
work.read / work.write
comments.write
runs.trigger
routines.manage
agents.read
projects.read
github.read / github.write
http.outbound:<allowlisted-domain>
secrets.use-ref
workspace.read:<scope> / workspace.write:<scope>
agent.tools.register
ui.slot:<slot>
activity.write
```

### Exit gate

- A crashing plugin cannot stop the coordinator.
- A plugin cannot call undeclared host services.
- Plugin UI cannot bypass worker capability checks or agent workspace permissions.

## CP-20 — Legacy Migration and Cutover

### Sources to import

- `altai-assignments.json`;
- existing local todo/project board state;
- current orchestration ledger tasks/attempts/approvals/artifacts;
- IsanAgent automation/cron definitions;
- background job and notification references;
- existing agent profiles;
- GitHub item/assignment mappings;
- session/run journal locators.

### Work

1. Generate deterministic import fingerprints.
2. Import into one default local organization and inferred projects.
3. Preserve original IDs in compatibility mapping tables.
4. Run read-only shadow comparisons before enabling control-plane claim.
5. Cut over one workspace at a time.
6. Disable legacy scheduler ownership before enabling the new writer.
7. Keep rollback capable of reading new projections without restarting the old scheduler.
8. Remove dual-write after one stable release; permanent dual-write is forbidden.
9. Persist legacy cron job ID to Routine ID mappings and keep remove/list compatibility
   until the alias window expires.
10. Select one `ScheduleBackendMode` per migrated workspace and reject startup if both
    native ALTAI cron and managed Routines are configured as writers.

### Exit gate

- Re-running migration creates no duplicates.
- Active work and sessions remain reachable.
- There is exactly one scheduler before, during, and after cutover.

## CP-21 — Verification, Soak, and Release Gates

### Required tests

1. State-machine transition and property tests.
2. Parent/dependency cycle and liveness property tests.
3. 1,000 work items, eight workers, zero duplicate active attempts.
4. Crash after every durable transition.
5. Two clients concurrently assigning/editing the same task.
6. Budget reservation race and mid-run hard stop.
7. Routine trigger replay, missed schedules, and coalescing.
8. UI/IDE shutdown while daemon continues work.
9. GitHub offline hour, reconnect, and conflict reconciliation.
10. Plugin worker crash, restart storm, capability denial, and secret redaction.
11. Worktree prepare/finalize/apply crash recovery.
12. Migration replay from every supported prior schema/store version.
13. IsanAgent standalone Local and MultiTenantEdge cron regression suites.
14. Managed cron bridge add/remove/list, permission denial, approval, timezone, idempotent
    retry, and legacy-ID mapping.
15. Runtime tool catalog asserts one `cron` definition and one selected schedule backend.

### Release invariants

- duplicate active attempt rate: zero;
- unowned non-terminal work: zero;
- required verification bypass: zero;
- cross-organization access: zero;
- raw secret in events/logs/UI: zero;
- renderer-required scheduling decision: zero;
- automatic protected-branch merge: zero by default.

## 9. Product UX, Screens, and Operating Workflows

This section is part of the engineering contract, not a later visual-design exercise. It
defines how a person supervises the control plane and prevents the frontend from recreating
task, scheduler, or approval ownership.

### 9.1 Experience principles

1. **Outcome first:** the primary object is a WorkItem linked to a project and goal, not a
   chat or model invocation.
2. **Attention over activity:** the default view prioritizes decisions, blockers, failed
   recovery, budget stops, and review rather than streaming every run.
3. **Progressive disclosure:** board cards show work state; Work detail shows coordination;
   Run detail shows execution internals.
4. **One object, many lenses:** board, list, inbox, run inspector, IDE, and CLI share the
   same IDs and revisions.
5. **Explain every automatic action:** every dispatch, retry, pause, wake, and budget stop
   links to the policy and event that caused it.
6. **Safe mutation:** commands show scope and consequence; irreversible or broad actions
   require confirmation and, where configured, an Approval.
7. **Local-first degradation:** GitHub or a plugin being offline never disables local work.
8. **No renderer ownership:** leaving a page, closing a window, or switching chat cannot
   change orchestration behavior.

### 9.2 Shell and information architecture

#### Desktop

Keep the existing editor-centric shell and three primary rail concepts rather than adding
an icon for every entity:

```text
Files                      existing workspace/editor tools
GitHub                     source control and external collaboration
Operations                 ALTAI control-plane workspace
  Overview                 dashboard and attention summary
  Work                     board/list/graph and Work detail
  Inbox                    approvals, questions, review, failures
  Runs                     live/recent execution and Run detail
  Organization             goals, projects, agents, workspaces
  Routines                 schedules and event-triggered work
  Governance               approvals, budgets, activity
```

`Operations` replaces the current semantic role of `Project Management`; it does not remove
the current board during migration. A compatibility flag switches its data source and
secondary navigation after CP-17B/CP-17C are ready.

The persistent Operations header contains:

- organization, project, and workspace context;
- daemon/background-operation health;
- global attention count;
- command/search entry;
- create WorkItem action.

#### IDE extension

Expose a compact subset optimized for code context:

- Project Lens: current project health, goal, active work, and workspace status;
- My Work: assigned/active/reviewable WorkItems;
- Inbox: approvals, questions, and review requests;
- Run Inspector: attempt timeline, tools, diff, evidence, steer/cancel;
- context actions: create work from selection, attach selection, open work/run in Desktop or
  Studio.

The IDE must not implement an independent board or scheduler store.

#### Studio

Studio is the widest control-plane surface. It uses the same screen contracts and adds
cross-project views, organization design, agent/profile administration, routine management,
budgets, audit, and integration health. It does not use Studio-only domain objects.

#### CLI

Every screen mutation has a command equivalent and every important detail projection has a
JSON form. Interactive TUI views may compose these APIs but do not receive privileged state
access.

### 9.3 Shared screen contract

Every screen follows the same data path:

```text
route with canonical ID
  -> query projection + projection revision
  -> render confirmed state
  -> command(expected_revision, idempotency_key, actor, payload)
  -> accepted/rejected receipt
  -> subscribed domain/projection event
  -> reconcile confirmed state
```

Rules:

- URL/route state may select, filter, sort, and lay out data; it may not own domain state.
- Optimistic changes are visibly pending and roll back on revision conflict.
- A rejection returns current state, reason, allowed actions, and correlation ID.
- Counts, badges, and lists must use a common projection checkpoint to avoid contradictions.
- Permissions are returned as allowed command descriptors; UI role-name checks are only a
  display optimization and never authorization.
- Every screen defines loading, empty, disconnected, stale, forbidden, and partial-source
  failure states.
- Destructive bulk commands preview their resolved target IDs before confirmation.

### 9.4 Screen inventory and data contracts

| Screen | Primary purpose | Required projection/query | Primary commands | Depends on |
| --- | --- | --- | --- | --- |
| Operations Overview | Answer “what needs attention and what is progressing?” | `OperationsSummary` | pause/resume project, open/create work | CP-04, CP-07, CP-10 through CP-13 |
| My Work | Personal/agent-scoped queue across projects | `WorkQueue(actor, filters)` | claim, unclaim, reassign, wake | CP-05, CP-07 |
| Inbox | Unified actionable communication | `InboxFeed(actor)` | approve/deny, answer, acknowledge, open target | CP-10, CP-13 |
| Work Board/List | Plan and monitor durable work | `WorkCollection(project, view)` | create, update, transition, assign, bulk command | CP-03, CP-06, CP-07 |
| Work Graph | Understand decomposition, blockers, and critical path | `WorkGraph(project/root)` | add/remove dependency, reparent, propose decomposition | CP-06 |
| Work Detail | Coordinate one outcome | `WorkDetail(work_item_id)` | edit, comment, assign, wake, transition, approve plan | CP-03, CP-06, CP-07, CP-10 |
| Run Detail | Inspect one execution attempt | `RunDetail(run_id)` plus run event cursor | steer, provide input, approve tool, cancel, retry | CP-08, CP-10 |
| Review & Delivery | Verify artifacts and move work to handoff | `DeliveryReview(work_item_id)` | rerun check, request changes, apply, publish draft PR, accept | CP-09, CP-10, CP-14 |
| Goals | Define outcomes and inspect progress lineage | `GoalTree(organization_id)` | create/update/archive goal | CP-04 |
| Projects | Administer project policy and linked workspaces | `ProjectDetail(project_id)` | create/update/archive, attach workspace/repository | CP-04, CP-09 |
| Workspaces | Show isolation, branch/worktree, health, and cleanup | `WorkspaceInventory(project_id)` | prepare, repair, open, finalize, cleanup | CP-09 |
| Agents & Org Chart | Manage durable worker identities and reporting | `AgentOrganization(organization_id)` | create/pause/resume agent, change manager/profile | CP-05 |
| Agent Detail/Profile | Explain capabilities, configuration revision, work, and cost | `AgentDetail(agent_instance_id)` | revise profile, assign project, set budget, wake | CP-05, CP-07, CP-11 |
| Routines | Define recurring/event-triggered work | `RoutineCollection(project/org)` | create, revise, enable/disable, run now | CP-12 |
| Routine Detail | Explain schedule, revision, next fire, and history | `RoutineDetail(routine_id)` | revise, backfill, skip, retry routine run | CP-12, CP-13 |
| Approvals | Review governance decisions independent of chat | `ApprovalQueue(actor/scope)` | approve, deny, revoke where valid | CP-10 |
| Budgets & Costs | Show reservations, actual usage, limits, and forecasts | `BudgetSummary(scope, period)` | set policy, pause scope, export | CP-11 |
| Activity & Audit | Explain who/what changed state and why | `ActivityFeed(scope, cursor)` | filter, correlate, export redacted trail | CP-02, CP-15 |
| Integrations | Configure adapters and resolve sync health/conflicts | `IntegrationStatus(project/org)` | connect, disconnect, map, retry, resolve conflict | CP-14, CP-19 |
| Settings & Health | Configure daemon/background mode and inspect diagnostics | `ControlPlaneHealth` | start/restart daemon, toggle background mode, export support bundle | CP-16 |

Screen queries are dedicated read models; clients must not reconstruct them by joining raw
entity lists. Commands use the domain vocabulary from Sections 4 and 5.

### 9.5 Work card, Work detail, and Run detail boundaries

These three levels must remain distinct:

| Level | Shows | Must not show as authoritative |
| --- | --- | --- |
| Work card | title, work status, priority, assignee, blocker/attention badges, current attempt summary | transcript-derived status or session as owner |
| Work detail | outcome/criteria, goal/project, parent/children, dependencies, comments, attempts, approvals, artifacts | raw tool stream as project history |
| Run detail | immutable attempt spec, model/profile revision, timeline, plan items, subagent runs, tools, transcript, usage | project status edits that bypass Work commands |

From Work detail a user can open a Run; from Run detail a user can return to its WorkItem.
Archiving a chat/session cannot remove either record.

### 9.6 Primary operating workflows

#### WF-01 — First-run organization and project setup

```text
choose/create organization
  -> create project and goal
  -> attach local repository/workspace
  -> select project instructions and delivery defaults
  -> create/select agent instances
  -> readiness validation
  -> Operations Overview
```

The setup wizard writes through normal commands; it is not a separate configuration store.
It can be skipped by importing the current workspace into a default local organization.
Partial setup is resumable.

#### WF-02 — Create, assign, and execute work

```text
Create WorkItem
  -> define outcome + acceptance criteria + project/goal
  -> optional parent/dependencies/budget/workspace policy
  -> assign AgentInstance
  -> durable WakeRequest
  -> coordinator eligibility decision
  -> checkout + Attempt
  -> IsanAgent Run
  -> verification/review/delivery
  -> Done or actionable next state
```

The create dialog may offer templates and advanced fields, but it must create a WorkItem
before a run. “Run now” is assignment plus wake, not direct session creation.

#### WF-03 — Plan and decompose into durable child work

```text
planner attempt
  -> proposed task graph artifact
  -> plan approval (version-specific)
  -> atomic child WorkItem + dependency creation
  -> independent assignment/wake per eligible child
  -> parent projection aggregates progress
```

IsanAgent `todo_write` remains visible only inside Run detail. Promotion to durable work is
an explicit, reviewable control-plane command.

#### WF-04 — Agent delegation and manager handoff

```text
agent requests durable delegation
  -> capability/budget/policy evaluation
  -> child WorkItem assigned to target AgentInstance
  -> manager/parent receives durable update
  -> child result and artifacts roll up
```

A run-internal subagent skips this flow and stays inside one attempt. The UI labels these as
“subagent runs,” never as assigned project work.

#### WF-05 — Approval, input, and steering

```text
run or policy raises Approval/InputRequest
  -> Inbox item + OS/host notification
  -> user opens scoped context and impact preview
  -> approve/deny/answer/steer with expected revision
  -> event wakes or resumes the correct attempt/work item
```

Tool permission, plan approval, business approval, and delivery approval use distinct types
and copy. A decision made from IDE, Desktop, Studio, or CLI resolves the same Approval ID.

#### WF-06 — Dependency blocked and automatic wake

```text
dependency edge added or upstream becomes non-terminal
  -> dependent WorkItem blocked with explainable reason
  -> upstream reaches qualifying terminal state
  -> eligibility recomputed transactionally
  -> coalesced WakeRequest
  -> dispatch or explicit remaining blocker
```

Work detail and graph show both the blocking path and the policy that determines whether a
terminal upstream result qualifies.

#### WF-07 — Review and safe delivery

```text
run succeeds
  -> required checks + evidence
  -> optional automated reviewer
  -> WorkItem enters in_review
  -> human reviews diff/findings/artifacts
  -> request changes OR apply locally OR publish draft PR
  -> post-delivery verification
  -> done
```

A successful model turn never jumps directly to `done`. The Review screen shows evidence
revision, target workspace, conflicts, and the exact delivery action.

#### WF-08 — Create and operate a routine

```text
create versioned Routine
  -> choose trigger, timezone, target project, agent, work template, budget
  -> preview next fires and policy
  -> enable
  -> daemon materializes RoutineRun + WorkItem/WakeRequest
  -> run history, skip/backfill/retry, next fire
```

Disabling a routine stops future materialization but does not silently cancel active work.
The existing Automations UI becomes this flow after cron import.

#### WF-09 — Failure, retry, and crash recovery

```text
heartbeat loss/process crash/restart
  -> lease and run reconciliation
  -> classify resumable, retryable, lost, or needs attention
  -> automatic bounded recovery OR Inbox decision
  -> preserve attempt/run lineage
```

The UI shows the last durable event, retry policy, next retry time, and why recovery did or
did not occur. “Retry” creates a new Attempt; it never rewrites the previous one.

#### WF-10 — Budget warning and hard stop

```text
usage/reservation event
  -> scope ledger recomputed
  -> warning threshold notification
  -> hard limit blocks new checkout and requests active-run stop per policy
  -> owner adjusts budget or keeps scope paused
  -> explicit resume/wake
```

Budget screens separate estimates, reservations, actual cost, and unavailable pricing.

#### WF-11 — GitHub synchronization and conflict resolution

```text
connect GitHub + choose mappings
  -> import/link ExternalObjects
  -> outbox/inbox synchronization
  -> local WorkItem remains authoritative
  -> remote conflict becomes Inbox item
  -> user/policy resolves with recorded provenance
```

Opening or closing a GitHub issue must not implicitly bypass ALTAI review, verification, or
dependency rules.

#### WF-12 — Cross-surface continuation

```text
create work in IDE
  -> observe/approve in Desktop
  -> inspect organization in Studio
  -> query/retry from CLI
```

Every hop uses a deep link containing canonical object IDs. A host that lacks a capability
offers “Open in Desktop/Studio” rather than emulating the command locally.

### 9.7 Attention and notification model

Inbox is an actionable projection, not a new message database. Items are derived from:

- pending approvals;
- input/clarification requests;
- review and delivery requests;
- failed or exhausted recovery;
- dependency or policy blockers needing human action;
- budget warnings/stops;
- external synchronization conflicts;
- routine failures requiring intervention.

`Activity` is the complete audit history; `Inbox` is the unresolved-action subset;
`Dashboard` is an aggregate summary. Acknowledging an informational notification does not
resolve its underlying Approval or blocker.

### 9.8 Current UI migration map

Disposition meanings:

- **Keep:** retain the component's product responsibility and rebind its data.
- **Move:** retain the capability but move it to the surface that owns the concept.
- **Merge:** consolidate duplicate surfaces into one shared component/route.
- **Replace:** preserve the user outcome but rebuild against the control plane.
- **Remove:** delete after compatibility and migration gates pass.

| Current implementation | Decision | Target / migration action |
| --- | --- | --- |
| `Project Management` rail destination | Rename + expand | Becomes `Operations`; secondary navigation owns Overview, Work, Inbox, Runs, Organization, Routines, Governance |
| `ProjectManagementSidebar` | Replace | Compact `OperationsSummary` lens; no assignment/session joins or lifecycle mutations |
| `CommandCenter` | Replace | Operations Overview using server projections; remove frontend aggregation across todo, assignment, run, and orchestration stores |
| `ProjectBoardPanel` / `ProjectBoardStack` | Replace + rename | `WorkCollection` route/stack with board, list, graph, and Work detail |
| local todo board state | Migrate + remove | Import eligible todos as WorkItems; keep IsanAgent todo data only as RunPlanItems |
| `NewWorkComposer` | Replace | WorkItem create flow; `Run now` becomes create + assign + wake |
| `AssignmentsRail` | Remove | My Work/Run projections replace it; GitHub surface no longer hosts agent operations |
| `TaskRunsPanel` | Move + replace | Canonical Operations `Runs` view; task creation is removed from this screen |
| `WorkHubPanel` | Remove | Work and Routines are canonical Operations routes, not an AI chat overlay |
| `AutomationsPanel` | Move + replace | Canonical `Routines` screen after cron import |
| `automationStore` | Remove | Replaced by Routine query cache/subscription client with no scheduling logic |
| `ProjectIntelligencePanel` | Move + keep capability | Project Settings/Instructions bound to Project and ProjectWorkspace revisions |
| `OrchestrationControlCenter` / `OrchestrationBar` | Remove + redistribute | Project pause/resume goes to Operations header; concurrency/retry policy goes to Project policy; routine triggers go to Routines |
| `OrchestrationController` | Remove | All claim, retry, reconciliation, and recovery run in `altai-control-plane` |
| orchestration frontend store | Remove | Replace with coordinator health and project-operation projections |
| orchestration `WorkflowEditor` | Replace | Versioned Project Policy editor; never directly starts a renderer scheduler |
| duplicate orchestration and AI run inspectors | Merge | One shared Run Detail/compact Run Inspector backed by `run_id` and event cursor |
| assignment attention badges | Replace | Inbox/Operations projection counts from one checkpoint |
| notification/background-job/ticket joins | Migrate + remove | Canonical Inbox projection derived from approvals, input, blockers, recovery, budgets, and sync conflicts |
| `notificationStore` lifecycle mutations | Remove | Inbox commands resolve underlying domain objects; read/unread preference may remain client-local |
| `todoStore` used for project queues | Remove use | Rename/restrict to RunPlan projection; it must never feed Work board eligibility |
| `agentRunsStore` as task status source | Narrow | Keep a run-event projection cache only; Work status comes from control plane |
| chat mini/run inspector | Keep + rebind | Bind by `run_id`; session remains transcript context, not project identity |
| `TodoSummaryChip` in AI topbar | Rename + narrow | `RunPlanSummaryChip`; explicitly labels run-internal checklist |
| AI `AgentsInspector` | Rename + narrow | `SubagentRunsInspector`; durable delegated work appears through linked child WorkItems |
| GitHub Overview cards | Replace semantics | Render linked ExternalObjects and sync state; never own ALTAI Work status |
| `AssignAgentButton` on issue/PR | Replace action | `Create/link work`; optional quick assignment invokes control-plane create/assign/wake commands |
| GitHub issue/PR list/detail/commenting | Keep | Remains an integration/source-collaboration surface |
| local changes, diff, stage, commit, push | Keep | Remains in GitHub/source-control surface; delivery commands reference these capabilities |
| `githubStore` remote cache | Narrow | Integration cache only; no canonical Work lifecycle fields |
| slash commands `/tasks`, `/automations`, `/inbox` | Redirect + deprecate aliases | `/work`, `/routines`, `/inbox` deep-link canonical Operations routes; keep old aliases for one stable release |
| `work.taskRuns`, `work.automations`, `inbox.notifications` host capabilities | Replace | Versioned control-plane query/command capabilities; no desktop-only ownership |

During migration, legacy and control-plane views must be visually marked and cannot both
mutate the same workspace. Read-only comparison is allowed behind developer diagnostics.

### 9.9 AI chat sidebar after consolidation

AI chat remains a first-class IsanAgent interaction surface, but it is not the project
management navigation shell.

#### Keep in AI chat

- new chat, open chat tabs, and chat history;
- active transcript and composer;
- model/tool/permission controls for an ad-hoc chat;
- compact Run Inspector for the active `run_id`;
- inline tool permission and input prompts for the active run;
- run-internal plan/todo and subagent-run summaries;
- a linked WorkItem context chip and `Open in Operations` action when managed work exists;
- change/evidence notification that deep-links to canonical Review & Delivery.

#### Remove from AI chat

- the Work overlay and its Runs/Scheduled tabs;
- project task creation/listing as a chat-owned feature;
- the standalone notification/background-job Inbox overlay;
- routine/cron authoring;
- project queue, agent assignment, retry policy, and delivery ownership;
- any task status derived from the currently selected session.

#### Replace with shortcuts

The AI topbar has at most two control-plane shortcuts:

1. the linked WorkItem chip, if the active run belongs to managed work;
2. an attention badge that opens the canonical Operations Inbox.

In narrow IDE mode these open the Operations route or host-provided compact view. They do
not mount a second copy of Work/Inbox state inside `AiSidePanel`.

Ad-hoc chats remain valid. They create sessions/runs without becoming project work. A
separate **Promote to Work** command creates a WorkItem with explicit project, outcome,
criteria, ownership, and selected conversation reference. A managed attempt has an immutable
workspace/profile specification; the chat target picker cannot silently change it.

### 9.10 GitHub and Project Management boundary after consolidation

```text
Operations owns                 GitHub/source control owns
-----------------------------   --------------------------------
WorkItem lifecycle              issues, pull requests, comments
assignment / wake / attempt     remote repository collaboration
dependency and hierarchy        local diff/stage/commit/push
approval / verification         branch and remote status
review policy / done decision   ExternalObject content and sync
```

An issue or pull request can be linked to zero or one canonical WorkItem per configured
mapping policy. “Create issue and assign” becomes an observable saga:

1. create the GitHub object;
2. record its ExternalObject identity;
3. create/link the WorkItem;
4. assign and wake through the control plane;
5. show a resumable partial-failure state if a later step fails.

The old combined Overview board is retired. Operations Work can filter by GitHub linkage,
while GitHub lists can show linked Work status as a read-only badge and deep link. Dragging
a GitHub card never performs an implicit ALTAI transition or closes a remote item unless the
configured sync policy produces an explicit, audited command.

### 9.11 Surface cutover and deletion sequence

1. **Inventory freeze:** record every route, menu, slash command, store mutation, event,
   badge, and deep link; add architecture tests preventing new legacy imports.
2. **Shared routing:** add canonical Operations routes and old-to-new deep-link aliases,
   initially read-only.
3. **Projection shadowing:** compare legacy UI aggregates with control-plane projections in
   developer diagnostics; only the legacy scheduler writes.
4. **Workspace cutover:** enable new Work/Inbox/Run/Routine commands for one workspace;
   disable legacy mutations before enabling control-plane mutations.
5. **Menu consolidation:** rename Project Management to Operations; replace AI Work/Inbox
   overlays with deep links; remove AssignmentsRail from GitHub.
6. **Store retirement:** stop persistence/refresh listeners for assignments, automations,
   notifications, and orchestration after import and reachability checks.
7. **Compatibility release:** keep old route/slash-command aliases and read-only migration
   diagnostics for one stable release.
8. **Physical deletion:** delete legacy components, stores, Tauri commands, event bridges,
   tests, and documentation only after telemetry/tests show no caller and migration is
   idempotent.

No phase permits two authoritative mutation paths. Rollback means selecting the previous
single writer before a workspace is migrated, not re-enabling a legacy writer against new
state.

### 9.12 UX delivery modules

UX work starts as contract/wireflow design alongside backend modules, but production
mutation is enabled only after the listed gates.

| UX module | Deliverable | Engineering gate |
| --- | --- | --- |
| UX-00 | Route map, canonical deep-link grammar, terminology, role/permission matrix | CP-00, CP-01 |
| UX-01 | Operations shell, context switcher, responsive layouts, health states | CP-04, CP-15, CP-16 |
| UX-02 | Work board/list/detail/graph and create flow | CP-03, CP-06, CP-07 |
| UX-03 | Run detail, timeline, transcript, approvals, evidence and delivery | CP-08 through CP-10 |
| UX-04 | Dashboard, My Work, Inbox and notifications | CP-07, CP-10, CP-13 |
| UX-05 | Goals, projects, workspaces, agents and org chart | CP-04, CP-05, CP-09 |
| UX-06 | Routines and recovery operations | CP-12, CP-13 |
| UX-07 | Budgets, activity/audit and integration health | CP-11, CP-14 |
| UX-08 | IDE/Studio/CLI parity and cross-host deep links | CP-15, CP-18 |
| UX-09 | Plugin slots, capability disclosures and failure isolation UI | CP-19 |

Each module produces, in order:

1. route and object-context specification;
2. happy-path plus failure/recovery wireflows;
3. query/command/event contract fixture;
4. interactive implementation using shared components;
5. keyboard, screen-reader, reduced-motion, and narrow-width verification;
6. screenshots or recordings for the acceptance scenario;
7. end-to-end tests with a deterministic mock control plane, then the real daemon.

### 9.13 UI acceptance gates

- A user can always answer: what outcome is being pursued, who owns it, what is blocking
  it, what ran, what it cost, and what happens next.
- Work status, execution phase, and attention reason are never collapsed into one badge.
- No UI action mutates a raw store or starts IsanAgent without a control-plane command.
- Refresh, host switch, offline external service, and renderer restart preserve object
  identity and confirmed state.
- Concurrent edits receive revision-conflict recovery rather than last-write-wins data loss.
- Every automated transition is explainable from the visible activity trail.
- All primary workflows are keyboard-complete and screen-reader labeled.
- Destructive/bulk actions show scope; approval decisions show policy and payload revision.
- Desktop, IDE, Studio, and CLI conformance tests prove the same command produces the same
  state transition.

## 10. Delivery Sequence and PR Sizing

### Foundation release

CP-00 through CP-03.

Expected: 7-10 focused PRs, including UX-00 route/surface ownership design.

Outcome: canonical WorkItem and control DB exist, but the legacy scheduler remains the
single writer.

### Control-plane preview

CP-04 through CP-09.

Expected: 13-18 focused PRs, including the first Operations shell, Work, Run, and
organization slices.

Outcome: goals/projects/agents, atomic wakes/checkouts, and IsanAgent attempt execution work
end to end for opt-in workspaces.

### Governance and autonomy preview

CP-10 through CP-13.

Expected: 10-14 focused PRs, including canonical Inbox, Routines, approvals, and budgets.

Outcome: approvals, budgets, routines, and liveness/recovery support reliable autonomous
operation.

### Multi-surface preview

CP-15 through CP-18.

Expected: 10-15 focused PRs, including AI sidebar/GitHub/Project Management consolidation
and cross-host conformance.

Outcome: Desktop, IDE, Studio, and CLI use one daemon and one work model.

### Ecosystem and cutover

CP-14 and CP-19 through CP-21.

Expected: 11-16 focused PRs, including legacy physical-deletion batches.

Outcome: normalized GitHub integration, application plugins, migration completion, and
production gates.

Overall expected size: approximately 51-73 focused PRs. The range is intentionally larger
than the earlier operations estimate because this plan corrects process ownership,
multi-surface lifecycle, information architecture, existing UI consolidation, and plugin
boundaries rather than only adding UI and scheduler features.

## 11. First Twelve PRs

1. ADR: control plane versus execution plane; amend ADR 0001/0002 scope.
2. UX-00: inventory all routes, menus, stores, events, deep links, state owners, and the
   keep/move/merge/replace/remove decisions from Section 9.
3. Add `altai-control-protocol` ID, revision, actor, error, and event contracts.
4. Add Rust/TypeScript golden protocol fixtures.
5. Scaffold `altai-control-plane` and versioned SQLite migration runner.
6. Add control event log, idempotency, and transaction repository primitives.
7. Add canonical WorkItem with split work/execution states.
8. Add compatibility import for `altai-assignments.json` and current ledger tasks.
9. Add Organization/Goal/Project/ProjectWorkspace repositories.
10. Add AgentProfileRevision and AgentInstance repositories.
11. Add parent/dependency/comment persistence and tests.
12. Add WakeRequest, atomic checkout, and lease transaction.

The next vertical-slice PRs are fixed: IsanAgent `AttemptExecutor` conformance, Operations
read-model projections, then a read-only Operations shell against the real daemon. Mutation
cutover follows only after those three pass together.

Do not start the daemon UI, org chart UI, routine editor, or plugin manager before these
twelve PRs establish identity and ownership. UX-00 wireflows and contract fixtures are the
exception; they are required early specifically to prevent another temporary navigation or
store architecture.

## 12. Explicit Deletions and Deprecations

Deletion is a planned engineering deliverable, not optional cleanup. Entries below are
removed only after their replacement is live, legacy data is imported, deep-link aliases
are verified, and the named deletion gate passes.

### 12.1 Frontend components and menus

| Remove or retire | Replacement | Deletion gate |
| --- | --- | --- |
| AI sidebar `WorkHubPanel` and Work overlay button | Operations Work/Runs/Routines routes plus WorkItem shortcut | CP-17B/C/F live in Desktop and compact IDE routes |
| AI sidebar `TaskRunsPanel` | Operations Runs | Run projection/action parity tests pass |
| AI sidebar `AutomationsPanel` | Operations Routines | all legacy cron definitions imported and scheduler cut over |
| AI sidebar `NotificationInboxPanel` | canonical Operations Inbox | approvals, tickets, jobs, failures, and notifications map without loss |
| AI sidebar task/automation/inbox overlay event handling | canonical route/deep-link dispatcher | old event and slash aliases resolve to new routes |
| `AssignmentsRail` in GitHub/project surfaces | My Work, Runs, and linked Work badges | no assignment mutation remains in GitHub components |
| current `CommandCenter` implementation | projection-backed Operations Overview | metric/attention/delivery parity plus failure-state tests pass |
| current `ProjectBoardPanel` local aggregation | canonical Work board/list/graph | imported todos/assignments are reachable by WorkItem ID |
| `OrchestrationControlCenter` and `OrchestrationBar` | Operations header plus Project Policy and Routines | control plane owns pause/claim/retry/schedule end to end |
| duplicate `src/modules/orchestration/RunInspector.tsx` | shared canonical Run Detail/Inspector | all run actions bind to `run_id` with conformance tests |

The `Project Management` rail label is not deleted without replacement; it is migrated to
`Operations`. GitHub issue/PR browsing, comments, diffs, staging, commits, pushes, and branch
work remain. AI chat/history/composer and compact run inspection remain.

### 12.2 Frontend stores and controllers

| Remove or narrow | Required action |
| --- | --- |
| `assignmentsStore` persistence and `altai-assignments.json` | import to WorkItem/Attempt mappings, switch callers, then delete persistence and lifecycle mutations |
| `automationStore` | replace with generic Routine projection cache; delete native cron CRUD calls from UI |
| `notificationStore` domain ownership | replace with Inbox projection; keep only optional local read/display preferences |
| orchestration store and `OrchestrationController` | delete renderer claim, reconcile, retry, recovery, and dispatch loops |
| `todoStore` project-management use | restrict/rename to RunPlan state; prohibit imports from Operations Work modules |
| `agentRunsStore` task-status derivation | narrow to run projection/event cache; remove writes back into assignment/work state |
| `githubStore` work-lifecycle fields | narrow to ExternalObject/integration cache |
| duplicated attention-count selectors | replace with one checkpointed Inbox/Operations selector |

Architecture tests must fail when:

- an Operations component imports a legacy assignment, automation, notification, todo, or
  orchestration mutation store;
- an AI chat component imports Work/Routine lifecycle mutations;
- a GitHub component starts IsanAgent or transitions Work directly;
- a renderer contains claim, lease, schedule, retry, or recovery loops.

### 12.3 Host contracts, native commands, and events

After all clients use CP-15 contracts:

- remove `desktop.orchestration` as an ownership capability;
- replace `work.taskRuns`, `work.automations`, and `inbox.notifications` with explicit
  control-plane query/command capabilities;
- remove Tauri commands that CRUD IsanAgent cron directly for ALTAI project scheduling;
  replace them with Routine commands while retaining the IsanAgent `cron` tool and its
  standalone/agent-dorm providers;
- remove renderer-local `altai:agent-inbox-changed` and legacy Work overlay events after
  canonical subscription/deep-link aliases expire;
- remove session-based task lookup and require explicit WorkItem/Attempt/Run mappings;
- remove native assignment lifecycle commands that bypass coordinator transactions.

Old slash commands `/tasks` and `/automations` remain aliases for one stable release, emit a
deprecation notice, and then are removed from help/completion. Persisted external deep links
receive a versioned redirect rather than silently failing.

### 12.4 Runtime and persistence ownership

After successful cutover:

- IsanAgent cron is no longer the scheduler of record for ALTAI-managed project routines;
  the tool and its NativeLocal/NativeMultiTenantEdge implementations remain supported in
  their own host modes;
- duplicate background-job ownership across stores is removed;
- in-memory task mailboxes are not used for durable coordination;
- GitHub Overview/local card state is not an authoritative Work lifecycle;
- chat/session ID is never used as task identity;
- frontend JSON/local-storage state never acts as the durable project database;
- legacy and control-plane schedulers cannot both be enabled or compiled into the same
  production ownership path.

### 12.5 Documentation and test cleanup

Update or retire documentation that instructs users to keep ALTAI open for orchestration,
describes agent plan todos as project queues, or presents the combined GitHub Overview board
as authoritative. Replace legacy component/store tests with:

- route/deep-link compatibility tests;
- projection and command contract tests;
- migration reachability tests;
- cross-surface conformance tests;
- negative architecture/import tests;
- one-writer and renderer-reload end-to-end tests.

### 12.6 Physical-deletion checklist

A legacy item may be physically deleted only when all are true:

1. its data has a deterministic, repeatable import or is explicitly classified ephemeral;
2. every production caller uses the replacement contract;
3. persisted routes/events/commands have a tested redirect or documented expiry;
4. no active or historical WorkItem/Attempt/Run becomes unreachable;
5. rollback does not require dual-write or two schedulers;
6. accessibility and keyboard parity are verified on the replacement;
7. `rg`-based dead-reference checks and package/type tests pass;
8. the deletion PR names the removed state owner and its new canonical owner.

## 13. Definition of Paperclip-Style Behavior for ALTAI

The implementation is complete when this scenario works without Paperclip code:

1. A user creates a project linked to a goal and repository.
2. The user creates or selects an agent instance with a role, manager, profile, and budget.
3. The user or another agent creates a work item and assigns it.
4. Assignment creates a durable wake request.
5. The coordinator atomically checks dependencies, budget, policy, agent status, and
   workspace.
6. It checks out the work item, creates an attempt, and starts IsanAgent.
7. IsanAgent plans, uses tools/subagents, emits events, and requests runtime approvals as
   needed.
8. Comments, business approvals, and steering wake or resume the correct work.
9. The control plane verifies output, records cost/artifacts, and decides review/handoff.
10. A crash or UI shutdown does not lose ownership, context, schedule, or the next action.
11. Every surface shows the same task, run, agent, project, and audit identities.
12. GitHub and plugins can observe or extend the flow without becoming the scheduler.

That is the target: Paperclip-like operating semantics, implemented as native ALTAI
architecture around IsanAgent rather than Paperclip embedded inside ALTAI.
