# Control-Plane Decision Index

> **Purpose:** Record accepted decisions and superseded conflicts. Every task
> reviewer checks this before accepting a PR. The parent plan is authoritative;
> this file records clarifications that the plan delegates to implementation.

## Accepted Decisions

| ID | Date | Decision | Affects | Status |
| --- | --- | --- | --- | --- |
| DEC-001 | 2026-08-03 | ALTAI will not embed, fork, or repackage Paperclip. Control-plane semantics are reimplemented natively. | All CP modules | accepted (parent plan §1) |
| DEC-002 | 2026-08-03 | `altai-control-plane` crate owns the durable SQLite database with WAL mode and one writer. | CP-02 | accepted (parent plan §6.2) |
| DEC-003 | 2026-08-03 | Work status and execution phase are stored as two separate axes. | CP-03 | accepted (parent plan §5.1–5.2) |
| DEC-004 | 2026-08-03 | IsanAgent `cron` tool is retained; schedule backend is host-selected per attempt. | CP-08 | accepted (parent plan §3.7) |
| DEC-005 | 2026-08-03 | Control-plane daemon uses user-scoped domain socket (Unix) / named pipe (Windows). No network listener by default. | CP-16 | accepted (parent plan §6.1) |
| DEC-006 | 2026-08-03 | Two persistence planes: user-scoped control DB + existing workspace/run journals. Run journals are not consolidated in the first migration. | CP-02 | accepted (parent plan §6.2) |
| DEC-007 | 2026-08-03 | AgentProfile is reusable config; AgentInstance is a durable worker identity. One profile can back many instances. An active attempt keeps its immutable profile revision. | CP-05 | accepted (parent plan §4.3) |
| DEC-008 | 2026-08-03 | Legacy `failed` assignment status maps to work_status `in_progress` + execution_phase `failed` (not `needs_attention`, which is a phase, not a Section 5.1 status). Attention derives from the phase via the Inbox projection. | GLM-CAL-03, CP-20 | accepted (user decision, GLM-CAL-03 packet amendment) |
| DEC-009 | 2026-08-03 | Control-plane/execution-plane ownership split codified in ADR 0003: one user-scoped `altai-control-plane` daemon owns all authoritative lifecycle mutations; IsanAgent remains the execution runtime and owns no project-management state. | CP-00, all CP modules | accepted (ADR 0003, parent plan §3.1–3.2) |

## Superseded / Conflicting Documents

| Document | Conflict | Resolution | Effective |
| --- | --- | --- | --- |
| `docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md` ownership sections (§2.2, §4.1) | Owner placed in a Tauri-resident orchestration service, conflicting with the parent plan's single control-plane owner | Parent plan wins; §2.2 and §4.1 marked superseded inline by CP-00-01 (2026-08-03) | Effective 2026-08-03 (CP-00-01) |
| ADR 0001 `Lifecycle and ownership` | States "Work that must outlive VS Code requires a separately designed daemon and is out of scope for v1" | Control-plane daemon makes durable work outlive any renderer. Amended by CP-00-01: see ADR 0001 `Amendment 2026-08-03: Control-plane scope`. | Effective 2026-08-03 (CP-00-01) |
| ADR 0002 `Decision` | Protocol covers run control only | Protocol will carry control-plane domains in addition to run control. Amended by CP-00-01: see ADR 0002 `Amendment 2026-08-03: Control-plane domains`. | Effective 2026-08-03 (CP-00-01) |

## Pending Decisions (require task packet + reviewer acceptance)

| Topic | Context | Needed by |
| --- | --- | --- |
| Exact Rust crate dependency rules for `altai-control-plane` ↔ `altai-agent-service` | CP-00 architecture fence | CP-00-02 |
| Whether control DB path is per-user or per-installation | CP-02 design | CP-02-01 |
| Typed error code namespace for control-plane RPC | CP-01 / CP-15 | CP-01-02 |