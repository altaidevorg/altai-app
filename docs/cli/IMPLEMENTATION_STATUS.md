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
| Shared, Tauri-free event journal (`altai-core::journal`) | Verified | moved `event_journal.rs` unit tests now run under `altai-core` |
| `altai run` appends `run_started` / `run_terminated` to the desktop journal | Verified | `journal_sink` unit tests + CLI smoke against a scratch workspace |
| `altai journal summary` / `altai journal fetch` inspection commands | Verified | clap contract tests + round-trip test against a seeded journal |
| Mid-run `/dev/tty` approval resume for `altai run` | Verified | oneshot prompts on controlling TTY; non-TTY still exits 4 |
| Runtime `/key` API key entry (TUI + line mode) | Verified | persist to config.toml + SwitchModel hot-swap; mask suffix only |

## Incomplete

| Capability | Gap | Planned milestone |
|---|---|---|
| Full bus-message parity in the CLI journal sink (tool calls, thinking, usage) | Only `run_started` / `run_terminated` are journaled; desktop's richer per-turn events are not yet mirrored by the CLI | **M6 follow-up** |
| Release packaging / installed-binary CI matrix | Partial | `altai-cli smoke` CI job builds release binary + version/doctor/completion/dry-run; full archives/installers still open |
| Upstream merge of oneshot + theme + approval + attachment host API | Local path pin; upstream PR open | Switch back to git rev after [isanagent#99](https://github.com/altaidevorg/isanagent/pull/99) merges |
| Full PTY visual golden frames | Width-fit string snapshots only | Expand in CI matrix follow-up |

## Blocked

| Blocker | Impact | Resolution path |
|---|---|---|
| IsanAgent upstream PR not yet merged | ALTAI still path-pins `tools/isanagent-oneshot` | Merge [isanagent#99](https://github.com/altaidevorg/isanagent/pull/99), then retarget Cargo deps to the git rev |

## Next milestone

### M6 — Packaging, CI matrix, richer journal parity

Do not start until M5 is committed and journal + `/dev/tty` approval resume above are accepted.
