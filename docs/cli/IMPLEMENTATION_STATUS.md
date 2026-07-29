# ALTAI CLI Implementation Status

**Date:** 2026-07-29
**Desktop base:** v0.6.4
**IsanAgent dependency:** temporary path pin `tools/isanagent-oneshot`
(based on `8c9eef2` + oneshot host API; see `docs/cli/isanagent-oneshot-api.md`)

## Existing

| Capability | Status | Evidence |
|---|---|---|
| `altai-core` crate (config, event, palette, workspace, policy, compaction) | Verified | unit tests pass |
| `altai-cli` crate (clap command tree, doctor, version, completion, config, models) | Verified | unit/smoke tests pass |
| Workspace path resolution and canonicalization | Verified | `altai-core::workspace` tests |
| Configuration precedence | Verified | `altai-core::config` tests |
| Event envelope schema v1 | Verified | `altai-core::event` + JSONL emitter tests |
| `altai agent` host config + supported TUI start | Verified | dry-run + `start_host` path |
| ALTAI terminal theme (dark/light/auto/no-color) | Verified | palette resolve + host `theme` + TUI Theme roles |
| Responsive layout (80 / 100 / 120+) | Verified | `LayoutDensity` + wide secondary split + width-fit tests |
| Dense status header (workspace/model/permission/session) | Verified | title/status width-fit snapshots |
| `--no-tui` line mode with status header | Verified | labeled assistant/tool/clarification lines |
| `altai run` real one-shot execution | Verified | scripted-provider smoke + dry-run |
| Output modes pretty / json / jsonl | Verified | clap + renderer tests |
| Ctrl-C / timeout / approval exit codes | Verified | `RunExitCode` mapping tests; runtime select wired |
| Non-TTY default permission `plan` | Verified | `resolve_run_permission` |
| IsanAgent oneshot host API | Verified locally | `run_oneshot` + scripted provider; upstream PR blocked by gh auth |
| Four-way approval replies (approve/deny/always/abort) | Verified | `classify_approval_reply` + grant cache |
| Edit-diff TUI / line-mode rendering | Verified | `EditDiffPayload` + `parse_diff_lines` + line-mode `[edit_diff]` |
| JSONL `clarification_requested` + `edit_diff` | Verified | `JsonlEmitter` unit test |
| Plan mode parity (shell ask / edit deny) | Verified | host mapping + `altai_core::policy` |
| Compaction prefs bridge (`--no-auto-compact`, `--compact-threshold`, `--compact-tail`) | Verified | `altai_core::compaction` + host `compact_*` + clap dry-run |
| Line-mode `/context` + `/compact [focus]` | Verified | `terminal.rs` memory GetContext / TriggerCompaction |
| Real `--file` content loading (oneshot + first line-mode message) | Verified | `load_host_file_attachments` |
| `@path` text / image / PDF + fuzzy basename resolve | Verified | `attachments.rs` unit tests |

## Incomplete

| Capability | Gap | Planned milestone |
|---|---|---|
| Mid-run `/dev/tty` resume for `altai run` | Oneshot still exits 4 on approval | **M3 follow-up / M6** |
| Multi-agent / desktop event journal parity | Not implemented | **M5** |
| Release packaging / installed-binary CI matrix | Not implemented | **M6** |
| Upstream merge of oneshot + theme + approval + attachment host API | Local path pin only | Switch back to git rev after merge |
| Full PTY visual golden frames | Width-fit string snapshots only | Expand in M6 CI matrix |

## Blocked

| Blocker | Impact | Resolution path |
|---|---|---|
| `gh` auth invalid for IsanAgent upstream push | Cannot land oneshot/theme API on `altaidevorg/isanagent` from this environment | Push/PR from an authenticated machine; then remove `tools/isanagent-oneshot` path pin |
| ALTAI runtime events still Tauri-bound | Shared durable event journal incomplete | M5 |

## Next milestone

### M5 — Multi-agent / event journal parity

Do not start until M4 is committed and context/attachment/compaction behavior above is accepted.
