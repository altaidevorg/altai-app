# ADR 0003: Split control-plane ownership from execution-plane ownership

Date: 2026-08-03

Status: Accepted

## Context

ALTAI's agent runtime grew inside the Desktop application, and the earlier
Agent Operations plan placed orchestration ownership in a Tauri-resident Rust
orchestration service. That model leaves durable work, scheduling, and
recovery tied to a renderer process and blurs who may perform authoritative
state transitions. The parent plan
(`docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md`) establishes a
Paperclip-style control plane and requires exactly one owner for lifecycle
mutations (§3.1) while keeping IsanAgent as the primary execution runtime
(§3.2).

Without an explicit boundary, two failure modes recur: execution components
accumulate project-management state (tasks, assignments, budgets, routines),
and UI or adapter layers perform authoritative transitions independently.
Both fork identity, recovery, and audit behavior.

## Decision

Adopt a control-plane/execution-plane split with exactly one owner per plane.

**One control-plane owner.** A single user-scoped `altai-control-plane`
daemon owns every authoritative lifecycle mutation. React, Tauri commands,
IDE webviews, plugins, and IsanAgent may request transitions; they may not
perform authoritative transitions independently. No second scheduler,
renderer-side store, or duplicate state model may claim or dispatch work.

**IsanAgent remains the execution runtime.** IsanAgent, reached through
`altai-agent-service`, executes one authorized attempt at a time and reports
exactly what happened. It receives an immutable attempt specification and
emits a durable, sequenced event stream. It does not become a project
system.

### Ownership boundary

| Control plane owns | IsanAgent owns |
| --- | --- |
| Work-item lifecycle | Provider/model execution |
| Assignment and checkout | One run's conversation/session context |
| Dependency eligibility | Tool invocation |
| Wake request coalescing | Permission enforcement for filesystem, shell, MCP, and edits |
| Routine scheduling | Checkpoints and run-level recovery |
| Retries and recovery | Steering and cancellation of an active run |
| Budget hard stops | Run-internal subagents |
| Authoritative terminal disposition | Compaction and model failover |
| External-source reconciliation | Sequenced run events and transcript replay |

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

Run-internal objects stay run-internal: `todo_write` items are `RunPlanItem`
records, subagents are `SubagentRun` records under one parent run, and a
chat session is execution context, not task ownership. If run-internal work
must become independently owned, scheduled, or reviewable, the control plane
creates a child work item with its own attempt and run.

## Consequences

Positive consequences:

- Every mutation has a single answerable owner; UI components are never
  required for scheduling or recovery correctness.
- Durable work outlives any renderer because the control-plane daemon, not
  an extension host or webview, holds it (amending ADR 0001 accordingly).
- IsanAgent standalone and agent-dorm deployments are unaffected: the
  execution plane keeps its capabilities, including the host-selectable
  `cron` tool, and gains no project-management obligations.
- The boundary is enforceable with dependency rules: execution crates may
  not import control-plane persistence, and renderers may not own
  schedulers.

Costs and constraints:

- Features that previously wrote state directly from Tauri commands or React
  stores must be routed through control-plane commands, which adds adapter
  work before any new capability lands.
- Two persistence planes (user-scoped control database plus workspace/run
  journals) must be kept consistent through stable identities and explicit
  mappings, not dual-write.
- The Agent Operations plan's ownership sections that predate this split are
  superseded where they conflict and must be read through the parent plan.

## Rejected alternatives

1. Keep orchestration ownership in the Tauri-resident orchestration modules:
   rejected because it couples durable work and scheduling to a renderer
   process and cannot serve IDE, Studio, CLI, and plugin clients equally.
2. Let IsanAgent grow project-task, assignment, and routine ownership:
   rejected because it forks the execution runtime into a second control
   plane and breaks standalone and multi-tenant-edge deployments.
3. Allow renderers to own projections with local mutation authority:
   rejected because independent authoritative transitions violate the
   one-owner rule and corrupt audit and recovery semantics.

## Follow-up

CP-01 defines the shared domain contracts that cross this boundary; CP-02
implements the control-plane database the owner persists to; CP-08 builds
the attempt adapter that hands immutable attempt specifications to
IsanAgent; CP-16 implements the daemon lifecycle. CP-00-02 adds the
dependency/architecture tests that make this boundary fail the build when
violated.
