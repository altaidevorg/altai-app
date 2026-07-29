# ALTAI CLI Implementation Status

**Date:** 2026-07-29
**Desktop base:** v0.6.4
**IsanAgent dependency:** temporary path pin `tools/isanagent-oneshot`
(based on `8c9eef2` + oneshot host API; see `docs/cli/isanagent-oneshot-api.md`)

## Existing

| Capability | Status | Evidence |
|---|---|---|
| `altai-core` crate (config, event, palette, workspace primitives) | Verified | unit tests pass |
| `altai-cli` crate (clap command tree, doctor, version, completion, config, models) | Verified | 17 unit/smoke tests pass |
| Workspace path resolution and canonicalization | Verified | `altai-core::workspace` tests |
| Configuration precedence | Verified | `altai-core::config` tests |
| Event envelope schema v1 | Verified | `altai-core::event` + JSONL emitter tests |
| `altai agent` host config + supported TUI start | Verified | dry-run + `start_host` path |
| `altai run` real one-shot execution | Verified | scripted-provider smoke + dry-run |
| Output modes pretty / json / jsonl | Verified | clap + renderer tests |
| Ctrl-C / timeout / approval exit codes | Verified | `RunExitCode` mapping tests; runtime select wired |
| Non-TTY default permission `plan` | Verified | `resolve_run_permission` |
| IsanAgent oneshot host API | Verified locally | `run_oneshot` + scripted provider; upstream PR blocked by gh auth |

## Incomplete

| Capability | Gap | Planned milestone |
|---|---|---|
| ALTAI palette wiring to TUI renderer | `--theme dark/light` blocked | **M2** |
| Responsive TUI layout (80/100/120+ cols) | Not implemented | **M2** |
| Typed policy rules, approval cards, diff review | Not implemented | **M3** |
| `@file` fuzzy references, context, compaction | Not implemented | **M4** |
| Multi-agent / desktop event journal parity | Not implemented | **M5** |
| Release packaging / installed-binary CI matrix | Not implemented | **M6** |
| Upstream merge of oneshot host API | Local path pin only | Switch back to git rev after merge |

## Blocked

| Blocker | Impact | Resolution path |
|---|---|---|
| `gh` auth invalid for IsanAgent upstream push | Cannot land oneshot API on `altaidevorg/isanagent` from this environment | Push/PR from an authenticated machine; then remove `tools/isanagent-oneshot` path pin |
| ALTAI runtime events still Tauri-bound | Shared durable event journal incomplete | M5 |

## Next milestone

### M2 — Ship the usable Task Session TUI

Do not start until M1 is committed and the oneshot result above is accepted.
