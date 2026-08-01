# ALTAI CLI-Anything Analysis

**Harness revision:** `bc536c9bebb7c3d9f7bb2736a732609139c1acdb`  
**Harness reference:** `tools/cli-anything.lock`  
**Status:** Phase 0 baseline

## Product boundary

ALTAI already has a native Rust backend and a native agent runtime. The CLI
must invoke those services directly; it must not be a generated Python proxy or
a screen-scraping wrapper around the desktop application.

CLI-Anything is used as the required development harness for action mapping,
state analysis, command design, test planning, gap review, validation, and
agent-facing skill documentation.

## Desktop-to-terminal action map

| ALTAI desktop capability | Terminal product surface | Backend owner |
|---|---|---|
| AI chat and streaming | `altai-cli`, `altai-cli -p` | IsanAgent + ALTAI agent runtime |
| Session history and replay | `altai chat`, TUI session browser | workspace memory + event journal |
| Model, fallback, and permission selection | `altai models`, `altai auth`, TUI selectors | shared config and secret store |
| Tool calls, approvals, and edit diffs | TUI tool rail and approval overlay | agent runtime event sink |
| Execution and background jobs | `altai jobs`, TUI execution/job panes | execution manager + durable jobs |
| Subagents and orchestration | `altai orchestration`, TUI task pane | orchestration services |
| Notifications and clarification tickets | `altai inbox`, TUI inbox pane | durable inbox services |
| Skills, MCP, checkpoints, automations, papers | matching subcommands and slash commands | existing workspace services |
| Workspace open and OS integration | `altai open`, legacy bare paths | desktop executable router |

## State map

| State | Canonical location | CLI rule |
|---|---|---|
| Workspace agent state | `<workspace>/.isanagent/` | Use the exact desktop workspace root. |
| Conversation memory | IsanAgent SQLite memory store | Do not make a second session database. |
| Durable agent events | ALTAI event journal | Replay on TUI startup and expose through `chat show --events`. |
| Desktop preferences | ALTAI application data store | Read through the shared preference resolver. |
| Credentials | ALTAI platform secret store | Never echo, log, or serialize values. |
| Runtime ownership | workspace runtime lease | Reject a concurrent live owner for the same chat. |
| Terminal visual mapping | `shared/altai-terminal-palette.json` | CSS semantic tokens remain the color-value source. |

## Mandatory harness checks

1. Every shipped command has `--help`, a human output mode, a machine output
   mode, deterministic exit codes, and a documented permission behavior.
2. Every state-changing command provides prior-state inspection or a dry-run
   where practical.
3. Every release runs installed-binary subprocess tests against the real Rust
   runtime and a real temporary workspace.
4. Every visual terminal surface has truecolor, ANSI fallback, and no-color
   snapshot coverage.
5. Every desktop capability either has a terminal equivalent or a documented
   visual-only rationale in the parity matrix.

## Initial gaps to close

- The desktop launcher is not a headless CLI.
- IsanAgent's TUI host is not yet exposed as a reusable library interface.
- ALTAI runtime events are Tauri-bound.
- The model catalog and desktop preferences are TypeScript/Tauri-bound.
- ALTAI visual tokens do not yet have a Ratatui palette implementation.
