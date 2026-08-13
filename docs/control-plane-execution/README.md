# Control-Plane Execution

> **Companion to:** `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md` (parent plan)
> and `docs/GLM_5_1_CONTROL_PLANE_EXECUTION_PLAYBOOK.md` (execution playbook).

## What This Directory Contains

This directory contains the **execution artifacts** that enable a fresh-context
coding agent (e.g., GLM 5.1) to implement the ALTAI control-plane plan safely
without prior conversation history.

These artifacts are defined by **playbook Section 10** and are the prerequisite
for all control-plane implementation tasks.

## Files

| File | Purpose |
| --- | --- |
| `WORK_OS_PROGRAM_BACKLOG.md` | **Canonical PM order:** current status, next package, dependencies, PR slices, acceptance gates, and progress percentages. |
| `CONTEXT.md` | Compact canonical architecture brief. Copied verbatim into every task packet. |
| `CURRENT_STATE.md` | Accepted tasks, schema/protocol versions, active flags, blockers. Updated only on task acceptance. |
| `DECISIONS.md` | Accepted decision index and superseded conflicts. |
| `TASK_TEMPLATE.md` | Packet schema and prompt contract for defining implementation tasks. |
| `REVIEW_CHECKLIST.md` | Tier A/B/C acceptance gates, self-review questions, and reviewer protocol. |
| `tasks/` | Individual task packet files, one per bounded implementation task. |
| `inventory/` | (Created by GLM-CAL-01) Route/store/state-owner inventory. |

## How to Use

Read `WORK_OS_PROGRAM_BACKLOG.md` first. If another document lists a different
implementation order, this backlog controls sequencing; the parent engineering
plan continues to control architecture and scope.

### If you are defining a new task

1. Copy `TASK_TEMPLATE.md`.
2. Fill in all required fields.
3. Save as `tasks/CP-XX-NN.md`.
4. Ensure `depends_on` references only accepted tasks.
5. Set `status: ready` or `status: blocked`.

### If you are implementing a task

1. Read `CONTEXT.md` first.
2. Read the task packet's `read_first` list completely.
3. Follow the execution protocol in `TASK_TEMPLATE.md`
   (preflight → implement → verify → self-review → handoff).
4. Do not edit outside `allowed_files`.
5. Stop and report if any `stop_conditions` are met.

### If you are reviewing a task

1. Use `REVIEW_CHECKLIST.md`.
2. Review in the specified order (scope → ownership → schema → concurrency →
   security → errors → tests → docs).
3. For Tier B/C, use a fresh reviewer context.
4. Update `CURRENT_STATE.md` only when the task is accepted.

## Workstream Sequence

### Calibration Lane (must pass before stateful work)

| Task | Risk | Outcome |
| --- | --- | --- |
| `GLM-CAL-01` | A | Route/store/state-owner inventory matching Section 9.8 |
| `GLM-CAL-02` | A | One small Rust/TS shared fixture with round-trip tests |
| `GLM-CAL-03` | A | One pure legacy-to-canonical mapping plus negative tests |

### Foundation Lane

| Order | Task | Risk | Depends on |
| ---: | --- | --- | --- |
| 1 | `CP-00-01` | A/B | calibration |
| 2 | `CP-00-02` | B | `CP-00-01` |
| 3 | `CP-01-01` | A | `CP-00-01` |
| 4 | `CP-01-02` | B | `CP-01-01` |
| 5 | `CP-01-03` | A | `CP-01-02` |
| 6 | `CP-02-01` | B | `CP-01-02` |
| 7 | `CP-02-02` | B/C | `CP-02-01` |
| 8 | `CP-03-01` | B | `CP-02-02` |
| 9 | `CP-03-02` | B | `CP-03-01` |

### Control-Plane Vertical Lane

| Order | Task | Risk | Outcome |
| ---: | --- | --- | --- |
| 10 | `CP-04-01` | B | Organization/Goal/Project/Workspace repositories |
| 11 | `CP-05-01` | B | AgentProfileRevision/AgentInstance repositories |
| 12 | `CP-06-01` | B | Parent/dependency/comment persistence |
| 13 | `CP-06-02` | C | Cycle and eligibility properties |
| 14 | `CP-07-01` | C | WakeRequest coalescing |
| 15 | `CP-07-02` | C | Checkout/lease transaction |
| 16 | `CP-08-01` | B/C | AttemptExecutor and RunBinding |
| 17 | `CP-08-02` | B | Schedule backend/provider seam |
| 18 | `CP-08-03` | B/C | Cron-to-Routine bridge |
| 19 | `CP-12-01` | B | Routine model/command port |
| 20 | `CP-12-02` | C | Scheduler materialization path |

### Product-Surface Lane

Begins read-only UX contracts after stable projections. See parent plan Section
9.12 for UX delivery modules (UX-00 through UX-09).

## Golden Rule

> **Repository artifacts carry state, not chat memory.**
>
> A brand-new session can resume the program by reading these files. Do not rely
> on conversational memory across PRs.
