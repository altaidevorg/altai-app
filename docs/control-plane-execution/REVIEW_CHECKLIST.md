# Control-Plane Review Checklist

> **Purpose:** Tier-based acceptance gates for every control-plane task.
> Reviewers use this checklist; implementers use it for mandatory self-review
> before handoff (playbook §8 Step 4).

## Risk Tier Definitions

### Tier A — Bounded and locally provable

**Examples:** IDs/serialization types, golden fixtures, read-only projections,
component rendering against mocks, documentation, pure mapping functions.

**Limits:**
- normally 1–5 edited files;
- normally under 500 net new/changed lines;
- targeted tests plus type/build checks;
- standard independent code review.

### Tier B — Stateful but isolated

**Examples:** SQLite migrations/repositories, idempotency records, Routine
command port, IsanAgent attempt adapter, cron-to-Routine bridge, legacy import
adapters, GitHub outbox/inbox adapter, one Operations UI slice.

**Limits:**
- normally under eight edited files or split the task;
- property, integration, restart, and failure-path tests as applicable;
- independent architecture review required;
- GLM self-review cannot accept its own implementation.

### Tier C — Safety- or ownership-critical

**Examples:** atomic checkout and leases, scheduler claim/coalescing, budget
reservation and hard stops, approval authorization, daemon
single-instance/authentication, workspace apply/finalize and destructive
cleanup, plugin capability/security enforcement, single-writer cutover and
physical legacy deletion.

**Acceptance requires:**
- an independent reviewer session or human reviewer;
- invariant/property tests written before or alongside implementation;
- crash/race/adversarial scenarios;
- narrow diff and explicit rollback;
- no release based only on model-reported success.

---

## Self-Review Questions (mandatory before handoff)

Before declaring a task complete, the implementer must answer all of these
against the **complete diff**:

- [ ] Did I create a second owner, scheduler, store, or identity?
- [ ] Did I derive Work status from session/run state?
- [ ] Did I bypass expected revision, idempotency, policy, or audit?
- [ ] Did I make an ALTAI-managed tool accept model-supplied scope as trusted
      authority?
- [ ] Did I accidentally change standalone IsanAgent behavior?
- [ ] Did I leave a compatibility path capable of mutation after cutover?
- [ ] Are new public contracts represented in fixtures and tests?
- [ ] Are deletions gated by reachability and migration checks?

If any answer reveals a violation, the implementer must fix it or stop and report.

---

## Reviewer Review Order

The reviewer receives the original packet, preflight, complete diff, and test
evidence. They do not receive only the implementer's summary.

Review in this order:

### 1. Scope and dirty-worktree preservation
- [ ] The diff touches only `allowed_files` from the packet.
- [ ] No unrelated user changes were modified or reverted.
- [ ] No neighboring refactors outside scope were added.

### 2. Ownership and identity invariants
- [ ] No second owner/scheduler/store was created.
- [ ] All canonical IDs are distinct and explicitly mapped in bridges.
- [ ] No identity is inferred from title, chat name, GitHub number, or path.

### 3. Schema/protocol compatibility
- [ ] Migrations are replay-safe and reject newer schemas.
- [ ] Schema version is unambiguous.
- [ ] Protocol changes maintain backward compatibility or negotiate explicitly.

### 4. Concurrency/idempotency/crash behavior
- [ ] Concurrent operations create at most one active attempt per exclusive work.
- [ ] Idempotency keys prevent duplicate effects on retry.
- [ ] Crash recovery does not produce duplicate child work or delivery.

### 5. Security and trusted-context boundaries
- [ ] No provider keys or secrets in events/logs/UI/protocol.
- [ ] Model-supplied scope is never treated as trusted authority.
- [ ] Cross-organization access fails closed.

### 6. Error and rollback paths
- [ ] Errors are typed, not string-matching.
- [ ] Stale revisions, unknown IDs, and invalid states return typed errors.
- [ ] Rollback does not require dual-write or two schedulers.

### 7. Tests that would fail under old behavior
- [ ] Positive tests prove the intended new behavior.
- [ ] Negative tests prove invalid inputs are rejected.
- [ ] No test is a no-op or always-pass placeholder.

### 8. Documentation/migration/deletion consequences
- [ ] Updated ADRs are justified by the packet, not accidental implementation.
- [ ] Deletion gates are met or the deletion is deferred.
- [ ] `CURRENT_STATE.md` is ready to be updated on acceptance.

---

## Tier-Specific Additional Gates

### Tier A additional gates
- [ ] Type check passes (`cargo check` / `tsc --noEmit`).
- [ ] Targeted tests pass.
- [ ] If fixtures exist, Rust and TypeScript validate the same fixtures.

### Tier B additional gates
- [ ] All Tier A gates.
- [ ] Integration/property tests cover happy path + failure path.
- [ ] Restart-safe: data survives process restart.
- [ ] Independent architecture reviewer has signed off.

### Tier C additional gates
- [ ] All Tier B gates.
- [ ] Invariant/property tests written before or alongside implementation.
- [ ] Adversarial/race/crash scenarios tested.
- [ ] Explicit rollback plan documented.
- [ ] No release based only on model-reported success.
- [ ] Independent reviewer session (fresh context).

---

## Go/No-Go Gates (program-level)

- [ ] Three accepted calibration tasks before stateful work.
- [ ] Two consecutive clean Tier B tasks before Tier C implementation.
- [ ] Any scheduler/lease/security ownership violation pauses Tier C delegation
      and triggers a prompt/host audit.
- [ ] No physical legacy deletion until replacement, migration, reachability,
      and rollback gates are independently verified.