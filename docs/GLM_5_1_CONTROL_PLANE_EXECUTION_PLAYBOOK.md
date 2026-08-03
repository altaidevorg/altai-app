# GLM 5.1 Control-Plane Execution Playbook

> Status: proposed implementation companion
>
> Date: 2026-08-03
>
> Parent plan: `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md`
>
> Purpose: let a fresh-context GLM 5.1 coding agent implement the ALTAI control-plane plan
> safely without prior Paperclip, ALTAI, or IsanAgent conversation history

## 1. Decision

GLM 5.1 may implement this program, but it must not receive the whole program as one vague
long-running task. Treat the model as a capable, zero-context engineer operating through a
coding-agent harness.

The execution unit is one bounded task packet and one reviewable PR:

```text
canonical context bootstrap
  -> task-specific context manifest
  -> preflight understanding
  -> bounded implementation
  -> targeted verification
  -> self-review and evidence
  -> independent acceptance
  -> next fresh task/session
```

The parent plan remains authoritative. This playbook controls how work is handed to GLM 5.1;
it does not let the model reinterpret the architecture or substitute Paperclip code.

## 2. What the model can and cannot solve

Official Z.AI documentation describes GLM 5.1 as a text model with a 200K context window,
function calling, structured output, and long-horizon coding support. Those capabilities are
sufficient for this repository when a host supplies filesystem, shell, git, and test tools.
They do not give the model repository knowledge automatically.

Sources:

- <https://docs.z.ai/guides/llm/glm-5.1>
- <https://docs.z.ai/devpack/tool/others>
- <https://docs.z.ai/devpack/quick-start>

Operational assumptions:

- GLM 5.1 is not assumed to know Paperclip internals.
- It is not assumed to remember earlier ALTAI sessions.
- It is not allowed to make architecture decisions from model memory.
- API access alone is insufficient; implementation requires a coding-agent harness with
  read, search, edit, shell, and test capabilities.
- The advertised long-horizon ability is reserve capacity, not permission to create an
  eight-hour unreviewed diff.
- Actual tool limits, compaction, retries, and context retention are properties of the host
  as well as the model and must be tested in the chosen GLM integration.

If using Z.AI Coding Plan, use its coding endpoint only through a supported coding tool. If
using GLM 5.1 directly inside IsanAgent/ALTAI, use the appropriate API product and test tool
calling, streaming, cancellation, and structured outputs before production tasks.

## 3. Non-negotiable knowledge bootstrap

Every fresh GLM task begins by reading a small canonical pack in this order.

### Tier 0 — Always read

1. The task packet supplied for the current PR.
2. `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md` Sections 1–5.
3. The parent-plan module named by the task packet.
4. The parent-plan deletion/migration entries that mention the affected files or owner.
5. Repository `AGENTS.md` files if they are added later.
6. `git status --short` and the task's allowed file manifest.

### Tier 1 — Read when routed by the task

- `docs/adr/0001-shared-agent-service.md`
- `docs/adr/0002-agent-host-protocol.md`
- `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md`
- `docs/ALTAI_CLAW_IMPLEMENTATION_PLAN.md`
- `docs/GITHUB_PROJECT_WORKFLOWS.md`
- protocol fixtures and schema documents named by the task

The task packet names the exact sections to read. If a secondary document conflicts with
the parent plan's ownership model, the parent plan wins and the conflict is reported.

### Tier 2 — Code context

The model receives or discovers only:

- target files it may edit;
- directly imported interfaces and tests;
- one level of callers/consumers needed to preserve contracts;
- migration sources relevant to the task;
- commands needed to verify the slice.

Do not preload the entire repository, all UI components, or all IsanAgent source. Search for
specific symbols and expand context only when a dependency is proven relevant.

## 4. Compact architecture brief supplied to every task

The following block is copied verbatim into every GLM task packet:

