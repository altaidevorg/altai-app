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
| A5.15 | `CreateFormActions` (create-form cancel/submit footer) |
| A5.16 | `PromptEditorSection` (instruction textarea + templates) |
| A5.17 | `ComposerConfigRow` (agent/model config slots) |
| A5.18 | `TaskRunConfigSection` (create-task config chrome) |
| A5.19 | `RunDetailsHeader` (run inspector header + stop) |
| A5.20 | `RunOverviewCard` (run inspector summary metrics) |
| A5.21 | `RunBlockedBanner` (run inspector error banner) |
| A5.22 | `SurfacePrimaryAction` / `SurfaceSecondaryAction` |
| A5.23 | `ConversationOwnerSection` (automation owner chat) |
| A5.24 | `SurfaceInlineError` (dismissible list error strip) |
| A5.25 | `SurfaceLoadingState` (queue/list loading chrome) |
| A5.26 | `RunActionRequiredSection` (approvals section chrome) |
| A5.27 | `SurfaceListGroup` (titled queue/list group chrome) |
| A5.28 | `AutomationList` (Scheduled card-list shell) |
| A5.29 | `AgentSwitcherTrigger` (agent-picker trigger variants) |
| A5.30 | `ComposerPrimaryRow` (composer tools/actions shell) |
| A5.31 | `ComposerTextArea` (composer text-entry chrome) |
| A5.32 | `ComposerShell` (composer card + attachment chrome) |

### A6 / Transcript path (Wave 4)

