# `@altai/agent-ui`

Shared ALTAI agent chat UI for Desktop and the VS Code Webview.

## Rules

- Depend on `@altai/host-contract` ports and capabilities only.
- Never import `@tauri-apps/*` or `vscode`.
- Hosts inject a `HostPorts` implementation via `HostPortsProvider`.
- Visible controls must be gated with `useCapability` / `isCapabilityEnabled`.

## Status (TASK-007 / A4 → A5)

Incremental extraction of the Desktop `AiSidePanel` tree, then Desktop
consumption of shared panel shells:

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
| A4.14 | `FileSuggestionList` (host supplies icons + popover) |
| A4.15 | `SelectionAskAi` (host supplies shortcut label) |
| A4.16 | `HoverActionButton` (chat message hover action) |
| A4.17 | `InspectorMetric` (run inspector metric tile) |
| A4.18 | `ContextAction` (composer attach menu row) |
| A4.19 | `RunStateMetric` (run state header metric tile) |
| A4.20 | `ProviderPill` (model dropdown provider rail pill) |
| A4.21 | `HistoryRow` (plan diff review restore row) |
| A4.22 | `ModelSectionLabel` (model dropdown section heading) |
| A4.23 | `InboxLoadFailed` (inbox error state with retry) |
| A4.24 | `FilteredEmptyInbox` (inbox filtered empty state) |
| A4.25 | `RowIconButton` (chat history/session row icon button, deduplicated) |
| A4.26 | `IconBtn` (AI status bar ghost icon button) |
| A4.27 | `ContextSourceToggle` (task runs context source toggle) |
| A4.28 | `TaskOutcome` (task run outcome summary) |
| A4.29 | `InboxSection` (inbox panel section wrapper) |
| A4.30 | `EmptyInbox` (inbox default empty state preset) |
| A4.31 | `SessionRow` (chat history session row with rename/delete) |
| A4.32 | `InspectorEmpty` (inspector section compact empty state) |
| A4.33 | `ModelOption` (model dropdown list option row) |
| A4.34 | `UnifiedDiffPreview` (coarse line-level diff preview) |
| A4.35 | `PlanRow` (plan diff review row with apply/reject/diff toggle) |
| A4.36 | `TodosInspector` (run inspector todos panel) |
| A4.37 | `AgentsInspector` (run inspector subagent task list) |
| A4.38 | `ChangeReviewBanner` (queued edits review banner) |
| A4.39 | `PlanModeStrip` (plan mode status strip) |
| A4.40 | `ResearchInspector` (run inspector research event list) |
| A4.41 | `McpInspector` (run inspector MCP call list) |
| A4.42 | `ArtifactsInspector` (run inspector artifact file list) |
| A4.43 | `ChangesInspector` (run inspector queued changes summary) |
| A4.44 | `ApprovalsInspector` (run inspector pending approvals) |
| A4.45 | `ChatProjectTarget` (composer project target chip) |
| A4.46 | `EmptyState` (empty chat home) |
| A4.47 | `ClarificationChoices` (edit approval / suggested replies) |
| A4.48 | `InboxNotificationCard` (+ inbox relative-time helpers) |
| A4.49 | `InboxJobCard` (background job inbox row) |
| A4.50 | `InboxTicketCard` (clarification ticket inbox row) |
| A4.51 | `RunRecoveryActions` (stuck/retry/warning strip) |
| A4.52 | `ReviewHistory` (plan review restore-points section) |
| A4.53 | `SnapshotsInspector` (run inspector restore snapshots) |
| A4.54 | `ActivityInspector` (run activity / timeline inspector) |
| A4.55 | `InspectorSection` (run inspector collapsible section) |
| A4.56 | `ChatTabStrip` (open-chat tab strip) |
| A4.57 | `WorkspaceTopbarActions` (Work / Inbox / Run details cluster) |
| A4.58 | `CheckpointMenuPanel` (edit-checkpoint popover body) |
| A4.59 | `ComposerToolbarIcon` (composer ghost icon control) |
| A4.60 | `ComposerAttachChips` (composer attachment chip row) |
| A4.61 | `CompactNowControl` (status-bar compact-context control) |
| A4.62 | `ProviderConnectBanner` (connect-provider strip) |
| A4.63 | `WorkspaceTargetForm` (choose-project dialog body) |
| A4.64 | `AiOpenControl` (status-bar show/hide AI toggle) |
| A5.1 | `PlanDiffReviewPanel` (change-review centre shell) |
| A5.2 | `NotificationInboxPanel` (agent inbox shell) |
| A5.3 | `ChatHistoryPanel` (+ session recency grouping helpers) |
| A5.4 | `TaskRunCard` (+ `formatTaskAge`) |
| A5.5 | `WorkHubNavigation` (Runs / Scheduled tab strip) |
| A5.6 | `AutomationCard` (+ schedule/next/last-run label helpers) |
| A5.7 | `ModelPickerPanel` (model dropdown popover body) |
| A5.8 | `AgentOptionRow` (agent switcher option chrome) |
| A5.9 | `TaskContextSources` (+ `contextFileName`) |
| A5.10 | `TaskSkillChips` (create-task skills multi-select) |
| A5.11 | `PromptTemplateGrid` + `SurfaceFilteredEmpty` |
| A5.12 | `AutomationScheduleFields` (+ `localDateTimeValue`) |
| A5.13 | `ComposerFollowupBar` (steer / queue strip) |
| A5.14 | `SurfaceFilterToolbar` (search + filter tabs strip) |

Desktop must import shared components from this package; local duplicates are
deleted as each slice lands.

```bash
pnpm --filter @altai/agent-ui typecheck
pnpm --filter @altai/agent-ui test
```
