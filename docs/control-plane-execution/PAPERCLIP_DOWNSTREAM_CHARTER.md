# Paperclip Downstream Charter

> **Authority:** DEC-001 (2026-08-13) — "ALTAI adopts Paperclip through an
> organization-owned downstream and narrow ALTAI adapter boundaries;
> upstream history, license and base SHA are preserved." This supersedes
> §1 of `PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md` (the
> 2026-08-03 native-only decision) for adoption policy; that plan remains
> the architecture authority. Parent input:
> `altai-agent-work-os/UPSTREAM_ADOPTION.md`. Backlog order: 080.

## 1. Upstream identity

| Field | Value |
| --- | --- |
| Upstream | `github.com/paperclipai/paperclip` |
| License | MIT (preserved verbatim in the downstream) |
| Base SHA | `f0e6c0f` (2026-08-13, verified: "feat(server): receive and apply the Paperclip Cloud onboarding seed (#11098)") |
| Default branch | `master` |

The base SHA is the charter's anchor, not a forever pin: the downstream
records its own current base in `UPSTREAM_BASE.md` and advances it only
through a reviewed sync PR.

## 2. Downstream policy (binding for every change)

1. **Organization-owned downstream** (`altaidevorg/paperclip`), private
   until the first security review passes, with an `upstream` remote
   pointing at the upstream repository.
2. **Preserve license, notices, and full source history.** No squashed
   imports, no history rewrites, no stripped attributions.
3. **`UPSTREAM_BASE.md` at the root records the upstream base SHA** and
   the date of the last reviewed sync.
4. **ALTAI-specific changes live in named packages/adapters**, not
   scattered edits inside adopted modules; anything upstream can absorb
   goes upstream as a PR first.
5. **Sync through reviewed PRs only** — never a moving branch, never a
   force-push over adopted history.
6. **Upstream tests plus ALTAI contract/conformance tests** must pass
   before a downstream change merges.
7. **SBOM and provenance list** for any release artifact built from the
   downstream.
8. **Dependency, secret, auth-boundary, and remote-code-execution
   review before exposing any downstream service beyond localhost.**

## 3. Module selection through adapter boundaries

"Adopt" means: run as the deployed control-plane reference behind an
ALTAI-owned contract that already exists in this repository. The
boundary is always a versioned, tested contract — never a direct call
into Paperclip internals. The control-plane contracts below are the
051/052 protocol line already shipped.

| Paperclip module (server/src, base `f0e6c0f`) | ALTAI contract boundary | Disposition |
| --- | --- | --- |
| Global PM graph: `companies.ts`, `projects.ts`, `goals.ts`, `issues.ts`, `agents.ts`, `company-member-roles.ts` | 010–012 Organization/Goal/Project/Work typed identities; 030 Attempt/RunBinding | **Adopt as deployed reference** — conformance target for the versioned protocol; ALTAI adds Ticket, Campaign, canonical Work links, context capsules, local-host leases on its side of the boundary |
| Approvals & decisions: `approvals.ts`, `decisions.ts`, `decision-signing.ts`, `decision-queues.ts`, `issue-approvals.ts` | 042 approval/governance contracts (decisions bind scope + payload revision; immutable audit) | **Adopt** |
| Budgets & cost: `budgets.ts`, `costs.ts`, `finance.ts` | 043 usage/cost ledger and budget hard-stops | **Adopt** |
| Activity & events: `activity.ts`, `activity-log.ts`, `live-events.ts` | 060 activity stream / server-side projections | **Adopt** |
| Plugin worker patterns: `plugin-worker-manager.ts`, `plugin-runtime-sandbox.ts`, `plugin-job-{coordinator,scheduler,store}.ts`, `plugin-secrets-handler.ts`, `plugin-registry.ts`, `plugin-manifest-validator.ts`, `plugin-capability-validator.ts` | 071–073 manifest/capability contracts, out-of-process workers, schema-driven UI | **Adopt** — the closest upstream analog to what 071–073 already shipped; upstream patterns inform the deployed-plane counterparts |
| External objects: `external-objects.ts`, `github-external-object-provider.ts`, `github-fetch.ts` | 070/074 ExternalObject contracts (idempotent sync, explicit authority, account scope) | **Adopt** — direct precedent already mirrored natively in 070/074 |
| Operations projections & UI (`ui/`, `dashboard.ts`, `attention.ts`, `inbox-*.ts`) | 060–068 read-model projections and surfaces | **Reference only** — informs projections; no UI code enters the desktop app (different runtime, different product) |
| Execution adapters: `environment-*`, `execution-*`, `local-service-supervisor.ts` | 053 host adapters; IsanAgent runtime | **Replace** — explicitly not adopted; ALTAI host/IsanAgent adapters ride the versioned protocol instead |
| Storage: `packages/db`, PostgreSQL/PGlite layer | 005 local SQLite consolidation (`work.db`) | **Replace on the desktop**; the deployed plane may keep PostgreSQL behind the same contracts |

## 4. Acceptance spike (080's exit gate)

From the adoption plan, made concrete against shipped ALTAI contracts:

> The Paperclip control plane (downstream, base `f0e6c0f`) dispatches
> one existing ALTAI Work to a local host over the 051/052 protocol;
> reconnect is idempotent (020–024 dispatch correctness holds), and the
> global projection reaches Review (045 evidence gates).

"No upstream becomes production-critical merely because its demo runs":
the spike passes only through the reviewed conformance harness, not a
hand-run demo.

## 5. What this charter does not authorize

- No Paperclip source enters `altai-app` or any Apache-2.0 ALTAI
  artifact from this charter alone — adoption happens in the downstream
  repository, integrated through versioned contracts.
- No execution-adapter or storage replacement work; those rows above
  are **Replace**, and replacements are separate PRs per the operating
  model.
- Nothing in this charter unblocks 082 (Macro, AGPL-3.0): its legal
  gate is independent and unchanged.

## 6. Stand-up record

CP-08-81 stood the downstream up per §2 (2026-08-16):
[`altaidevorg/paperclip`](https://github.com/altaidevorg/paperclip)
(private), `master` = upstream base `f0e6c0f` plus root policy files
only (`UPSTREAM_BASE.md`, `DOWNSTREAM.md`, `SECURITY_REVIEW.md` —
commit `e7536aa9`); full history preserved (3,597 commits, 1,155 tags
reachable from the base); `upstream` remote set; the §2.8 security
review is recorded as **unpassed** — nothing is exposed beyond
localhost until it is.

Next: the §4 acceptance spike, through the reviewed conformance
harness.