| Slice | Contents |
|---|---|
| A6.1 | `stripUserContextBlocks` (shared user-message context parsing → chips) |
| A6.2 | `TranscriptToolGroup` (collapsible file/web/shell tool bursts) |
| A6.3 | `buildTranscriptPartGroups` + tool summaries (read/web/cmd collapse rules) |
| A6.4 | `TranscriptReadPaths` (expanded multi-file read list) |
| A6.5 | `AssistantBrandLabel` (assistant message brand + streaming hint) |
| A6.6 | `TranscriptReadRow` (single-file read row) |
| A6.7 | `TranscriptConversationEmpty` (empty transcript chrome + status slot) |
| A6.8 | `TranscriptRunError` (assertive run failure / attention banner) |
| A6.9 | Composer `@` / `#` / `/` caret triggers (`composerTriggers`) |
| A6.10 | `resolveComposerEnterAction` + post-send draft residual helpers |
| A6.11 | `composerFollowupVisibility` / `resolveComposerSubmitMode` |
| A6.12 | `autoresizeTextarea` (composer growth helper) |
| A6.13 | `isWebHref` / `hrefToFilePath` / `resolveWorkspacePath` |
| A6.14 | Run lifecycle chrome (`runBlockedMessageFromEvent`, recovery copy) |
| A6.15 | `segmentChatContent` / link token helpers (streamdown-light) |
| A6.16 | `looksLikePath` / `pathToFileUri` / `toolBubbleContent` |
| A6.17 | Interactive prompt parse (`normalizeAgentEventType`, apply helpers) |
| A6.18 | Pure `todoParse` (`isTodoToolName`, `parseTodoItems`, summarize) |
| A6.19 | User-turn edit helpers (`parseUserTurnId`, truncate, renumber) |
| A6.20 | `getComposerActionAvailability` (send/steer/queue from run status) |
| A6.21 | Composer Send/Stop policy (`composerSubmitChromeMode`, enable helpers) |
| A6.22 | Side panel chrome layout (`resolveSidePanelChromeLayout` breakpoints) |
| A6.23 | `AiSidePanelFrame` (outer panel landmark + topbar slot) |
| A6.24 | `AiChatMainColumn` (plan → transcript → run chrome → composer) |
| A6.25 | Composer attachment pure helpers (text context, draft, token estimate) |
| A6.26 | Composer snippet pure helpers (`expandSnippetTokens`, catalogs, picks) |
| A6.27 | Composer submit compose (`composeComposerSubmitText`, multimodal parts) |
| A6.28 | User-turn display (`prepareUserTurnDisplay`, command markers, stream ids) |
| A6.29 | Flat display tool-group blocks (`buildDisplayTranscriptBlocks`, aliases) |
| A6.30 | Composer draft/dispatch prelude (`commandSource`, file classify, picks) |
| A6.31 | Chat display transcript (`applyAgentEventToMessages`, session map) |
| A6.32 | Chat transcript frame + aria/retry/error pure chrome |
| A6.33 | Composer submit plan (`planComposerSubmit` ports-first prelude) |
| A6.34 | Composer draft clear after accept (`clearComposerDraftAfterAccept`) |
| A6.35 | Composer submit host intent (`mapComposerSubmitPlanToHostIntent`) |
| A6.36 | Ports-first submit execute + `useComposerController` headless draft |
| A6.38 | Display transcript list frame + role/bubble chrome (`AiDisplayTranscriptList`) |
| A6.39 | Display message action flags (`resolveDisplayMessageActions`, copy gates) |
| A6.40 | User turn body chrome (`AiUserTurnBody` command + chips + text) |
| A6.41 | AI-SDK assistant group list (`AiSdkAssistantGroups` + run-action pure) |
| A6.42 | Ports-first `AiChatViewFrame` + row meta (store-free chat shell) |
| A6.43 | AI-SDK tool part map (`mapSdkToolApprovalPart` / card) |
| A6.44 | AI-SDK UI part kind (`classifySdkUiPart` / text extract) |
| A6.45 | AI-SDK UI part view (`mapSdkUiPartView` host render routing) |
| A6.46 | AI-SDK UI part switch (`AiSdkUiPartSwitch` host chrome slots) |
| A6.47 | AI-SDK tool part switch (`AiSdkToolPartSwitch` approval vs card) |
| A6.48 | Assistant run action mode (`resolveAssistantRunActionMode`) |
| A6.49 | Assistant SDK groups state (`buildAssistantSdkGroupsState`) |
| A6.50 | Assistant run actions switch (`AiAssistantRunActions` slots) |
| A6.51 | Display message bubble shell (`AiDisplayMessageBubble`) |
| A6.52 | Display message edit form (`AiDisplayMessageEditForm`) |
| A6.53 | Display action labels (`displayCopyActionLabel`, open/diff titles) |
| A6.54 | Display body extras (`AiDisplayMessageBodyExtras` diff + todos) |
| A6.55 | Display list interaction hook (`useDisplayMessageInteractionState`) |
| A6.56 | Display tool-group icon key (`displayToolGroupIconKey`) |
| A6.57 | Display message content body (`AiDisplayMessageContent`) |
| A6.58 | Display message actions cluster (`AiDisplayMessageActions`) |
| A6.59 | Composer compact policy (`canMountCompactControl` / invoke) |
| A6.60 | Composer affordance hints (`listComposerAffordances`) |
| A6.61 | Composer compact control (`AiComposerCompactControl` slots) |
| A6.62 | Composer follow-up control (`AiComposerFollowupControl`) |
| A6.63 | Empty-state affordance hint slot (`EmptyState.affordanceHint`) |
| A6.64 | Composer attach menu policy (`canMountComposerAttachMenu`) |
| A6.65 | Composer suggestion keyboard (`resolveComposerSuggestionKeyAction`) |
| A6.66 | Composer suggestion list hook (`useComposerSuggestionList`) |
| A6.67 | Edit-diff display policy (`countPendingEditDiffs` / last) |
| A6.68 | Run inspector section mappers (`buildRunInspectorSections`) |
| A6.69 | Change-review panel policy (`listChangeReviewItems`, line stats) |
| A6.70 | Run details summary chrome (`canShowRunDetailsChrome`, metrics) |
| A6.71 | Agent status pill derive (`deriveAgentStatusMeta`, step labels) |
| A6.72 | Run usage token meter (`usageDeltaFromPayload`, accumulate) |
| A6.73 | Shell surface tab keyboard nav (`nextAltaiSurface`) |
| A6.74 | Side-chat shell density chrome (`shouldShowSurfaceTextTabs`) |
| A6.75 | Chat keyboard Escape dismiss (`isEscapeDismissKey`) |
| A6.76 | Replay control mount gate (`canMountReplayControl`) |
| A6.77 | Wait-shell diagnostic clipboard (`formatDiagnosticClipboardText`) |
| A6.78 | Permission-mode switcher gate (`canMountPermissionModeSwitcher`) |
| A6.79 | Skills status chrome (`canMountSkillsStatus`, summary) |
| A6.80 | Session remove mode (`resolveSessionRemoveMode`, archive prefer) |
| A6.81 | MCP status chrome (`canMountMcpStatus`, sort, summaries) |
| A6.82 | Composer caret helpers (`resolveComposerCaret`, advance after draft) |
| A6.83 | Settings hub nav catalog (`listSettingsHubNav`, normalize section) |
| A6.84 | Settings hub search filter (`filterSettingsNav`, score) |
| A6.85 | Transcript clipboard export (`formatTranscriptForCopy`) |
| A6.86 | Project-target chip chrome (`projectTargetFromWorkspace`) |
| A6.87 | Model picker mount/filter helpers (`canMountModelPicker`, Auto id) |
| A6.88 | Composer agent profiles (`COMPOSER_AGENT_PROFILES`, prefix) |
| A6.89 | Model catalog merge/filter (`mergeModelCatalog`, provider id) |
| A6.90 | Provider status merge/gates (`mergeProviderCatalog`, banner) |
| A6.91 | Settings snippet prefs → composer catalog merge |
| A6.92 | Checkpoint mount/map helpers (`toCheckpointMenuItems`) |
| A6.93 | Plan-mode strip helpers (`isPlanPermissionMode`, sticky todos) |
| A6.94 | Attach format helpers (`formatGitDiffSummary`, terminal text) |
| A6.95 | Workspace topbar mount/pressed helpers |
| A6.96 | Flat chat line kind classifier (`chatLineKind`) |
| A6.97 | Host-neutral composer slash command registry |
| A6.98 | Composer context chip helpers (`composeRunPrompt`) |
| A6.99 | Composer attach builders (`buildFileContextItem`, …) |
| A6.100 | Operations → Chat deep-link helpers (`buildOpenChatFocus`) |
| A6.101 | Task-run draft validation and skill prompt composition |
| A6.102 | Automation schedule draft validation |
| A6.103 | Session transcript line formatting for flat chat log |
| A6.104 | Operations overview aggregation + nav helpers |
| A6.105 | Operations attention badge poll helpers |
| A6.106 | Host recovery command allowlist + action list |
| A6.107 | Attention badge report parse + status-bar command |
| A6.108 | Multi-cursor selection join for attach context |
| A6.109 | Host lifecycle status pill labels |
| A6.110 | Host connecting progress presentation |
| A6.111 | Host user-facing error message map |
| A6.112 | Host status recovery toast policy |
| A6.113 | Host error action command policy |
| A6.114 | Search exclude glob builder |
| A6.115 | Preferred multi-root host URI retention |
| A6.116 | Provider base URL validation |
| A6.117 | Problems/diagnostics attach formatter |
| A6.118 | Virtual-only workspace classifier |
| A6.119 | Host lifecycle status-bar presentation |
| A6.120 | Open-chat-with-file deep link payloads |

