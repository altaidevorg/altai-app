# ALTAI CLI Implementation Status

**Date:** 2026-07-29
**Desktop base:** v0.6.4
**IsanAgent dependency:** `8c9eef2` (merged host API via
[`altaidevorg/isanagent#98`](https://github.com/altaidevorg/isanagent/pull/98))

## Existing

| Capability | Status | Evidence |
|---|---|---|
| `altai-core` crate (config, event, palette, workspace primitives) | Verified | 12 unit tests pass |
| `altai-cli` crate (clap command tree, doctor, version, completion, config, models) | Verified | 11 unit tests pass |
| Workspace path resolution and canonicalization | Verified | `altai-core::workspace` tests |
| Configuration precedence (IsanAgent config → ALTAI project config → environment) | Verified | `altai-core::config` tests |
| Event envelope schema v1 (`EventEnvelope<T>`) | Verified | `altai-core::event` test |
| Terminal palette manifest loading and validation | Verified | `altai-core::palette` test |
| `altai agent` — resolves host config, maps model/fallback/permission/theme/resume/files | Verified | dry-run preview; `host_adapter` tests |
| `altai agent` — starts the embedded IsanAgent TUI host for the supported path | Verified | `start_host` called for non-dry-run; `unsupported_agent_options` gates unimplemented flags |
| `altai agent --no-tui` — line mode mapping | Verified | `HostConfig.line_mode = true` |
| `altai run --dry-run` — preview of resolved workspace and run parameters | Verified | dry-run smoke check |
| `altai open` — desktop launcher router with `--dry-run` | Verified | desktop router test |
| `altai config path/list` — config location and resolved-origin inspection | Verified | contract tests |
| `altai models current` — resolved model inspection with origins | Verified | contract test |
| `altai doctor` / `altai version --verbose` | Verified | smoke checks |
| IsanAgent host API: `HostConfig`, `start_host`, `spawn_host`, `HostHandle` | Verified | upstream merged; `spawn_host` lifecycle test |

## Verified test results — 2026-07-29

```
cargo test -p altai-core   →  12 passed
cargo test -p altai-cli    →  11 passed
                             ─────────────
                             23 total

cargo run -q -p altai-cli -- agent . --no-tui --dry-run   → OK (JSON preview)
cargo run -q -p altai-cli -- run . --prompt "summarize this project" --dry-run → OK (JSON preview)
cargo run -q -p altai-cli -- doctor --json                → OK
cargo run -q -p altai-cli -- version --verbose            → OK
```

## Incomplete

| Capability | Gap | Planned milestone |
|---|---|---|
| `altai run` real execution | Currently dry-run only; returns `HostUnavailable` for non-dry-run. No one-shot host API exists. | **M1** |
| One-shot host API in IsanAgent | `run_host` always starts interactive channels. No headless prompt-injection or event-tap mechanism. | **M1 (upstream change)** |
| Output renderers (pretty/plain/final/jsonl) | `OutputMode` enum exists; no event-to-renderer pipeline. | **M1** |
| Ctrl-C cancellation for `run` | Not implemented. | **M1** |
| Timeout handling for `run` | `--timeout` parsed but not enforced. | **M1** |
| Meaningful exit codes for `run` | Only generic codes 1 and 10 used. | **M1** |
| Non-TTY permission safety | No explicit rejection logic for headless approval requests. | **M1** |
| ALTAI palette wiring to TUI renderer | `--theme dark/light` explicitly blocked by `unsupported_agent_options`. | **M2** |
| Responsive TUI layout (80/100/120+ cols) | Not implemented. | **M2** |
| Typed policy rules, approval cards, diff review | Not implemented. | **M3** |
| `@file` fuzzy references, context inspection, compaction | Not implemented. | **M4** |
| Custom slash commands, skills, MCP discovery, model profiles | Not implemented. | **M4** |
| Session list/resume/export commands | Not implemented. | **M4** |
| Multi-agent profiles, background jobs, task graph | Not implemented. | **M5** |
| JSONL schema stabilization, CI contract, shell completion packaging | Not implemented. | **M6** |

## Blocked

| Blocker | Impact | Resolution path |
|---|---|---|
| No public one-shot/headless IsanAgent host API | `altai run` cannot inject a prompt, stream events, or detect run completion without duplicating the agent loop | M1: narrowest upstream change — add `oneshot_prompt` and `event_sink` to `HostConfig`, modify `run_host` to support headless injection + auto-shutdown on `RunLifecycle::Terminated` |
| ALTAI runtime events are Tauri-bound | Desktop and CLI cannot yet share a durable event journal | M5: extract event sink trait into `altai-core` |

## Next milestone

### M1 — Make `altai run` real

**Scope:**
- Add a minimal public one-shot host API to IsanAgent (upstream patch).
- Implement `altai run [PATH] --prompt TEXT` as a real, one-shot ALTAI/IsanAgent session.
- Support pretty, JSON, and JSONL output modes.
- Implement Ctrl-C cancellation, timeout handling, meaningful exit codes.
- Handle non-TTY permission requests safely (reject, never silently approve).
- Preserve model/fallback/permission/resume/files/workspace settings.
- Add unit tests and at least one non-dry-run smoke test.

**Files likely affected:**
- `src-tauri/crates/altai-cli/src/main.rs` — `run_prompt` implementation
- `src-tauri/crates/altai-cli/src/host_adapter.rs` — one-shot host config
- New: `src-tauri/crates/altai-cli/src/run_renderer.rs` — output renderers
- Upstream: `isanagent/src/host.rs` — one-shot host API
- `docs/cli/TEST.md` — test results
- `docs/cli/isanagent-host-api.patch` — updated upstream diff

**Risks:**
- Upstream change to `run_host` could destabilize the interactive `agent` path.
- Mock provider needed for deterministic smoke test (no paid API calls in CI).
- Exit-code mapping must agree with JSONL `run_finished` event.

**Acceptance checks:**
- `cargo test -p altai-cli` passes with new tests.
- `altai run . --prompt "test" --output jsonl` emits stable lifecycle events.
- `altai run . --prompt "test" --output json` prints a final structured result.
- Ctrl-C returns exit code 7.
- Timeout returns exit code 8.
- Non-TTY approval request returns exit code 4.
- Dry-run still works unchanged.

## Architecture summary

```
altai-core (no Tauri, no UI)
  ├── config.rs      — precedence resolution (IsanAgent → ALTAI → env)
  ├── event.rs       — versioned JSONL envelope (schema v1)
  ├── palette.rs     — terminal palette manifest from CSS tokens
  └── workspace.rs   — canonical path resolution

altai-cli (no Tauri, no UI)
  ├── main.rs        — clap command tree, command handlers
  └── host_adapter.rs — HostConfig mapping for workspace/sandbox separation

isanagent::host (upstream, pinned rev 8c9eef2)
  ├── HostConfig     — workspace, config, sandbox, model, permission, etc.
  ├── start_host()   — blocks until host exits (interactive)
  ├── spawn_host()   — returns HostHandle with shutdown/event control
  └── run_host()     — internal: full runtime construction + channel routing
```
