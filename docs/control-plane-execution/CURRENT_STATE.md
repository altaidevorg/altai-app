# Control-Plane Execution Current State

> **Rule:** This file is updated **only** when a task is accepted (merged/reviewed),
> not when an agent says it finished. It records canonical progress.
>
> **Date:** 2026-08-03
>
> **Last updated by:** PR #262 acceptance review

## Accepted Tasks

| Task ID | Status | PR/Commit | Date | Notes |
| --- | --- | --- | --- | --- |
| GLM-CAL-01 | accepted | PR #262 | 2026-08-03 | Route/store/state-owner inventory validated; 56 unique entries with valid source references |

## Current Schema and Protocol Versions

| Artifact | Version | Status |
| --- | --- | --- |
| Agent host protocol (`shared/agent-protocol/v1/`) | v1 | accepted (ADR 0002) |
| Host contract (`packages/host-contract/`) | v1 | accepted |
| Control-plane protocol (`shared/control-protocol/v1/`) | — | not yet created (CP-01) |
| Control-plane DB schema | — | not yet created (CP-02) |

## Active Feature Flags

| Flag | Default | Status | Owner module |
| --- | --- | --- | --- |
| `control_plane_enabled` | `false` | not yet created | CP-00 / CP-02 |
| `schedule_backend_mode` | `NativeLocal` | IsanAgent default | CP-08 |
| `legacy_cron_compatibility` | `false` | not yet needed | CP-08 / CP-20 |

## Active Compatibility Adapters

| Adapter | Legacy source | Canonical target | Status |
| --- | --- | --- | --- |
| _(none yet)_ | — | — | Migration begins at CP-20 |

## Next Ready Tasks

| Task ID | Risk | Depends on | Status |
| --- | --- | --- | --- |
| GLM-CAL-02 | A | GLM-CAL-01 | **ready** |
| GLM-CAL-03 | A | GLM-CAL-02 | **ready** |
| CP-00-01 | A/B | calibration | blocked (calibration required) |
| CP-00-02 | B | CP-00-01 | blocked |

## Known Failing Tests / Blockers

| Test/Area | Evidence | Impact | Since |
| --- | --- | --- | --- |
| _(none)_ | — | — | — |

## Existing Code Owners (Inventory Summary)

> Full inventory: `docs/control-plane-execution/inventory/ROUTE_STORE_INVENTORY.md`.

| Area | Current owner | Target owner | Migration task |
| --- | --- | --- | --- |
| Orchestration scheduler/controller | `src/modules/orchestration/` (React) | `altai-control-plane` (Rust) | CP-07 / CP-20 |
| Assignment lifecycle | `assignmentsStore.ts` | control-plane WorkItem | CP-03 / CP-20 |
| Automation/cron | `automationStore.ts` + IsanAgent `CronTool` | control-plane Routines | CP-08 / CP-12 |
| Notifications | `notificationStore.ts` | control-plane Inbox projection | CP-10 / CP-13 |
| GitHub task status | `githubStore.ts` | control-plane ExternalObject | CP-14 |

## Important Notes

- The agent host protocol (ADR 0002) and shared agent service (ADR 0001) are
  accepted. They govern the execution plane, not control-plane ownership.
- CP-00 ADR amendments will amend ADR 0001 and 0002 to add control-plane scope.
- No production code has been written for the control plane yet.
