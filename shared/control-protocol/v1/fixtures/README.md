# Control-Protocol v1 Fixtures

This directory holds golden JSON fixtures for the ALTAI control-plane protocol.

## Purpose

These fixtures are shared between Rust (`altai-control-protocol` crate, to be
created in CP-01) and TypeScript (`packages/control-contract/` or extended
`packages/host-contract/`, to be created in CP-01). Both language
implementations must serialize and deserialize these fixtures identically.

## Status

- **Not yet populated.** Fixtures are added by task packets beginning with
  CAL-02 (calibration) and CP-01-03 (golden cross-language fixtures).
- The first fixture will be `work-item-id.json` (CAL-02).

## Naming convention

| Prefix | Domain |
| --- | --- |
| `org-` | Organization |
| `goal-` | Goal |
| `project-` | Project |
| `workspace-` | ProjectWorkspace |
| `agent-profile-` | AgentProfile / AgentProfileRevision |
| `agent-instance-` | AgentInstance |
| `work-` | WorkItem |
| `dependency-` | WorkDependency |
| `comment-` | WorkComment |
| `attempt-` | Attempt |
| `run-binding-` | AttemptRunBinding |
| `lease-` | Lease |
| `wake-` | WakeRequest |
| `routine-` | Routine / RoutineRevision / RoutineRun |
| `approval-` | Approval |
| `budget-` | BudgetPolicy |
| `cost-` | CostEvent |
| `external-` | ExternalObjectLink |
| `activity-` | ActivityEvent |
| `error-` | Typed error examples |
| `invalid-` | Negative-test fixtures that must be rejected |

## Rules

1. Every fixture is valid JSON.
2. Every fixture includes a `"$schema"` field once a JSON schema is defined (CP-01).
3. Fixtures are never edited after acceptance; they are versioned.
4. Invalid fixtures (prefix `invalid-`) must be rejected by both Rust and TS parsers.