```text
ALTAI is implementing Paperclip-style control-plane semantics without embedding or copying
Paperclip.

ALTAI Control Plane owns durable organization, goal, project, agent identity, WorkItem,
assignment, dependencies, wake queue, checkout/lease, Attempt, Routine, approval, budget,
recovery, audit, and external synchronization state.

IsanAgent owns execution of one authorized attempt: model/provider calls, sessions,
transcript, tools, permissions, checkpoints, compaction, failover, run-internal todo items,
run-internal subagents, and run events.

A session/chat is not a WorkItem. A todo_write item is a RunPlanItem. A subagent run is not
durable delegated project work unless the control plane creates a child WorkItem.

UI, Tauri commands, IDE webviews, plugins, GitHub, and IsanAgent may request transitions.
They are not authoritative state owners. Exactly one coordinator/scheduler may write
control-plane lifecycle state.

IsanAgent's cron tool is retained. Standalone Local and MultiTenantEdge providers remain.
ALTAI managed mode registers one ALTAI cron-compatible bridge to routines.* and suppresses
only the native ALTAI-hosted cron backend. Never expose two schedule backends or duplicate
cron tool definitions.

Do not invent temporary IDs, duplicate stores, renderer schedulers, dual-write, or a second
status model. Stop and report if the task requires violating these rules.
```

## 5. Context size strategy

The 200K model window is a ceiling, not a target.

| Context class | Target | Contents |
| --- | ---: | --- |
| Architecture brief | 2–4K tokens | invariants, vocabulary, task objective |
| Plan excerpts | 4–12K | current module, dependencies, migration/deletion gates |
| Code and tests | 20–80K | allowed files, interfaces, nearby callers, fixtures |
| Working reserve | at least 40% | tool results, compiler errors, diffs, test output |

If context exceeds the target:

1. summarize observed code facts into the task journal;
2. retain exact interface signatures, invariants, and unresolved errors;
3. drop unrelated file bodies and verbose successful test logs;
4. start a fresh session from the task journal if less than 25% useful context remains.

Do not rely on conversational memory across PRs. Repository artifacts carry state.

## 6. Required task-packet schema

Every task is a Markdown file or structured prompt with these fields:

```yaml
task_id: CP-XX-NN
title: short outcome
risk_tier: A | B | C
parent_module: CP-XX
status: ready | blocked | in_progress | review | accepted
depends_on: [accepted task ids]
objective: one externally verifiable outcome
non_goals: [explicit exclusions]
allowed_files: [files/directories]
read_first: [documents and exact sections]
invariants: [rules that must remain true]
commands: [allowed domain commands or APIs]
events: [events produced/consumed]
migrations: [schema/import implications]
tests_required: [exact commands and scenarios]
acceptance: [observable pass conditions]
stop_conditions: [conditions requiring escalation]
```

The packet also contains:

- current relevant code facts with file/line evidence;
- an expected diff shape, not a full implementation solution;
- required negative tests;
- compatibility and deletion effects;
- output/handoff format.

No task uses “implement CP-08” as its objective. A module is decomposed until one agent can
finish and verify it without changing more than one authoritative ownership boundary.

## 7. Risk tiers and acceptance authority

### Tier A — Bounded and locally provable

Examples:

- IDs and serialization types;
- golden Rust/TypeScript fixtures;
- read-only projections;
- component rendering against mock projections;
- documentation and compatibility redirects;
- pure mapping functions.

Limits:

- normally 1–5 edited files;
- normally under 500 net new/changed lines;
- targeted tests plus type/build checks;
- standard independent code review.

### Tier B — Stateful but isolated

Examples:

- SQLite migrations/repositories;
- idempotency records;
- Routine command port;
- IsanAgent attempt adapter;
- cron-to-Routine bridge;
- legacy import adapters;
- GitHub outbox/inbox adapter;
- one Operations UI slice with real commands.

Limits:

- normally under eight edited files or split the task;
- property, integration, restart, and failure-path tests as applicable;
- independent architecture review required;
- GLM self-review cannot accept its own implementation.

### Tier C — Safety- or ownership-critical

Examples:

- atomic checkout and leases;
- scheduler claim/coalescing;
- budget reservation and hard stops;
- approval authorization;
- daemon single-instance/authentication;
- workspace apply/finalize and destructive cleanup;
- plugin capability/security enforcement;
- single-writer cutover and physical legacy deletion.

GLM 5.1 may implement or propose these tasks, but acceptance requires:

- an independent reviewer session or human reviewer;
- invariant/property tests written before or alongside implementation;
- crash/race/adversarial scenarios;
- narrow diff and explicit rollback;
- no release based only on model-reported success.

## 8. Per-task execution protocol

### Step 1 — Preflight, no edits

The model must:

