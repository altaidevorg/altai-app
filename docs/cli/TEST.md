# ALTAI CLI Test Plan

**Status:** Initial plan — results are appended as phases complete.

## Planned test inventory

| Area | Planned coverage |
|---|---|
| `altai-core` | Config precedence, event schema, palette manifest, secret redaction, workspace leases |
| `altai-cli` | Argument parsing, help snapshots, desktop routing, output/exit-code contracts |
| IsanAgent adapter | Session resume, runtime events, approvals, cancellation, compaction, model switching |
| TUI | PTY interaction, 80/100/160-column snapshots, truecolor/ANSI/no-color modes |
| Integration | Real temporary workspace, installed binary, durable state sharing, concurrent ownership |
| Packaging | macOS, Linux, and Windows installed-command smoke tests |

## Required workflows

1. Interactive edit with an approval, followed by desktop session resume.
2. One-shot JSONL run with stdout-only machine output and deterministic exit
   code.
3. Model switch, fallback failure, cancellation, and timeout.
4. Background job promotion, restart, inspection, and cancellation.
5. Skill installation, MCP probe, checkpoint restore dry-run, and automation
   lifecycle.
6. Orchestration lifecycle from readiness scan through stop/recovery.
7. Narrow terminal, no-color terminal, and screen-reader-compatible line mode.

## Evidence rules

- Tests invoke the installed command, not only Rust library functions.
- E2E tests use the real ALTAI/IsanAgent backend and persist evidence paths.
- Snapshot updates require review of visual intent, not a blanket re-record.
- Secret values must be absent from stdout, stderr, JSONL, logs, and support
  bundles.

## Initial results — 2026-07-29

- `cargo test --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli`
  passed: 23 tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli --
  --check` passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli
  -- -D warnings` passed.
- `cargo check --manifest-path src-tauri/Cargo.toml --workspace --quiet` passed.
- CLI smoke checks passed for `doctor --json`, `version --verbose`, Bash
  completion generation, `open --dry-run`, and the declared `agent` contract's
  host-unavailable exit code (`10`).
- `agent --dry-run` and `run --dry-run` now emit resolved workspace/state,
  model, permissions, theme, and output-contract previews without creating
  IsanAgent state or starting a host process. Relative file targets resolve to
  their parent workspace.
- `config path --json` reports the project-local ALTAI and IsanAgent
  configuration locations through the same workspace resolution contract.
- `config list --resolved --show-origin --json` resolves non-secret model,
  fallback, provider, and base URL fields from native IsanAgent `[provider]`,
  ALTAI `[agent]`, and `ALTAI_*` environment layers. Smoke coverage confirms
  environment values win and origin labels remain machine-readable.
- `models current --show-origin --json` reuses that resolved configuration
  contract for primary and fallback model inspection.
