# Control-Plane Decision Index

> **Purpose:** Record accepted decisions and superseded conflicts. Every task
> reviewer checks this before accepting a PR. The parent plan is authoritative;
> this file records clarifications that the plan delegates to implementation.

## Accepted Decisions

| ID | Date | Decision | Affects | Status |
| --- | --- | --- | --- | --- |
| DEC-001 | 2026-08-13 | ALTAI adopts Paperclip through an organization-owned downstream and narrow ALTAI adapter boundaries; upstream history, license and base SHA are preserved. | M2, all CP modules | supersedes 2026-08-03 native-only decision |
| DEC-002 | 2026-08-13 | The global control plane uses Postgres in deployed environments or PGlite for an embedded local control DB. Workspace-local `work.db` remains an execution ledger; explicit field authority prevents split-brain. | M2–M3 | supersedes 2026-08-03 single SQLite control DB decision |
| DEC-003 | 2026-08-03 | Work status and execution phase are stored as two separate axes. | CP-03 | accepted (parent plan §5.1–5.2) |
| DEC-004 | 2026-08-03 | IsanAgent `cron` tool is retained; schedule backend is host-selected per attempt. | CP-08 | accepted (parent plan §3.7) |
| DEC-005 | 2026-08-13 | The control plane exposes authenticated, versioned local and deployed transports. Transport selection is capability-negotiated; no unauthenticated listener is permitted. | M2, daemon lifecycle | supersedes local-socket-only decision |
| DEC-006 | 2026-08-13 | Two persistence planes: global control DB plus workspace-local execution ledger. Run journals are not consolidated; durable inbox/outbox/cursor synchronization is required. | M2–M3 | amended |
| DEC-007 | 2026-08-03 | AgentProfile is reusable config; AgentInstance is a durable worker identity. One profile can back many instances. An active attempt keeps its immutable profile revision. | CP-05 | accepted (parent plan §4.3) |
| DEC-008 | 2026-08-03 | Legacy `failed` assignment status maps to work_status `in_progress` + execution_phase `failed` (not `needs_attention`, which is a phase, not a Section 5.1 status). Attention derives from the phase via the Inbox projection. | GLM-CAL-03, CP-20 | accepted (user decision, GLM-CAL-03 packet amendment) |
| DEC-009 | 2026-08-13 | ADR 0003 codifies a global control-plane authority plus a local execution ledger. IsanAgent remains the execution runtime and owns no project-management state. | M2, all CP modules | amended |

## Superseded / Conflicting Documents

| Document | Conflict | Resolution | Effective |
| --- | --- | --- | --- |
| `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` ownership sections (§2.2, §4.1) | Owner placed in a Tauri-resident orchestration service, conflicting with the parent plan's single control-plane owner | Parent plan wins; §2.2 and §4.1 marked superseded inline by CP-00-01 (2026-08-03) | Effective 2026-08-03 (CP-00-01) |
| ADR 0001 `Lifecycle and ownership` | States "Work that must outlive VS Code requires a separately designed daemon and is out of scope for v1" | Control-plane daemon makes durable work outlive any renderer. Amended by CP-00-01: see ADR 0001 `Amendment 2026-08-03: Control-plane scope`. | Effective 2026-08-03 (CP-00-01) |
| ADR 0002 `Decision` | Protocol covers run control only | Protocol will carry control-plane domains in addition to run control. Amended by CP-00-01: see ADR 0002 `Amendment 2026-08-03: Control-plane domains`. | Effective 2026-08-03 (CP-00-01) |
| ADR 0003 2026-08-11 amendment | Prohibits a separate control-plane daemon and global control store | Superseded in part by ADR 0003 `Superseding amendment (2026-08-13): federated control plane`. M1 local Work semantics remain valid. | Effective 2026-08-13 |

## Pending Decisions (require task packet + reviewer acceptance)

| Topic | Context | Needed by |
| --- | --- | --- |
| Exact Rust crate dependency rules for `altai-control-plane` ↔ `altai-agent-service` | CP-00 architecture fence | CP-00-02 |
| Whether control DB path is per-user or per-installation | CP-02 design | CP-02-01 |
| Typed error code namespace for control-plane RPC | CP-01 / CP-15 | CP-01-02 |
