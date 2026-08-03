# `@altai/agent-ui`

Shared ALTAI agent chat UI for Desktop and the VS Code Webview.

## Rules

- Depend on `@altai/host-contract` ports and capabilities only.
- Never import `@tauri-apps/*` or `vscode`.
- Hosts inject a `HostPorts` implementation via `HostPortsProvider`.
- Visible controls must be gated with `useCapability` / `isCapabilityEnabled`.

## Status (TASK-007 / A4)

Incremental extraction of the Desktop `AiSidePanel` tree:

| Slice | Contents |
|---|---|
| A4.1 | `HostPortsProvider` / capability hooks |
| A4.2 | `AuxiliarySurface` chrome (`SurfaceHeader`, `SurfaceSearch`, …) |
| A4.3 | `AiToolApproval` (host supplies assertive-announce pref) |
| A4.4 | `EditApprovalCard` (host supplies `onRespond`) |
| A4.5 | `TodoChecklist` / `parseTodoItems` |
| A4.6 | `ChatPathLink` / `ChatExternalLink` (host supplies `onOpen`) |
| A4.7 | `AgentStatusPill` (host supplies `meta` + `formatStepLabel`) |
| A4.8 | `TodoSummaryChip` (host supplies `todos`) |
| A4.9 | `ComposerConfigTrigger` (agent/model picker chrome) |
| A4.10 | `ContextChips` (typed context chip row for chat) |
| A4.11 | `PermissionModeSwitcher` (host supplies mode + callbacks) |
| A4.12 | `CommandSnippet` (host supplies slash-command meta) |
| A4.13 | `ComposerSuggestionList` (host owns popover chrome) |

Desktop must import shared components from this package; local duplicates are
deleted as each slice lands.

```bash
pnpm --filter @altai/agent-ui typecheck
pnpm --filter @altai/agent-ui test
```
