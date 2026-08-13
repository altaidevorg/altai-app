# Task Packet: CAL-01 — Route/Store/State-Owner Inventory

```yaml
task_id: CAL-01
title: Produce a read-only route/store/state-owner inventory matching parent plan Section 9.8
risk_tier: A
parent_module: CP-00
status: accepted
depends_on: []
objective: |
  Produce a complete, evidence-backed inventory of every frontend route, menu,
  slash command, store mutation, event, badge, and deep link that currently owns
  or mutates control-plane-equivalent state. The inventory must match the
  keep/move/merge/replace/remove decisions in parent plan Section 9.8 and be
  saved as a machine-checkable artifact.
non_goals:
  - Implementing any control-plane code
  - Modifying any existing component behavior
  - Creating the Operations shell or routes
  - Deleting any legacy code
allowed_files:
  - docs/control-plane-execution/inventory/ (new directory for inventory artifacts)
read_first:
  - docs/control-plane-execution/CONTEXT.md (full)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md Section 9.8 (full table)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md Section 12.1–12.2
  - packages/host-contract/src/capabilities.ts (full)
  - packages/host-contract/src/ports.ts (full)
  - packages/host-contract/src/types.ts (full)
invariants:
  - This is a read-only task. No source code is modified.
  - Every inventory entry cites a file path and line number.
  - The inventory covers all items from the Section 9.8 migration map.
commands: []
events: []
migrations: []
tests_required:
  - "The inventory file passes a structural check (valid markdown/JSON with required fields per entry)"
  - "Every component name in Section 9.8's migration table is found in the codebase and cited"
acceptance:
  - "Inventory document exists at docs/control-plane-execution/inventory/ROUTE_STORE_INVENTORY.md"
  - "Every entry from Section 9.8 table has a corresponding row with file/line evidence"
  - "Each row has: component_name, file_path, line_range, current_owner, target_owner, disposition (keep/move/merge/replace/remove), deletion_gate"
  - "No source files were modified (git diff shows only new files under docs/)"
stop_conditions:
  - "A component in Section 9.8 cannot be found in the codebase (report as blocker)"
  - "The codebase has significantly diverged from Section 9.8 (report before improvising)"
```

## Current relevant code facts

The codebase has frontend modules under `src/modules/`:
- `src/modules/orchestration/` — React orchestration controller and UI
- `src/modules/ai/` — AI chat sidebar with Work/Inbox/Automation overlays
- `src/modules/github/` — GitHub integration surface
- `src/modules/source-control/` — diff/stage/commit/push

Host contract capabilities in `packages/host-contract/src/capabilities.ts` include:
- `work.taskRuns`, `work.automations` (lines 58–59)
- `inbox.notifications` (line 60)
- `desktop.orchestration` (line 68)

These will be replaced/narrowed per Section 12.2–12.3.

## Expected diff shape

New files only:
- `docs/control-plane-execution/inventory/ROUTE_STORE_INVENTORY.md` — the human-readable inventory
- Optionally `docs/control-plane-execution/inventory/route-store-inventory.json` — machine-readable version

No edits to existing source files.

## Required negative tests

This is a documentation/inventory task; no code tests are needed. However:
- The inventory must explicitly note any Section 9.8 component that is **not** found in the codebase.
- The inventory must note any existing store/mutation/event **not** listed in Section 9.8 (gap report).

## Compatibility and deletion effects

None — read-only task.

## Output/handoff format

Return the standard handoff report. The `CURRENT_STATE.md` "Existing Code Owners"
table should be updated (in a follow-up by the reviewer) with the completed
inventory reference.