### A5 complete enough

The TASK-007 / A5 stop condition is met. Desktop Work and Scheduled lists,
run-details chrome, and composer chrome now compose exported
`@altai/agent-ui` components. Desktop retains only host glue around those
surfaces: stores and message orchestration, Tauri/native calls, Radix
popover/dialog shells, resizable window layout, secrets, and capability
decisions. A VS Code Webview host can mount the shared package without copying
the Desktop JSX for these surfaces.

The next workstream is A7 / canonical Operations surfaces. Remaining
Desktop-local message-pipeline and window-shell code is intentionally outside
this extraction boundary; it should move only when a host-neutral contract is
required by that workstream.

### A7 / Operations surfaces

| Slice | Contents |
|---|---|
| A7.1 | `OperationsNavigationShell` (shared operations navigation chrome) |
| A7.2 | `OperationsOverview` (attention/progress summary; hosts aggregate data) |
| A7.3 | Desktop overview bridge (`CommandCenter` → `OperationsOverview` + row actions) |
| A7.4 | Operations work/runs/inbox domain views + embedded `AuxiliarySurface` |
| A7.5 | Operations view semantics (new-work → overview, Runs title, shell tests) |
| A7.6 | Operations deep-links from slash `/tasks`, `/inbox`, `/automations` |
| A7.7 | AI Work/Inbox topbar + legacy AI surface events → Operations |
| A7.8 | Rename Project Management chrome to Operations |

Desktop must import shared components from this package; local duplicates are
deleted as each slice lands.

```bash
pnpm --filter @altai/agent-ui typecheck
pnpm --filter @altai/agent-ui test
```
