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
  path-pinned at `tools/isanagent-oneshot` pending upstream merge; see
  `docs/cli/isanagent-oneshot-api.md`.
- `altai run` now performs a real one-shot host session (pretty / json / jsonl,
  Ctrl-C → exit 7, timeout → exit 8, non-TTY approval/clarification → exit 4).
- Non-interactive runs default to `--permission plan` when no permission flag is
  set.
- `cargo test --manifest-path src-tauri/Cargo.toml -p altai-cli` passed: 17
  tests, including `oneshot_smoke_completes_with_scripted_provider`.
- `cargo run -q -p altai-cli -- run . --prompt "summarize this project" --dry-run`
  still emits the resolved preview JSON.
