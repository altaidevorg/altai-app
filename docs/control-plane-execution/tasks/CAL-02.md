# Task Packet: CAL-02 — Shared Fixture Round-Trip Tests

```yaml
task_id: CAL-02
title: Create one small Rust/TypeScript shared fixture with round-trip tests
risk_tier: A
parent_module: CP-01
status: accepted
depends_on: [CAL-01]
objective: |
  Create one golden JSON fixture representing a minimal control-plane concept
  (WorkItemId) and prove Rust and TypeScript can serialize/deserialize it
  identically. This validates the cross-language fixture pipeline before real
  domain contracts are built in CP-01.
non_goals:
  - Defining all CP-01 domain types (that is CP-01-01)
  - Creating the altai-control-protocol crate
  - Creating the altai-control-plane crate
  - Modifying existing agent-protocol fixtures
allowed_files:
  - shared/control-protocol/v1/fixtures/ (new directory)
  - src-tauri/crates/altai-protocol/tests/control_fixture_cal.rs (new test file)
  - packages/agent-protocol/src/control-fixture-cal.ts (new test helper)
  - packages/agent-protocol/src/__tests__/control-fixture-cal.test.ts (new test file)
read_first:
  - docs/control-plane-execution/CONTEXT.md (full)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md Section 3.3 (identity rules)
  - docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md CP-01 exit gate
  - shared/agent-protocol/v1/schema.json (existing fixture pattern)
  - shared/agent-protocol/v1/fixtures/initialize-request.json (existing fixture example)
  - src-tauri/crates/altai-protocol/tests/fixtures.rs (existing Rust fixture test pattern)
  - packages/agent-protocol/src/__tests__/fixtures.test.ts (existing TS fixture test pattern)
invariants:
  - The fixture uses a prefixed string ID format (e.g. "org_" prefix for OrganizationId).
  - Rust and TypeScript must produce byte-identical JSON when serializing the same value.
  - Deserialization rejects malformed/unknown values with typed errors, not panics.
  - No production crate is created; this proves the fixture pipeline only.
commands: []
events: []
migrations: []
tests_required:
  - "cargo test --manifest-path src-tauri/Cargo.toml control_fixture_cal"
  - "pnpm --filter @altai/agent-protocol test -- control-fixture-cal"
acceptance:
  - "Golden fixture exists at shared/control-protocol/v1/fixtures/work-item-id.json"
  - "Rust test reads, deserializes, re-serializes, and asserts byte-identical output"
  - "TypeScript test reads, deserializes, re-serializes, and asserts byte-identical output"
  - "Both tests reject an invalid fixture (missing/empty prefix) with a typed error"
  - "cargo check passes"
  - "tsc --noEmit passes for the agent-protocol package"
stop_conditions:
  - "Existing altai-protocol crate structure prevents adding a test file without modifying Cargo.toml"
  - "agent-protocol package test configuration cannot load JSON fixtures from outside its package"
```

## Current relevant code facts

### Existing Rust fixture pattern

`src-tauri/crates/altai-protocol/tests/fixtures.rs` contains integration tests
that read JSON fixtures from `shared/agent-protocol/v1/fixtures/` and validate
them. The workspace `Cargo.toml` includes `altai-protocol` as a workspace member.

### Existing TypeScript fixture pattern

`packages/agent-protocol/src/__tests__/fixtures.test.ts` imports JSON fixtures
and validates them using `validateMessage()` from `schema.ts`. Vitest is the test
runner (`vitest run` in package.json scripts).

### Fixture directory

`shared/agent-protocol/v1/fixtures/` contains golden JSON fixtures like
`initialize-request.json`, `run-start-request.json`, etc.

### ID format from parent plan

Section 3.3 defines canonical IDs. For this calibration task, use a simple
prefixed-string format:
```json
{
  "type": "work_item_id",
  "value": "wi_01923abc-def0-7abc-8def-0123456789ab"
}
```

## Expected diff shape

New files only:
1. `shared/control-protocol/v1/fixtures/work-item-id.json` — golden fixture
2. `src-tauri/crates/altai-protocol/tests/control_fixture_cal.rs` — Rust round-trip test
3. `packages/agent-protocol/src/control-fixture-cal.ts` — TS type + validator
4. `packages/agent-protocol/src/__tests__/control-fixture-cal.test.ts` — TS round-trip test

No modifications to existing source files.

## Required negative tests

- Rust: deserializing `{"type": "work_item_id", "value": ""}` returns a typed
  `Err`, not a panic.
- Rust: deserializing `{"type": "work_item_id", "value": "no-prefix"}` returns
  a typed `Err`.
- TypeScript: parsing the same invalid values throws a typed error, not a
  generic `Error`.

## Compatibility and deletion effects

None — new files only, no changes to existing protocol or fixtures.

## Output/handoff format

Return the standard handoff report. Include the exact `cargo test` and `pnpm test`
output proving round-trip success and negative-test rejection.
