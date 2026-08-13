# Control-Plane Execution

> **Canonical program plan:** `WORK_OS_PROGRAM_BACKLOG.md`
>
> **Architecture and scope reference:**
> `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md`

## What This Directory Contains

This directory contains the model-independent execution artifacts required to
deliver the ALTAI Work OS program without relying on conversation history.

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
| `inventory/` | Historical route/store/state-owner inventory. |

## How to Use

Read `WORK_OS_PROGRAM_BACKLOG.md` first. It is the only authoritative delivery
queue and status source. Other documents may define architecture, task mechanics,
or historical evidence, but they cannot define a competing implementation order.

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

## Golden Rule

> **Repository artifacts carry state, not chat memory.**
>
> A brand-new session can resume the program by reading these files. Do not rely
> on conversational memory across PRs.
