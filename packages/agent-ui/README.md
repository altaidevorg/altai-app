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
| A6.121 | Open-chat-with-selection deep link payloads |
| A6.122 | Workspace trust attach helpers |
| A6.123 | Open-settings deep link payloads |
| A6.124 | Operations deep link payloads |
| A6.125 | Composer draft persist policy |
| A6.126 | Webview asset cache-bust helper |
| A6.127 | Secure random / id helpers |
| A6.128 | CSP nonce helper |
| A6.129 | Webview message id helper |
| A6.130 | Host recovery diagnostic hints |
| A6.131 | Persisted webview presentation state |
| A6.132 | Extension / Studio preference coerce helpers |
| A6.133 | Side panel width parse/clamp helpers |
| A6.134 | Side panel chrome surface + open-chat tabs |
| A6.135 | Side panel width storage I/O helpers |
| A6.136 | Operations open-intent builder from AI chrome |
| A6.137 | Side panel Escape dismiss policy |
| A6.138 | Run continue prompts + terminal attention copy |
| A6.139 | Budget-segment auto-continue soft cap |
| A6.140 | Pure model routing (auto-pick + tools constraint) |
| A6.141 | Native RPC method list parse + availability |
| A6.142 | MCP tool name parse + activity kind |
| A6.143 | Skill install source parse |
| A6.144 | Todo status normalize |
| A6.145 | Cleared-output markers + token estimate |
| A6.146 | Composer placeholder rotation |
| A6.147 | Plan edit proposal kind map |
| A6.148 | Chat title derive from user messages |
| A6.149 | Sensitive token redaction |
| A6.150 | Project instructions path + combine |
| A6.151 | Session id generator |
| A6.152 | Agent id / find / override helpers |
| A6.153 | Snippet id generator |
| A6.154 | Prune old tool outputs (recency budget) |
| A6.155 | Slash command index filter/resolve |
| A6.156 | Workspace slash path + command name |
| A6.157 | Workspace workflow frontmatter parse |
| A6.158 | Path / shell safety guards |
| A6.159 | Compaction threshold resolve |
| A6.160 | IsanAgent target URL + unresolved errors |
| A6.161 | Provider key presence helper |
| A6.162 | Join workspace relative path |
| A6.163 | Parse todo_write items |
| A6.164 | Native transcript message id |
| A6.165 | Backend chat message → transcript map |
| A6.166 | Configured local model target match |
| A6.167 | Merge recovered backend sessions |
| A6.168 | Fallback provider spec map |
| A6.169 | HeadersInit to record |
| A6.170 | Cloud catalog model target resolve |
| A6.171 | Model favorite/recent id list ops |
| A6.172 | Slash command focus suffix |
| A6.173 | Parse composer slash/hash lead |
| A6.174 | Find agent by id or name |
| A6.175 | Plan mode off slash tail |
| A6.176 | Sync body → byte array helpers |
| A6.177 | Request URL + method normalize |
| A6.178 | IsanAgent target resolution result |
| A6.179 | Slash agent switch toast copy |
| A6.180 | Slash command non-empty tail |
| A6.181 | Backend/session default title |
| A6.183 | Uint8Array body byte pack |
| A6.182 | Session list workspace query targets |
| A6.184 | Plan mode on/off toast copy |
| A6.185 | Slash session/run toast copy |
| A6.187 | Untitled session meta factory |
| A6.186 | Slash settings/ops toast copy |
| A6.188 | Untitled session title check |
| A6.189 | Desktop New chat uses DEFAULT_SESSION_TITLE |
| A6.190 | Deleted session id blocklist ops |
| A6.191 | Hydrate active session resolve |
| A6.192 | Insert session after active tab |
| A6.193 | Rename session title in list |
| A6.194 | Apply session workspace target |
| A6.195 | Remove session + next active id |
| A6.196 | Maybe auto-derive untitled session title |
| A6.197 | Cut transcript through Nth user turn |
| A6.198 | Desktop workspacePath via sessionWorkspacePathForId |
| A6.199 | Live/isolated env block builders |
| A6.200 | Slash command prompt templates |
| A6.201 | Project agent meta from run snapshot |
| A6.202 | Compaction activity toast copy |
| A6.203 | Notification inbox attention view |
| A6.204 | Plan queue proposal + applied mark |
| A6.205 | errorMessage/mutationKey/workspace path normalize |
| A6.206 | Automation list sort + cron job index |
| A6.207 | Pending id map start/end chrome |
| A6.208 | Omit record key + list item by id |
| A6.209 | Automation schedule next-run + default at |
| A6.210 | Automation filter counts/match/sort chrome |
| A6.211 | Automation schedule form error copy |
| A6.212 | Session history snippet + content presence |
| A6.213 | Task assignment status from live run |
| A6.214 | Task Runs filter counts/match/group chrome |
| A6.215 | Notification inbox filter/search chrome |
| A6.216 | Session id set + title map |
| A6.217 | Operations task/automation prompt templates |
| A6.218 | Ops create readiness + auto model pick |
| A6.219 | Conversation owner chat resolve |
| A6.220 | Provider API key presence checks |
| A6.221 | Timed cache fresh + task title from prompt |
| A6.222 | Catalog model availability filter |
| A6.223 | Automation every interval ms conversion |
| A6.224 | Task list sort + enabled agents filter |
| A6.225 | Task context path merge + bot title strip |
| A6.226 | Chat history session search filter |
| A6.227 | Catalog id lookup + model label |
| A6.228 | Task run outcome counts |
| A6.229 | Task run card tokens/skills/name labels |
| A6.230 | Task selected-context compose blocks |
| A6.231 | Waiting job state helpers |
| A6.232 | Id map + unread filter + list remove |
| A6.233 | Session history items + rename trim |
| A6.234 | Task context source detail labels |
| A6.235 | Model dropdown filter + fav/recent partition |
| A6.236 | Notification inbox empty + search filter |
| A6.237 | Automation form datetime/minutes parse |

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