1. inspect repository instructions and dirty worktree state;
2. read the required context;
3. trace current owner, callers, persistence, commands, and events;
4. list files it expects to edit;
5. restate objective, non-goals, invariants, and tests;
6. identify contradictions or missing dependencies.

Required response shape:

```json
{
  "task_id": "CP-XX-NN",
  "understanding": "...",
  "current_owner": "...",
  "target_owner": "...",
  "files_to_edit": ["..."],
  "invariants": ["..."],
  "tests": ["..."],
  "blockers": []
}
```

Implementation begins only if this matches the packet. For Tier C, a reviewer accepts the
preflight before mutation.

### Step 2 — Implement the smallest vertical slice

Rules:

- preserve unrelated user changes;
- do not broaden scope because a neighboring refactor looks useful;
- use existing patterns unless they violate the parent plan;
- no placeholder domain store or temporary scheduler;
- no silent fallback to legacy mutation paths;
- add typed errors rather than string-matching domain state;
- add tests with the implementation, not as a follow-up promise;
- do not update accepted ADRs to justify an accidental implementation.

### Step 3 — Verify in layers

Run the narrowest relevant command first, then expand:

```text
Rust format/check:
  cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  cargo check --manifest-path src-tauri/Cargo.toml

Rust targeted/full tests:
  cargo test --manifest-path src-tauri/Cargo.toml <target>
  cargo test --manifest-path src-tauri/Cargo.toml
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

Frontend targeted/full checks:
  pnpm test -- <target>
  pnpm exec tsc --noEmit
  pnpm lint
  pnpm test
  pnpm build
```

Task packets select the required subset. Do not hide failures from unrelated existing
changes; classify them with evidence.

### Step 4 — Mandatory self-review

Before handoff, GLM must inspect its complete diff and answer:

- Did I create a second owner, scheduler, store, or identity?
- Did I derive Work status from session/run state?
- Did I bypass expected revision, idempotency, policy, or audit?
- Did I make an ALTAI-managed tool accept model-supplied scope as trusted authority?
- Did I accidentally change standalone IsanAgent behavior?
- Did I leave a compatibility path capable of mutation after cutover?
- Are new public contracts represented in fixtures and tests?
- Are deletions gated by reachability and migration checks?

### Step 5 — Handoff

Required final report:

```text
Outcome
Files changed
Contract/schema changes
Tests run with exact results
Invariants checked
Known limitations or follow-up task IDs
Migration/rollback impact
Diff risks requiring reviewer attention
```

## 9. Stop conditions

GLM must stop without improvising when:

- the parent plan and an accepted ADR conflict and the task does not authorize amendment;
- required dependency tasks are not accepted;
- a requested change would create dual-write or two schedulers;
- schema state/version is ambiguous;
- a migration could make historical work unreachable;
- unrelated dirty changes overlap allowed files;
- a public contract must change outside the packet's allowed scope;
- the same root-cause fix fails twice without new evidence;
- a required test cannot be run because of missing credentials/platform/service;
- a destructive operation lacks exact target and rollback;
- tool output or context compaction makes current state uncertain.

The stop report includes evidence and the smallest unblock request. It must not mark partial
work complete.

## 10. Repository execution artifacts

Before GLM implements production code, add this lightweight artifact structure in a
dedicated bootstrap PR:

```text
docs/control-plane-execution/
  CONTEXT.md                 compact canonical architecture brief
  CURRENT_STATE.md           accepted tasks, schema/protocol versions, active flags
  DECISIONS.md               accepted decision index and superseded conflicts
  TASK_TEMPLATE.md           packet schema and prompt contract
  REVIEW_CHECKLIST.md        Tier A/B/C acceptance gates
  tasks/
    CP-00-01.md
    CP-01-01.md
    ...
shared/control-protocol/v1/
  fixtures/
```

`CURRENT_STATE.md` is updated only when a task is accepted, not when GLM says it finished.
It records:

- accepted task/PR/commit;
- current schema and protocol versions;
- single-writer feature-flag state;
- active compatibility adapters;
- next ready tasks and blockers;
- known failing tests with evidence.

This prevents a new session from inferring progress from unmerged code or stale chat.

## 11. GLM workstream sequence

### Calibration lane

Do not begin with the scheduler. First prove the integration and instructions:

| Task | Risk | Outcome |
| --- | --- | --- |
| GLM-CAL-01 | A | read-only route/store/state-owner inventory matching Section 9.8 |
| GLM-CAL-02 | A | one small Rust/TypeScript shared fixture with round-trip tests |
| GLM-CAL-03 | A | one pure legacy-to-canonical mapping plus negative tests |

Acceptance measures instruction adherence, diff discipline, tool reliability, test honesty,
and handoff quality. Failure on two calibration tasks means the host/prompt integration must
be fixed before GLM receives stateful work.

### Foundation lane

| Order | Task packet | Risk | Depends on |
| ---: | --- | --- | --- |
| 1 | CP-00-01 ADR ownership amendments | A/B | calibration |
| 2 | CP-00-02 architecture import/boundary tests | B | CP-00-01 |
| 3 | CP-01-01 canonical IDs/revisions/actors/errors | A | CP-00-01 |
| 4 | CP-01-02 Rust/TypeScript event and command envelopes | B | CP-01-01 |
| 5 | CP-01-03 golden cross-language fixtures | A | CP-01-02 |
| 6 | CP-02-01 control-plane crate and migration runner | B | CP-01-02 |
| 7 | CP-02-02 event/idempotency transaction primitives | B/C | CP-02-01 |
| 8 | CP-03-01 WorkItem and split work/execution states | B | CP-02-02 |
| 9 | CP-03-02 legacy assignment/todo read-only importer | B | CP-03-01 |

### Control-plane vertical lane

| Order | Task packet | Risk | Outcome |
| ---: | --- | --- | --- |
| 10 | CP-04-01 Organization/Goal/Project/Workspace repositories | B | durable scope model |
| 11 | CP-05-01 AgentProfileRevision/AgentInstance repositories | B | durable agent identity |
| 12 | CP-06-01 parent/dependency/comment persistence | B | graph data |
| 13 | CP-06-02 cycle and eligibility properties | C | graph safety |
| 14 | CP-07-01 WakeRequest coalescing | C | durable wake |
| 15 | CP-07-02 checkout/lease transaction | C | single active owner |
| 16 | CP-08-01 AttemptExecutor and RunBinding | B/C | IsanAgent vertical execution |
| 17 | CP-08-02 schedule backend/provider seam | B | conditional schedule registration |
| 18 | CP-08-03 cron-to-Routine bridge | B/C | managed scheduling intent |
| 19 | CP-12-01 Routine model/command port | B | prerequisite may move before task 18 |
| 20 | CP-12-02 scheduler materialization path | C | RoutineRun → Work/Wake |

Dependency correction: CP-08-03 cannot merge before the minimum CP-12-01 Routine command
port exists. Task numbering reflects parent modules, not necessarily merge order. The ready
queue must enforce actual dependencies.

### Product-surface lane

Begin read-only UX contracts after stable projections, then enable commands one slice at a
time:

```text
UX-00 route/owner inventory
  -> Operations shell and context selector
  -> Work board/list/detail read-only
  -> Run detail read-only
  -> canonical Inbox read-only
  -> command enablement with revision conflicts
  -> AI sidebar consolidation
  -> GitHub ownership consolidation
  -> legacy menu/store physical deletion
```

No GLM task may simultaneously build a replacement UI and delete the old owner. Separate
replacement, migration/cutover, and physical-deletion PRs.

## 12. Special packet: cron-to-Routine bridge

This is the reference shape for the recent IsanAgent scheduling decision.

### Objective

In ALTAI managed mode, expose exactly one agent-visible `cron` tool backed by typed Routine
commands while preserving standalone IsanAgent Local and MultiTenantEdge behavior.

### Read first

- parent plan Sections 3.7, CP-08 Cron correction, CP-12, CP-20, and CP-21;
- `src-tauri/crates/altai-agent-service/src/host.rs`;
- `src-tauri/crates/altai-agent-service/src/instance_builder.rs`;
- Desktop and stdio host adapters;
- pinned IsanAgent `ToolRegistry`, `CronTool`, `CronActor`, and scheduling-mode code;
- Routine command protocol and fixtures from accepted CP-12-01.

### Invariants

- no removal/change to standalone IsanAgent cron implementations;
- one selected schedule backend and one tool catalog entry;
- scope comes from trusted attempt/tool context;
- idempotent add by attempt/tool-call ID;
- canonical Routine ID plus legacy compatibility mapping;
- no direct chat injection as authoritative scheduled project work;
- capability, approval, budget, timezone, and interval policy enforced;
- native and managed scheduler cannot both write.

