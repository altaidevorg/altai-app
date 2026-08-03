# Task Packet: GLM-CAL-03 — Pure Legacy-to-Canonical Mapping with Negative Tests

```yaml
task_id: GLM-CAL-03
title: One pure legacy-to-canonical mapping function plus negative tests
risk_tier: A
parent_module: CP-20
status: ready
depends_on: [GLM-CAL-02]
objective: |
  Create one pure mapping function that converts a legacy assignment record
  (from the current altai-assignments.json format) into a canonical WorkItem
  shape defined by the parent plan. The function must be pure (no side effects,
  no I/O) and must reject malformed input with typed errors. This proves the
  migration mapping pipeline before CP-20.
non_goals:
  - Creating the actual migration runner
  - Connecting to the control-plane database
  - Importing real data
  - Modifying any existing assignment store
  - Creating durable WorkItem records
allowed_files:
  - src-tauri/crates/altai-core/src/legacy_mapping.rs (new file)
  - src-tauri/crates/altai-core/src/legacy_mapping_tests.rs (new test file)
  - packages/host-contract/src/legacy-mapping.ts (new file)
  - packages/host-contract/src/__tests__/legacy-mapping.test.ts (new test file)
read_first:
  - docs/control-plane-execution/CONTEXT.md (full)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md Section 5.1 (work status)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md Section 5.2 (execution phase)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md CP-03 (Work domain shape)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md CP-20 (migration sources)
  - packages/host-contract/src/types.ts (existing TaskRunInfo type, lines 211–217)
invariants:
  - The mapping function is pure: same input always produces same output.
  - No I/O, no network, no database access.
  - Legacy status strings map to canonical WorkStatus values exactly as defined in Section 5.1.
  - Unknown legacy status values are rejected with a typed error, not silently mapped.
  - The function never invents IDs; it preserves legacy IDs in a compatibility field.
commands: []
events: []
migrations: []
tests_required:
  - "cargo test --manifest-path src-tauri/Cargo.toml legacy_mapping"
  - "pnpm --filter @altai/host-contract test -- legacy-mapping"
acceptance:
  - "Rust mapping function exists and compiles"
  - "TypeScript mapping function exists and passes type check"
  - "Both map a sample legacy assignment to the canonical WorkItem shape"
  - "Both reject unknown status with a typed error"
  - "Both reject missing required fields with a typed error"
  - "Both preserve the legacy ID in a 'legacy_compat_id' field"
  - "Round-trip: mapping the same input twice produces the same output"
stop_conditions:
  - "The legacy assignment format cannot be determined from the codebase"
  - "CP-01 canonical types are required but not yet accepted"
```

## Current relevant code facts

### Legacy assignment shape

The current `TaskRunInfo` type in `packages/host-contract/src/types.ts` (lines
211–217) represents the closest existing concept:

```typescript
export type TaskRunInfo = {
  id: string;
  chatId?: string;
  title: string;
  status: "queued" | "running" | "succeeded" | "failed" | "cancelled";
  createdAt: string;
};
```

### Canonical WorkItem shape (target)

From parent plan Sections 4.1 and 5.1:
```json
{
  "work_item_id": "wi_...",
  "title": "...",
  "work_status": "backlog|todo|in_progress|in_review|blocked|done|cancelled",
  "execution_phase": "none|queued|running|...",
  "legacy_compat_id": "original-id",
  "created_at": "2026-08-03T..."
}
```

### Status mapping

| Legacy status | Canonical `work_status` | Canonical `execution_phase` |
| --- | --- | --- |
| `queued` | `todo` | `queued` |
| `running` | `in_progress` | `running` |
| `succeeded` | `done` | `terminal` |
| `failed` | `in_progress` | `failed` |

> **Amendment (2026-08-03):** the original table mapped `failed` to work_status
> `needs_attention`, which is not a canonical WorkStatus under Section 5.1 (it is
> an ExecutionPhase under Section 5.2). Corrected to `in_progress` + `failed`:
> the work was started but is not complete; attention is derived from the
> execution phase via the Inbox projection.
| `cancelled` | `cancelled` | `terminal` |

## Expected diff shape

New files only:
1. `src-tauri/crates/altai-core/src/legacy_mapping.rs` — Rust pure mapping function + types
2. `src-tauri/crates/altai-core/src/legacy_mapping_tests.rs` — Rust tests
3. `packages/host-contract/src/legacy-mapping.ts` — TypeScript pure mapping function + types
4. `packages/host-contract/src/__tests__/legacy-mapping.test.ts` — TypeScript tests

No modifications to existing source files.

## Required negative tests

- Input with unknown status `"foobar"` → typed error `UnknownLegacyStatus`
- Input missing `title` → typed error `MissingRequiredField("title")`
- Input missing `id` → typed error `MissingRequiredField("id")`
- Input with empty `id` → typed error `InvalidLegacyId`
- Input with `null` status → typed error `MissingRequiredField("status")`

## Compatibility and deletion effects

None — new files only, no changes to existing assignment stores or types.

## Output/handoff format

Return the standard handoff report. Include the exact `cargo test` and `pnpm test`
output proving all positive and negative test cases pass.