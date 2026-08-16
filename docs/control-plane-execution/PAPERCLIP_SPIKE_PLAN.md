# Paperclip acceptance-spike plan

> **Objective (charter §4, verbatim):** the Paperclip control plane
> (downstream, base `f0e6c0f`) dispatches one existing ALTAI Work to a
> local host over the 051/052 protocol; reconnect is idempotent
> (020–024 dispatch correctness holds), and the global projection
> reaches Review (045 evidence gates). No upstream becomes
> production-critical merely because its demo runs — the spike passes
> only through the reviewed conformance harness.
>
> **Backlog order:** 080 (PR 3). **Charter:** `PAPERCLIP_DOWNSTREAM_CHARTER.md`.

## 1. What runs where

```text
┌────────────────────────────────────────┐        ┌──────────────────────────────┐
│ Paperclip plane (downstream, local)    │        │ ALTAI host (this repository) │
│ server/src Express API + PGlite        │        │ authenticated host process   │
│ packages/adapters/altai-host  ─────────┼───────▶│ /v1/hosts/register           │
│   (named package, charter §2.4)        │ wakes/ │ wake claim → checkout        │
│ environment-run-orchestrator dispatch  │ events │ IsanAgent attempt (034)      │
│ activity/projections (adopt, §3)       │◀───────┤ event report → Review (045)  │
└────────────────────────────────────────┘        └──────────────────────────────┘
```

- **The plane is the downstream** at base `f0e6c0f`: Paperclip's graph,
  orchestrator, activity and projections — the charter §3 *adopt* rows.
- **The bridge is a named package**, `packages/adapters/altai-host` in
  the downstream: it satisfies Paperclip's adapter contract
  (`server/src/adapters/registry.ts` + `builtin-adapter-types.ts` —
  the same seam `claude-local` and `codex-local` ride), and its
  execution side speaks ALTAI's authenticated transport instead of a
  local CLI. This is the charter's "replacing its execution adapters
  with ALTAI host/IsanAgent adapters over a versioned protocol," made
  concrete.
- **The host is this repository's contracts, nothing new:** host
  registration (003), the wake/checkout transport (020–024:
  `POST /v1/wakes`, `/v1/wakes/{work_item_id}/claim`,
  `/v1/work-checkouts`, `/v1/work-checkouts/release`), attempt
  execution (030–034), event translation and finalization (035),
  review gates (045). The protocol handshake is 051's
  `POST /v1/protocol/negotiate` + `/v1/protocol/commands`
  (major-version gate first, then dispatch).

## 2. The exercise, step by step

1. **Bring-up.** Downstream server runs locally on PGlite (upstream
   `docker/` and `packages/db` provide the path); the ALTAI host
   process starts with a registration grant. Everything stays on
   localhost — charter §2.8's review is still unpassed.
2. **Work exists first.** One ALTAI Work item exists (the spike's
   fixture) with an eligible agent, no blockers, budget available
   (022 eligibility).
3. **Dispatch.** The altai-host adapter enqueues a wake for the Work
   (`POST /v1/wakes`); coalescing is the plane's (020).
4. **Checkout.** The host claims the wake and checks the Work out
   transactionally — exactly one live checkout (020–021).
5. **Execute.** The bound attempt runs through IsanAgent (034); events
   are reported back and translated (035) — run completion signals
   verification, it never directly completes Work.
6. **Review.** The projection reaches Review with evidence attached
   (045).
7. **Reconnect.** The host connection drops mid-attempt and
   re-registers: the wake does not double-fire, the lease is not
   stolen, a stale finalizer cannot release another attempt's lease
   (021), and the checkout reattaches idempotently (023 recovery).

## 3. The harness (the only way to pass)

- **Shape:** an `altai-cli` subcommand (`paperclip-spike`) in this
  repository — not a script, not a hand-run demo. It starts/points at
  the downstream plane, provisions the fixture, drives steps 2–7, and
  exits non-zero with a typed failure if any assertion breaks. CI can
  adopt it once a Paperclip service can run there; until then the pass
  evidence is the harness output recorded in the downstream's task
  record and the 080 acceptance row.
- **Idempotency assertions are part of the harness**, not prose: the
  reconnect step (7) is executed by the harness itself, twice, with
  the lease state asserted between runs.

## 4. Milestones (remaining 080 PRs)

| PR | Delivers |
| --- | --- |
| CP-08-83 (080 PR 4) | Downstream `packages/adapters/altai-host` skeleton: registers with Paperclip's adapter registry, declares capabilities, no execution. Runs upstream tests. |
| CP-08-84 (080 PR 5) | `altai-cli paperclip-spike` harness: registration, fixture, dispatch, checkout, reconnect-idempotency assertions against a stub plane. |
| CP-08-85 (080 PR 6) | End-to-end: real downstream plane, IsanAgent attempt, event translation, Review projection; spike evidence recorded; 080 acceptance. |

Each lands green independently; the spike's pass is declared only when
PR 6's harness run is green against the real downstream.

## 5. Non-goals

- No execution-adapter or storage adoption beyond the bridge package
  (charter §3 *replace* rows unchanged).
- No exposure beyond localhost; no sync of the upstream base (still
  `f0e6c0f`).
- No product UI for the spike; the projection check reads the plane's
  API, not a rendered surface.