### Required tests

- NativeLocal registration unchanged;
- NativeMultiTenantEdge registration unchanged;
- managed mode registers bridge but no native CronActor writer;
- `add`, `list`, `remove` round trip;
- repeated add returns one Routine;
- forged project/agent/chat scope is ignored or rejected;
- recurring creation without capability/approval fails closed;
- timezone and unsupported interval errors are typed;
- legacy cron ID removes the mapped Routine;
- tool catalog contains one `cron` definition;
- startup rejects conflicting backend flags.

### Non-goals

- redesigning IsanAgent cron schemas;
- deleting the cron skill;
- changing agent-dorm delivery;
- building the Routines UI;
- importing all legacy records in the bridge PR.

## 13. Reviewer protocol

The reviewer receives the original packet, preflight, complete diff, and test evidence. It
does not receive only GLM's summary.

Review order:

1. scope and dirty-worktree preservation;
2. ownership and identity invariants;
3. schema/protocol compatibility;
4. concurrency/idempotency/crash behavior;
5. security and trusted-context boundaries;
6. error and rollback paths;
7. tests that would fail under the old/incorrect behavior;
8. documentation/migration/deletion consequences.

Tier B/C reviews should use a fresh reviewer context. The implementation session must not
approve its own claims. Reviewer findings become a bounded follow-up packet, not an open
“fix everything” prompt.

## 14. Prompt templates

### 14.1 Bootstrap/preflight prompt

```text
You are implementing one bounded ALTAI control-plane task in an existing dirty-capable
repository. You have no prior conversation context. Repository documents and code are the
only source of truth.

Read the task packet completely, then read every required source in its read_first list.
Read the canonical architecture brief verbatim. Inspect git status and preserve unrelated
changes.

Do not edit yet. Return only the required preflight JSON. Cite file paths and line numbers
for current ownership claims. If a dependency, contract, or scope is missing, report it as
a blocker instead of inventing a temporary implementation.
```

### 14.2 Implementation prompt

```text
Your preflight is accepted for TASK_ID. Implement only the accepted file/scope manifest.
Follow the task invariants and parent plan. Add required positive and negative tests. Run
the narrow tests first and expand exactly as specified. Do not create dual-write, a second
scheduler/store, temporary identities, or renderer-owned lifecycle logic.

If reality differs from the accepted preflight, stop and report the new evidence before
broadening scope. At completion inspect the full diff, execute the mandatory self-review,
and return the handoff format with exact test outcomes.
```

### 14.3 Reviewer prompt

```text
Review TASK_ID independently against its original packet and canonical architecture brief.
Do not trust the implementer's summary. Inspect the complete diff and relevant callers.
Prioritize ownership, concurrency, idempotency, trusted scope, migration reachability,
rollback, and tests that prove the intended behavior. Report findings by severity with file
and line evidence. Do not implement unrelated improvements.
```

## 15. Metrics and go/no-go gates

Track GLM work separately from overall product metrics:

- task packet acceptance on first preflight;
- scope violations per task;
- fabricated/nonexistent API references;
- test-result reporting accuracy;
- reviewer findings by severity;
- reopened tasks and escaped regressions;
- median tokens/time per accepted Tier A/B/C task;
- context restart frequency;
- diffs exceeding packet limits;
- ownership or dual-writer violations: target zero.

Go/no-go:

- three accepted calibration tasks before stateful work;
- two consecutive clean Tier B tasks before Tier C implementation;
- any scheduler/lease/security ownership violation pauses Tier C delegation and triggers a
  prompt/host audit;
- no physical legacy deletion until replacement, migration, reachability, and rollback gates
  are independently verified.

## 16. Definition of success

This GLM execution system succeeds when a brand-new session can:

1. identify the correct current and target owners without conversation history;
2. implement exactly one task packet without architecture drift;
3. prove behavior through repository tests and artifacts;
4. stop safely when prerequisites or authority are missing;
5. hand off enough evidence for independent acceptance;
6. resume the program from accepted repository state rather than chat memory.

GLM 5.1's role is implementation capacity. The repository's contracts, task packets, tests,
and independent acceptance provide continuity and control.