- The IsanAgent host-extraction patch is recorded at
  `docs/cli/isanagent-host-api.patch` and published as
  IsanAgent `main` merge commit `8c9eef2` via
  [`altaidevorg/isanagent#98`](https://github.com/altaidevorg/isanagent/pull/98).
  Its isolated worktree passed `cargo check --bin isanagent` and a public
  `HostConfig` unit tests; lifecycle shutdown, state/sandbox separation, and
  model/permission override mapping are covered by isolated host tests. ALTAI
  pins that commit and maps project root to the host sandbox while keeping
  state under `.isanagent`.
- `altai-cli agent` now starts the reusable IsanAgent host for the supported
  TUI path. `--model` is applied as a provider/model runtime override and
  `--permission` maps to the host shell/edit policy (`ask`, `auto-edit`,
  `plan`, or `bypass`); the remaining adapter flags continue to fail explicitly
  rather than being ignored. `agent --dry-run` exposes the resolved host
  state/config/sandbox mapping.

## M1 results — 2026-07-29

- Added a narrow IsanAgent oneshot host API (`run_oneshot`, `oneshot_prompt`,
  `observe_tx`, `scripted_responses`, headless `altai-cli` channel). Temporarily
  later merged upstream via
  [`altaidevorg/isanagent#101`](https://github.com/altaidevorg/isanagent/pull/101);
  see `docs/cli/isanagent-oneshot-api.md`.
- `altai run` now performs a real one-shot host session (pretty / json / jsonl,
  Ctrl-C → exit 7, timeout → exit 8, non-TTY approval/clarification → exit 4).
- Non-interactive runs default to `--permission plan` when no permission flag is
  set.
- `cargo test --manifest-path src-tauri/Cargo.toml -p altai-cli` passed: 17
  tests, including `oneshot_smoke_completes_with_scripted_provider`.
- `cargo run -q -p altai-cli -- run . --prompt "summarize this project" --dry-run`
  still emits the resolved preview JSON.

## M2 results — 2026-07-29

- Wired ALTAI terminal palette roles (truecolor RGB derived from
  `src/styles/globals.css`) through IsanAgent `Theme` + `HostConfig.theme`.
- `altai agent --theme auto|dark|light|no-color` is unblocked; `NO_COLOR` and
  `ALTAI_TUI_THEME` resolve via `altai_core::resolve_terminal_appearance*`.
- Dense status header shows `ALTAI · workspace · model · permission · session`.
- Responsive layout: narrow (&lt;80), medium (80–119), wide (120+) with transcript
  + secondary pane split when a side pane is focused.
- `--no-tui` line mode prints a status banner and labeled outbound lines.
- Width-fit snapshot coverage at 80/100/160 columns (string snapshots; full PTY
  golden frames deferred).
-   `cargo test -p altai-core -p altai-cli` → 35 tests; IsanAgent `width_fit` +
  `theme` + oneshot host tests pass against the (then) path pin.

## M3 results — 2026-07-29

- Approval replies accept `approve` / `deny` / `always` (this process) / `abort`;
  session grant cache skips repeat prompts for the same shell/edit key.
- Edit approvals carry unified diffs into the TUI transcript and line mode;
  hotkeys `y/n/a/x` (and `1–4`) submit when an approval is pending.
- JSONL emits `edit_diff` + `clarification_requested` (in addition to shell
  `approval_requested`). Edit oneshot outcomes map to `ApprovalRequired`.
- Plan mode parity with desktop: shell `ask`, edit `deny`
  (`altai_core::policy` + host mapping).
- `/dev/tty` helper added for future interactive resume; oneshot still exits `4`
  when approval is required without a live TUI.
- `cargo test -p altai-core -p altai-cli` → 38 tests; IsanAgent approval/diff/
  policy unit tests pass.

## M4 results — 2026-07-29

- `altai-core::compaction` resolves `--no-auto-compact` / `--compact-threshold` /
  `--compact-tail` (plus `ALTAI_DISABLE_AUTOCOMPACT` / `ALTAI_COMPACT_*` env) into
  IsanAgent `HostConfig.compact_*` / `AgentLogicParams`.
- Line mode supports `/context`, `/compact [focus]`, and `@path` attachments
  (text / image / PDF) with fuzzy basename resolve under the sandbox.
- `altai run --file` and oneshot loads real file content via
  `load_host_file_attachments` (not path-only notes). Line mode merges `--file`
  attachments into the first user message; TUI still seeds `@path` into the
  composer (parsed on send).
- Dry-run for `agent` / `run` includes a `compaction` preview object.
- Focused tests: `altai-core` compaction, attachment unit tests, clap compaction
  flag parse + preview.
- `cargo test -p altai-core -p altai-cli` → 42 tests; IsanAgent
  `channels::terminal_ui::attachments` → 4 tests.

## M5 results — 2026-07-29

- Moved `EventJournal` / `JournalEvent` / `JournalError` / `AppendStatus` /
  `RunJournalSummary` from `src-tauri/src/altai/agent/event_journal.rs` into
  `altai-core::journal` (Tauri-free; `rusqlite` moved to `altai-core`'s own
  dependencies). Desktop's `event_journal` module is now a one-line
  `pub use altai_core::journal::*;` re-export, so `runtime.rs` and
  `tauri_channel.rs` are unchanged. All of the moved journal unit tests
  (migration idempotency, concurrent migration race, append/fetch ordering,
  duplicate/conflict handling, terminal CAS race) now run under
  `altai-core::journal::tests` and still pass.
- Added `WorkspacePaths::agent_event_journal_db()` to `altai-core::workspace`
  (`<isanagent_state>/.system_generated/agent_event_journal.db`), matching the
  path desktop's `ensure_workspace_services` already used inline, plus a unit
  test.
- The main Tauri crate now depends on `altai-core` directly
  (`src-tauri/Cargo.toml`), which it previously only referenced transitively
  as a workspace member.
- New `altai-cli::journal_sink::JournalSink` opens the same
  `agent_event_journal_db()` during `altai run` and appends a minimal subset
  mirroring desktop's journal `kind` conventions: `run_started` on the first
  `RunLifecycle::Started` bus message, then a single `run_terminated` once the
  oneshot host returns — synthesizing the run/chat identity from the final
  `OneshotResult` if no bus message was ever observed (e.g. an early host
  failure). Append failures are logged to stderr and never fail the run.
  `describe_oneshot_outcome` was extracted from `run_output::FinalRunResult`
  so the sink's terminal `outcome.kind` / `outcome.detail` match the
  `run --output json` outcome labels exactly.
- Added `altai journal summary [--chat ID] [--json]` (incomplete runs +
  optional latest-run-for-chat) and `altai journal fetch --run ID [--after
  SEQ] [--limit N] [--json]`, both reusing `altai_core::resolve_workspace` +
  `EventJournal::open` the same way `config` / `models` do.
- `CARGO_TARGET_DIR=src-tauri/target cargo test --manifest-path
  src-tauri/Cargo.toml -p altai-core -p altai-cli` → 56 tests passed (30
  `altai-core`, 26 `altai-cli`), including `journal::tests::*` (the moved
  desktop tests) and the new `journal_sink::tests::*` / `journal_*_contract_*`
  / `journal_summary_and_fetch_round_trip_a_run` tests.
- `cargo check --manifest-path src-tauri/Cargo.toml -p altai` (desktop crate)
  and `cargo test --manifest-path src-tauri/Cargo.toml -p altai --lib
  altai::agent::runtime` (34 tests, including
  `run_event_tests::restart_classifies_incomplete_runs_once_without_resuming_work`)
  both passed against the re-exported journal module.
- Manual smoke: `altai-cli run . --prompt "..." --permission plan --output
  jsonl` against a scratch workspace with no provider configured (so the run
  fails with `ProviderRetriesExhausted`) still committed `run_started` +
  `run_terminated` (`outcome.kind = "failed"`) to
  `.isanagent/.system_generated/agent_event_journal.db`; `altai journal
  summary --chat <id> --json` and `altai journal fetch --run <id> --json`
  round-tripped that run correctly.
- Not covered in M5 (deferred to M6 follow-up): mirroring tool-call/thinking/usage
  bus messages into the journal.

## M6 partial — `/dev/tty` approval resume — 2026-07-29

- Oneshot channel resumes approvals/clarifications on the controlling TTY when
  available (`prompt_on_tty`); hotkeys `y/n/a/x` normalize to
  approve/deny/always/abort. Non-TTY / failed tty still exits with code `4`.
- Synced into IsanAgent upstream via
  [`altaidevorg/isanagent#101`](https://github.com/altaidevorg/isanagent/pull/101)
  (superseding the [#99](https://github.com/altaidevorg/isanagent/pull/99) / revert cycle).
- Packaging / installed-binary CI matrix and full PTY golden frames remain open.

## Retarget — drop path pin — 2026-07-30

- After IsanAgent `main` landed the oneshot host surface (`ea90fa0`), ALTAI
  `Cargo.toml` deps switched back to
  `git = "https://github.com/altaidevorg/isanagent.git", branch = "main"`.
- Removed `tools/isanagent-oneshot`.
- `cargo test -p altai-core -p altai-cli` and `cargo check -p altai --lib`
  passed against the git tip; `pnpm isanagent:sync` (`cargo update -p isanagent`)
  works again.

## M6 partial — richer CLI journal telemetry — 2026-07-30

- `JournalSink` mirrors Desktop-shaped durable events after `run_started`:
  `tool_call_start`, `tool_call_end`, `thinking` (ToolProgress as
  `[tool] message`), and `usage`, with `isanagent::redact` before append.
- Legacy `ToolCallStarted` / `ToolCallFinished` ignored (no duplicate rows).
- Telemetry before run identity is known is dropped.
- Unit tests cover ordered kinds, tool error field, redaction, and pre-start
  ignore. Outbound assistant / subagent / execution journal parity remains open.
