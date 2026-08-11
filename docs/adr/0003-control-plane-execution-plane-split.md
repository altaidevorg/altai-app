# ADR 0003: Split control-plane ownership from execution-plane ownership

Date: 2026-08-03

Status: Amended (2026-08-11)

## Amendment (2026-08-11)

The logical **control vs execution** split remains. The **deployment** decision
does not.

Authoritative Work / Attempt / Review / Inbox mutations live in the **existing
Rust agent host/service** process (user-scoped `work.db` beside current host
state). Do **not** create a separate `altai-control-plane` crate, daemon,
server, or deployment.

Canonical product plan: [`altaidevorg/altai-agent-work-os`](https://github.com/altaidevorg/altai-agent-work-os)
(`ENGINEERING.md`, `PRODUCT.md`, `SCREENS.md`, `ROADMAP.md`).

| Keep from original ADR | Change |
| --- | --- |
| One owner for lifecycle mutations | Owner = existing host/service, not a new daemon |
| IsanAgent is execution-only | Unchanged |
| Renderers must not own authoritative transitions | Unchanged |
| Separate Assignment/Org/Budget/Outbox ownership in IsanAgent | Still rejected; those domains are out of M1 scope |

Follow-ups CP-02/CP-16 that assumed a new daemon are superseded by Work OS
Milestone 1 (five tables + Work API inside the existing host).

---

## Context

ALTAI's agent runtime grew inside the Desktop application, and the earlier
Agent Operations plan placed orchestration ownership in a Tauri-resident Rust
orchestration service. That model leaves durable work, scheduling, and
recovery tied to a renderer process and blurs who may perform authoritative
state transitions.

Without an explicit boundary, two failure modes recur: execution components
accumulate project-management state (tasks, assignments, budgets, routines),
and UI or adapter layers perform authoritative transitions independently.
Both fork identity, recovery, and audit behavior.

## Decision

Adopt a control-plane/execution-plane split with exactly one owner per plane.

**One control-plane owner (amended).** The existing ALTAI Rust host/service
owns every authoritative Work lifecycle mutation and persists them in
`work.db`. React, IDE webviews, CLI, and plugins may request transitions; they
may not perform authoritative transitions independently.

**IsanAgent remains the execution runtime.** IsanAgent, reached through
`altai-agent-service`, executes one authorized attempt at a time and reports
exactly what happened. It receives an immutable attempt specification and
emits a durable, sequenced event stream. It does not become a project
system.

### Ownership boundary

| Control plane owns (host `work.db`) | IsanAgent owns |
| --- | --- |
| Work-item lifecycle | Provider/model execution |
| Attempt / Review records | One run's conversation/session context |
| Inbox projection inputs | Tool invocation |
| Work events / recovery history | Permission enforcement for filesystem, shell, MCP, and edits |
| (Later) Routines | Checkpoints and run-level recovery |
| Authoritative Accept/Return disposition | Steering and cancellation of an active run |
| | Run-internal subagents |
| | Compaction and model failover |
| | Sequenced run events and transcript replay |

IsanAgent must not own:

- project Work items as a PM system;
- organization or project administration state;
- durable task assignment stores;
- cross-run task dependencies as a product graph;
- company/project/agent budget policy;
- the canonical work inbox as notification records.

Run-internal objects stay run-internal: `todo_write` items are Run Plan
records, subagents are under one parent run, and a chat session is execution
context, not Work identity. If run-internal work must become independently
owned or reviewable, the host creates a child Work item with its own Attempt.

## Consequences

Positive consequences:

- Every mutation has a single answerable owner; UI components are never
  required for scheduling or recovery correctness.
- Durable work outlives any renderer because the host process, not an
  extension host or webview, holds it.
- IsanAgent standalone and agent-dorm deployments are unaffected.

Costs and constraints:

- Features that previously wrote state from React stores must route through
  host Work commands.
- Workspace run journals and user-scoped `work.db` stay consistent through
  stable IDs, not dual-write of lifecycle state.

## Rejected alternatives

1. Keep orchestration ownership only in Tauri-resident renderer modules:
   rejected because it couples durable work to a UI process.
2. Let IsanAgent grow project-task and assignment ownership: rejected.
3. Allow renderers to own projections with local mutation authority: rejected.
4. **(Amended rejection)** Ship a separate `altai-control-plane` daemon for M1:
   rejected as premature complexity; revisit only with a measured failure the
   existing host cannot solve (Work OS hard limit).

## Follow-up

Implement Work OS Milestone 1: five SQLite tables, Work/Inbox API, shared
agent-ui screens, CLI `work`/`inbox`, and in-place `altai-vscode` migration.
