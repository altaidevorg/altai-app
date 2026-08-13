# Control-Plane Decision Index

> **Purpose:** Record accepted decisions and superseded conflicts. Every task
> reviewer checks this before accepting a PR. The parent plan is authoritative;
> this file records clarifications that the plan delegates to implementation.

## Accepted Decisions

| ID | Date | Decision | Affects | Status |
| --- | --- | --- | --- | --- |
| DEC-001 | 2026-08-13 | ALTAI adopts Paperclip through an organization-owned downstream and narrow ALTAI adapter boundaries; upstream history, license and base SHA are preserved. | M2, all CP modules | supersedes 2026-08-03 native-only decision |
| DEC-002 | 2026-08-13 | Desktop Work OS uses the existing workspace-local SQLite `work.db` as its single durable store. “Control plane” is an ownership boundary, not a second database or a user-managed service. A future multi-machine product, if accepted, uses an ALTAI-managed backend/API; Postgres is then backend infrastructure, never a desktop prerequisite. | all CP modules | supersedes federated Postgres/PGlite decision |
| DEC-003 | 2026-08-03 | Work status and execution phase are stored as two separate axes. | CP-03 | accepted (parent plan §5.1–5.2) |
| DEC-004 | 2026-08-03 | IsanAgent `cron` tool is retained; schedule backend is host-selected per attempt. | CP-08 | accepted (parent plan §3.7) |
| DEC-005 | 2026-08-13 | The control plane exposes authenticated, versioned local and deployed transports. Transport selection is capability-negotiated; no unauthenticated listener is permitted. | M2, daemon lifecycle | supersedes local-socket-only decision |
| DEC-006 | 2026-08-13 | `work.db` is the single local durable Work OS store; the existing agent event journal remains a specialized append-only execution record. No synchronization plane exists inside one desktop workspace. | M2–M3 | supersedes two-persistence-plane decision |
| DEC-007 | 2026-08-03 | AgentProfile is reusable config; AgentInstance is a durable worker identity. One profile can back many instances. An active attempt keeps its immutable profile revision. | CP-05 | accepted (parent plan §4.3) |
| DEC-008 | 2026-08-03 | Legacy `failed` assignment status maps to work_status `in_progress` + execution_phase `failed` (not `needs_attention`, which is a phase, not a Section 5.1 status). Attention derives from the phase via the Inbox projection. | CAL-03, CP-20 | accepted (user decision, CAL-03 packet amendment) |
| DEC-009 | 2026-08-13 | ADR 0003 codifies one local SQLite Work OS authority. IsanAgent remains the execution runtime and owns no project-management state. | all CP modules | supersedes federated deployment amendment |

## Superseded / Conflicting Documents

| Document | Conflict | Resolution | Effective |
| --- | --- | --- | --- |
| `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` ownership sections (§2.2, §4.1) | Owner placed in a Tauri-resident orchestration service, conflicting with the parent plan's single control-plane owner | Parent plan wins; §2.2 and §4.1 marked superseded inline by CP-00-01 (2026-08-03) | Effective 2026-08-03 (CP-00-01) |
| ADR 0001 `Lifecycle and ownership` | States "Work that must outlive VS Code requires a separately designed daemon and is out of scope for v1" | Control-plane daemon makes durable work outlive any renderer. Amended by CP-00-01: see ADR 0001 `Amendment 2026-08-03: Control-plane scope`. | Effective 2026-08-03 (CP-00-01) |
| ADR 0002 `Decision` | Protocol covers run control only | Protocol will carry control-plane domains in addition to run control. Amended by CP-00-01: see ADR 0002 `Amendment 2026-08-03: Control-plane domains`. | Effective 2026-08-03 (CP-00-01) |
| ADR 0003 2026-08-13 federated amendment | Introduced a separate daemon and global Postgres/PGlite control store | Superseded by the local SQLite decision above. The 2026-08-11 local-first deployment boundary applies again. | Effective 2026-08-13 |

## Pending Decisions (require task packet + reviewer acceptance)

| Topic | Context | Needed by |
| --- | --- | --- |
| Exact Rust crate dependency rules for `altai-control-plane` ↔ `altai-agent-service` | CP-00 architecture fence | CP-00-02 |
| Managed remote backend tenancy model | Only needed if a separately authorized multi-machine product begins | remote deployment discovery |
| Typed error code namespace for control-plane RPC | CP-01 / CP-15 | CP-01-02 |
