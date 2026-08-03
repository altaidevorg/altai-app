# ALTAI Control-Plane Canonical Architecture Brief

> **Purpose:** This file is copied verbatim into every control-plane task packet.
> It provides the minimal non-negotiable invariants a fresh-context agent needs.
>
> **Parent authority:** `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md`
> This file is a compact summary; the parent plan wins on any conflict.

## Product Boundary

```text
ALTAI Control Plane decides what should run, why, when, for whom, and under which policy.
IsanAgent executes one authorized attempt and reports exactly what happened.
```

## Ownership Model

### ALTAI Control Plane owns

- durable organization, goal, project, agent identity, WorkItem;
- assignment, dependencies, wake queue, checkout/lease;
- Attempt, Routine, approval, budget, recovery, audit;
- external synchronization state.

### IsanAgent owns

- execution of one authorized attempt;
- model/provider calls, sessions, transcript;
- tools, permissions, checkpoints, compaction;
- failover, run-internal todo items, run-internal subagents, run events.

### IsanAgent must NOT own

- project tasks, organization or project state;
- durable task assignment, cross-run task dependencies;
- project routines, company/project/agent budget policy;
- GitHub synchronization, business approvals, the canonical work inbox.

## Invariants (never violate)

1. **One control-plane owner.** Exactly one service owns lifecycle mutations.
   React, Tauri commands, IDE webviews, plugins, and IsanAgent may request
   transitions. They are not authoritative state owners.

2. **One identity per concept.** These IDs are distinct and never substituted:
   `organization_id`, `goal_id`, `project_id`, `workspace_id`,
   `agent_instance_id`, `agent_profile_id` + `agent_profile_revision_id`,
   `work_item_id`, `attempt_id`, `run_id`, `session_id`,
   `routine_id` + `routine_revision_id` + `routine_run_id`,
   `approval_id`, `external_object_id`.

3. **Parentage is not dependency.** `parent_work_item_id` explains decomposition.
   `work_dependencies` controls execution eligibility. A child does not
   automatically block its parent.

4. **Configuration and state are separate.** Version-controlled config stays in
   `ALTAI.md`, `WORKFLOW.md`, and `.altai/` profile files. Runtime state lives in
   the control-plane database.

5. **No second scheduler during migration.** Feature flags select a single writer.
   Shadow evaluation may compare decisions but only one scheduler claims/dispatches.

6. **IsanAgent cron is host-selectable, not removed.** Standalone Local and
   MultiTenantEdge providers remain. ALTAI managed mode registers one
   ALTAI cron-compatible bridge to `routines.*` and suppresses only the native
   ALTAI-hosted cron backend. Never expose two schedule backends or duplicate
   cron tool definitions.

7. **No invented identities or dual-write.** Do not invent temporary IDs,
   duplicate stores, renderer schedulers, dual-write, or a second status model.
   Stop and report if a task requires violating these rules.

## Domain Vocabulary (compact)

| Object | Meaning |
| --- | --- |
| Organization | Isolation, policy, agent, and budget boundary |
| Goal | Desired outcome and ancestry |
| Project | Work, repository/workspace, policy, and delivery context |
| WorkItem | User- or agent-visible unit of project work |
| Dependency | Execution blocker between work items |
| Attempt | One coordinator-authorized execution attempt |
| Run | IsanAgent execution corresponding to an attempt |
| Routine | Versioned recurring or event-triggered work definition |
| WakeRequest | Idempotent request to evaluate/run an agent |
| Approval | Governance decision with explicit scope and payload |

### Run-internal objects (not project tasks)

- `todo_write` items → `RunPlanItem` records.
- IsanAgent subagents → `SubagentRun` records under one parent run.
- Background shell/process jobs → run execution resources.
- A chat session → execution context, not task ownership.

## State Model

### Work status (human/integration-facing)

`backlog → todo → in_progress → in_review → done`
`→ blocked`, `→ cancelled`

### Execution phase (stored separately)

`none, queued, planning, awaiting_plan_approval, running, awaiting_input,
awaiting_approval, verifying, reviewing, retrying, paused, failed,
needs_attention, terminal`

### Attempt state

`created → claimed → dispatched → running → finalizing → terminal`

Terminal outcomes: `succeeded, failed, cancelled, timed_out, budget_stopped,
policy_denied, lost`

Task completion is **not** inferred from a successful model response.

## Schedule Backend Selection

| Host mode | Agent-visible intent | Backend owner |
| --- | --- | --- |
| IsanAgent standalone/local | `cron` | IsanAgent `CronActor` local mode |
| IsanAgent agent-dorm/multi-tenant edge | `cron` | IsanAgent multi-tenant-edge cron scheduler |
| ALTAI legacy compatibility | `cron` | existing IsanAgent local cron (migration only) |
| ALTAI managed control plane | `cron` compatibility + Routine affordances | ALTAI `routines.*` + control-plane scheduler |

Selected mode is immutable for an attempt. Exactly one backend is registered.

## Current Repository Architecture

```text
src-tauri/crates/
  altai-core/           workspace, policy, config, journal primitives
  altai-protocol/       JSON-RPC framing, message types
  altai-agent-service/  IsanAgent lifecycle, workspace services
  altai-collaboration/  collaboration primitives
  altai-cli/            stdio host adapter, one-shot host, serve mode

packages/
  agent-protocol/       TypeScript JSON-RPC schema + validation
  agent-ui/             shared UI components
  host-contract/        host ports, capabilities, DTOs

shared/
  agent-protocol/v1/    schema.json + golden JSON fixtures

src/modules/            Desktop frontend modules (orchestration, ai, github, ...)
```

### Not-yet-existing crates/packages (to be created by task packets)

- `src-tauri/crates/altai-control-protocol/` — Rust domain contracts (CP-01)
- `src-tauri/crates/altai-control-plane/` — control DB + repositories (CP-02)
- `shared/control-protocol/v1/` — golden JSON fixtures for control-plane types
- `packages/control-contract/` — TypeScript control-plane contracts (CP-01)

## Module Dependency Map

```text
CP-00 Architecture fence
  → CP-01 Shared domain contracts
      → CP-02 Control database
          → CP-03 Work domain
          → CP-04 Organizations / goals / projects
          → CP-05 Agent registry
          → CP-06 Hierarchy / dependencies / comments
              → CP-07 Assignment / checkout / wake queue
                  → CP-08 IsanAgent attempt adapter
                  → CP-09 Workspace / delivery
                  → CP-10 Approval / governance
                  → CP-11 Usage / cost / budgets
                  → CP-12 Routines / scheduler
                      → CP-13 Liveness / recovery
                          → CP-14 External objects / GitHub
                          → CP-15 Public protocol
                              → CP-16 Daemon lifecycle
                              → CP-17 Read models / UI
                              → CP-18 Adapters
                              → CP-19 Plugin runtime
                                  → CP-20 Migration / cutover
                                      → CP-21 Soak / release gates