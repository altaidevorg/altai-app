# Control-Plane Task Packet Template

> **Purpose:** Every control-plane implementation task is defined by a packet
> following this exact schema. Copy this file, fill in the fields, and save as
> `docs/control-plane-execution/tasks/CP-XX-NN.md`.
>
> A packet is a prompt contract, not a design document. It must be specific
> enough that a fresh-context agent can implement exactly one vertical slice
> without architecture drift.

## Required Fields

```yaml
task_id: CP-XX-NN          # e.g. CP-07-03
title: short outcome        # externally verifiable, not "implement CP-08"
risk_tier: A | B | C        # see REVIEW_CHECKLIST.md for tier definitions
parent_module: CP-XX        # from the module dependency map
status: ready | blocked | in_progress | review | accepted
depends_on: []              # list of task IDs that must be accepted first
objective: |                # one externally verifiable outcome
  ...
non_goals: []               # explicit exclusions — what NOT to do
allowed_files: []           # exact files/directories the agent may edit
read_first: []              # documents and exact sections to read before starting
invariants: []              # rules that must remain true after implementation
commands: []                # allowed domain commands or APIs
events: []                  # events produced or consumed
migrations: []              # schema/import implications
tests_required: []          # exact commands and scenarios
acceptance: []              # observable pass conditions
stop_conditions: []         # conditions requiring escalation
```

## Additional Packet Contents

Beyond the YAML frontmatter, each packet must include:

### Current relevant code facts

Cite file paths and line numbers for:
- current owner of the concept being changed;
- callers/consumers that depend on existing contracts;
- persistence or state that must be preserved.

### Expected diff shape

Describe the kind of changes expected (new file, trait addition, migration,
test), **not** a full implementation. The agent discovers the exact code.

### Required negative tests

List specific scenarios that must fail correctly (e.g., "stale revision rejected
with typed error", "unknown ID returns 404 not 500").

### Compatibility and deletion effects

- Does this change break any existing API consumer?
- Does this require a migration?
- Is any legacy path removed? If so, cite the deletion gate.

### Output/handoff format

The implementer must return the handoff report below.

## Execution Protocol Summary

Every task follows these steps:

1. **Preflight (no edits):** read context, trace owners, list files, restate
   objective. Return preflight JSON.
2. **Implement the smallest vertical slice:** preserve unrelated changes, use
   existing patterns, no placeholder stores or temporary schedulers.
3. **Verify in layers:** run narrowest tests first, then expand.
4. **Mandatory self-review:** answer the self-review questions.
5. **Handoff:** return the final report with exact test outcomes.

## Context Budget Guidelines

| Context class | Target | Contents |
| --- | ---: | --- |
| Architecture brief | 2–4K tokens | `CONTEXT.md` invariants + task objective |
| Plan excerpts | 4–12K | current module, dependencies, migration/deletion gates |
| Code and tests | 20–80K | allowed files, interfaces, nearby callers, fixtures |
| Working reserve | ≥40% | tool results, compiler errors, diffs, test output |

If context exceeds the target, summarize code facts into the task journal,
retain exact signatures and invariants, and start a fresh session from the
journal if less than 25% useful context remains.

## Preflight Response Shape

Before any edits, the agent must return this JSON:

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

Implementation begins only if the preflight matches the packet. For Tier C, a
reviewer must accept the preflight before any mutation.

## Handoff Response Shape

```text
Outcome
Files changed
Contract/schema changes
Tests run with exact results
Invariants checked
Known limitations or follow-up task IDs
Migration/rollback impact
Diff risks requiring reviewer attention
