# Upstream adoption register

Status: active
Last reviewed: 2026-08-13

This register applies the canonical [ALTAI Work OS adoption
plan](https://github.com/altaidevorg/altai-agent-work-os/blob/main/UPSTREAM_ADOPTION.md)
to `altai-app`. No upstream source may be copied into this repository until a
downstream record, license review and security review are complete.

| Upstream | Research revision | License | Intended ALTAI boundary | Initial gate |
| --- | --- | --- | --- | --- |
| [Paperclip](https://github.com/paperclipai/paperclip) | `f0e6c0f` | MIT | `altai-control-plane` downstream, global graph/adapters/projections | one existing Work dispatches idempotently to a local ALTAI host |
| [LongHorizon-Harness](https://github.com/AMAP-ML/LongHorizon-Harness) | `53bc678` / v0.1.4 | MIT | supervised Manager/Executor/Auditor worker companion | verified checkpoint becomes native Attempt/Review evidence |
| [OpenTag](https://github.com/fancyboi999/open-tag) | `6042bf2` | Apache-2.0 | channels, rooms, persistent agent teammates and attachments | channel mention creates a linked Ticket/Task |
| [qm](https://github.com/yc-software/qm) | `d719f54` | MIT | scope, memory, sandbox, keychain, cron and app substrate | person/room scope proves restrictive inheritance and audit |
| [Macro](https://github.com/macro-inc/macro) | `5c6b242` | AGPL-3.0 | Canvas/mail/docs/CRDT/mobile only via approved boundary | legal path selected before any source/code import |

## Required downstream record

Before adding an upstream as a submodule, vendored package, service dependency
or companion binary, create an organization-owned downstream that retains its
license and history and contains:

- `UPSTREAM_BASE.md` with repository URL, immutable base SHA, license, review
  date, ALTAI owner and sync cadence;
- a complete NOTICE/provenance entry and release SBOM input;
- an adapter contract owned by ALTAI, plus upstream and conformance tests;
- dependency, secret, authorization-boundary and remote-code-execution review.

Updates are reviewed sync PRs. Moving `main` branches and unreviewed upstream
archives are not release inputs.

## Macro licensing hard stop

`altai-app` is Apache-2.0. Macro source cannot enter an Apache artifact unless
legal records a compatible commercial license. The only alternate route is an
independently buildable, separately distributed AGPL companion with complete
corresponding source and a reviewed versioned network boundary. Until one path
is accepted, implement compatible interfaces without Macro code.

## M2 implementation order

1. Paperclip downstream provenance and license review.
2. Versioned authenticated control-plane health and host registration skeleton.
3. Explicit field-authority schema and local `work.db` outbox/inbox design.
4. Paperclip-to-ALTAI Work dispatch spike.
5. LongHorizon, OpenTag, qm and Macro gates in roadmap order.
