export {
  HostPortsProvider,
  useHostPorts,
  useHostPortsContext,
  useCapability,
  type HostPortsContextValue,
  type HostPortsProviderProps,
} from "./host/HostPortsProvider.js";

export {
  HostPortUnsupportedError,
  unsupported,
  withUnsupportedDefaults,
} from "./host/unsupported.js";

export {
  AuxiliarySurface,
  SurfaceEmptyState,
  SurfaceHeader,
  SurfaceIconAction,
  SurfaceSearch,
  SurfaceSectionHeader,
  SurfaceTabs,
} from "./components/AuxiliarySurface.js";

export {
  AiToolApproval,
  type AiToolApprovalProps,
  type ToolApprovalPart,
} from "./components/AiToolApproval.js";

export {
  EditApprovalCard,
  parseDiffLines,
  type EditApprovalCardProps,
  type EditApprovalDiff,
} from "./components/EditApprovalCard.js";

export {
  TodoChecklist,
  parseTodoItems,
  summarizeTodos,
  type TodoChecklistProps,
  type TodoItem,
  type TodoItemStatus,
} from "./components/TodoChecklist.js";

export {
  isTodoToolName,
  parseTodoItemsFromInput,
  summarizeTodoItems,
} from "./lib/todoParse.js";

export {
  canEditUserMessage,
  parseUserTurnId,
  renumberUserTurnIds,
  truncateBoundaryForEdit,
  truncateDisplayAfterUserTurn,
} from "./lib/chatMessageEdit.js";

export {
  ChatPathLink,
  ChatExternalLink,
  type ChatPathLinkProps,
  type ChatExternalLinkProps,
} from "./components/ChatPathLink.js";

export {
  AgentStatusPill,
  type AgentStatusMeta,
  type AgentStatusPillProps,
} from "./components/AgentStatusPill.js";

export {
  TodoSummaryChip,
  type TodoSummaryChipProps,
} from "./components/TodoSummaryChip.js";

export {
  ComposerConfigTrigger,
  type ComposerConfigTriggerProps,
} from "./components/ComposerConfigTrigger.js";

export {
  ContextChips,
  type ContextChip,
  type ContextChipsProps,
} from "./components/ContextChips.js";

export {
  PermissionModeSwitcher,
  effectivePermissionMode,
  visiblePermissionModes,
  PERMISSION_MODE_LABELS,
  PERMISSION_MODE_DESCRIPTIONS,
  type PermissionModeSwitcherProps,
} from "./components/PermissionModeSwitcher.js";

export {
  CommandSnippet,
  type CommandSnippetMeta,
  type CommandSnippetProps,
} from "./components/CommandSnippet.js";

export {
  ComposerSuggestionList,
  type ComposerSuggestionCommand,
  type ComposerSuggestionItem,
  type ComposerSuggestionListProps,
  type ComposerSuggestionSnippet,
} from "./components/ComposerSuggestionList.js";

export {
  FileSuggestionList,
  type FileSuggestionListProps,
} from "./components/FileSuggestionList.js";

export {
  SelectionAskAi,
  type SelectionAskAiProps,
} from "./components/SelectionAskAi.js";

export {
  HoverActionButton,
  type HoverActionButtonProps,
} from "./components/HoverActionButton.js";

export {
  InspectorMetric,
  type InspectorMetricProps,
} from "./components/InspectorMetric.js";

export {
  ContextAction,
  type ContextActionProps,
} from "./components/ContextAction.js";

export {
  RunStateMetric,
  type RunStateMetricProps,
} from "./components/RunStateMetric.js";

export {
  ProviderPill,
  type ProviderPillProps,
} from "./components/ProviderPill.js";

export {
  HistoryRow,
  type HistoryRowProps,
} from "./components/HistoryRow.js";

export {
  ModelSectionLabel,
  type ModelSectionLabelProps,
} from "./components/ModelSectionLabel.js";

export {
  InboxLoadFailed,
  type InboxLoadFailedProps,
} from "./components/InboxLoadFailed.js";

export {
  FilteredEmptyInbox,
  type FilteredEmptyInboxProps,
} from "./components/FilteredEmptyInbox.js";

export {
  RowIconButton,
  type RowIconButtonProps,
} from "./components/RowIconButton.js";

export {
  IconBtn,
  type IconBtnProps,
} from "./components/IconBtn.js";

export {
  ContextSourceToggle,
  type ContextSourceToggleProps,
} from "./components/ContextSourceToggle.js";

export {
  TaskOutcome,
  type TaskOutcomeProps,
} from "./components/TaskOutcome.js";

export {
  InboxSection,
  type InboxSectionProps,
} from "./components/InboxSection.js";

export {
  EmptyInbox,
} from "./components/EmptyInbox.js";

export {
  SessionRow,
  type SessionRowProps,
} from "./components/SessionRow.js";

export {
  InspectorEmpty,
  type InspectorEmptyProps,
} from "./components/InspectorEmpty.js";

export {
  ModelOption,
  type ModelOptionProps,
} from "./components/ModelOption.js";

export {
  UnifiedDiffPreview,
  type UnifiedDiffPreviewProps,
} from "./components/UnifiedDiffPreview.js";

export {
  PlanRow,
  type PlanRowProps,
} from "./components/PlanRow.js";

export {
  TodosInspector,
  type TodosInspectorItem,
  type TodosInspectorProps,
} from "./components/TodosInspector.js";

export {
  AgentsInspector,
  type AgentsInspectorTask,
  type AgentsInspectorProps,
} from "./components/AgentsInspector.js";

export {
  ChangeReviewBanner,
  type ChangeReviewBannerProps,
} from "./components/ChangeReviewBanner.js";

export {
  PlanModeStrip,
  type PlanModeStripProps,
} from "./components/PlanModeStrip.js";

export {
  ResearchInspector,
  type ResearchInspectorEvent,
  type ResearchInspectorProps,
} from "./components/ResearchInspector.js";

export {
  McpInspector,
  type McpInspectorEvent,
  type McpInspectorProps,
} from "./components/McpInspector.js";

export {
  ArtifactsInspector,
  type ArtifactsInspectorItem,
  type ArtifactsInspectorProps,
} from "./components/ArtifactsInspector.js";

export {
  ChangesInspector,
  type ChangesInspectorItem,
  type ChangesInspectorProps,
} from "./components/ChangesInspector.js";

export {
  ApprovalsInspector,
  approvalPreview,
  type ApprovalsInspectorItem,
  type ApprovalsInspectorProps,
} from "./components/ApprovalsInspector.js";

export {
  ChatProjectTarget,
  type ChatProjectTargetProps,
} from "./components/ChatProjectTarget.js";

export {
  EmptyState,
  type EmptyStateProps,
} from "./components/EmptyState.js";

export {
  ClarificationChoices,
  type ClarificationChoicesProps,
} from "./components/ClarificationChoices.js";

export {
  InboxNotificationCard,
  type InboxNotificationCardProps,
  type InboxNotificationItem,
} from "./components/InboxNotificationCard.js";

export {
  InboxJobCard,
  labelForInboxJob,
  type InboxJobCardProps,
  type InboxJobItem,
} from "./components/InboxJobCard.js";

export {
  InboxTicketCard,
  type InboxTicketCardProps,
  type InboxTicketItem,
} from "./components/InboxTicketCard.js";

export {
  RunRecoveryActions,
  type RunRecoveryActionsProps,
} from "./components/RunRecoveryActions.js";

export {
  ReviewHistory,
  type ReviewHistoryItem,
  type ReviewHistoryProps,
} from "./components/ReviewHistory.js";

export {
  SnapshotsInspector,
  type SnapshotsInspectorAppliedItem,
  type SnapshotsInspectorCheckpointItem,
  type SnapshotsInspectorProps,
} from "./components/SnapshotsInspector.js";

export {
  ActivityInspector,
  type ActivityInspectorEvent,
  type ActivityInspectorProps,
} from "./components/ActivityInspector.js";

export {
  InspectorSection,
  type InspectorSectionProps,
} from "./components/InspectorSection.js";

export {
  ChatTabStrip,
  type ChatTabItem,
  type ChatTabStripProps,
} from "./components/ChatTabStrip.js";

export {
  WorkspaceTopbarActions,
  type WorkspaceTopbarActionsProps,
} from "./components/WorkspaceTopbarActions.js";

export {
  CheckpointMenuPanel,
  checkpointBasename,
  formatCheckpointTimeAgo,
  type CheckpointItem,
  type CheckpointMenuPanelProps,
} from "./components/CheckpointMenuPanel.js";

export {
  ComposerToolbarIcon,
  type ComposerToolbarIconProps,
} from "./components/ComposerToolbarIcon.js";

export {
  ComposerAttachChips,
  fileExtensionLabel,
  selectionLineCount,
  type ComposerAttachChipsProps,
  type ComposerAttachCommand,
  type ComposerAttachFile,
  type ComposerAttachSnippet,
} from "./components/ComposerAttachChips.js";

export {
  CompactNowControl,
  type CompactNowControlProps,
} from "./components/CompactNowControl.js";

export {
  AiComposerCompactControl,
  type AiComposerCompactControlProps,
} from "./components/AiComposerCompactControl.js";

export {
  canInvokeCompact,
  canMountCompactControl,
  type ComposerCompactFlags,
} from "./lib/composerCompactPolicy.js";

export {
  formatComposerHintLine,
  listComposerAffordances,
  type ComposerHint,
} from "./lib/composerHintChrome.js";

export {
  canMountComposerAttachMenu,
  composerAttachSurfaceShowsAttachments,
  composerAttachSurfaceShowsToolbar,
  type ComposerAttachCapabilityFlags,
  type ComposerAttachSurface,
} from "./lib/composerAttachPolicy.js";

export {
  ProviderConnectBanner,
  type ProviderConnectBannerProps,
} from "./components/ProviderConnectBanner.js";

export {
  WorkspaceTargetForm,
  type WorkspaceTargetBusy,
  type WorkspaceTargetFormProps,
} from "./components/WorkspaceTargetForm.js";

export {
  AiOpenControl,
  type AiOpenControlProps,
} from "./components/AiOpenControl.js";

export {
  PlanDiffReviewPanel,
  planDiffStats,
  type PlanDiffReviewPanelProps,
  type PlanDiffReviewQueueItem,
} from "./components/PlanDiffReviewPanel.js";

export {
  NotificationInboxPanel,
  type NotificationInboxFilter,
  type NotificationInboxJobRow,
  type NotificationInboxNotificationRow,
  type NotificationInboxPanelProps,
  type NotificationInboxTicketRow,
} from "./components/NotificationInboxPanel.js";

export {
  ChatHistoryPanel,
  type ChatHistoryPanelProps,
} from "./components/ChatHistoryPanel.js";

export {
  AgentChatLayout,
  type AgentChatLayoutDensity,
  type AgentChatLayoutProps,
} from "./components/AgentChatLayout.js";

export {
  groupSessionsByRecency,
  sessionHistoryBucket,
  startOfDay,
  SESSION_HISTORY_GROUP_ORDER,
  type SessionHistoryGroup,
  type SessionHistoryItem,
} from "./lib/sessionHistory.js";

export {
  TaskRunCard,
  formatTaskAge,
  type TaskRunCardProps,
  type TaskRunStatus,
} from "./components/TaskRunCard.js";

export {
  WorkHubNavigation,
  type WorkHubNavigationProps,
  type WorkHubView,
} from "./components/WorkHubNavigation.js";

export {
  OperationsNavigationShell,
  OPERATIONS_VIEWS,
  type OperationsNavigationShellProps,
  type OperationsView,
} from "./components/OperationsNavigationShell.js";

export {
  OperationsOverview,
  type OperationsOverviewMetric,
  type OperationsOverviewProps,
  type OperationsOverviewRow,
} from "./components/OperationsOverview.js";

export {
  AutomationCard,
  automationLastRunLabel,
  automationNextRunLabel,
  automationScheduleLabel,
  type AutomationCardProps,
  type AutomationSchedule,
} from "./components/AutomationCard.js";

export {
  AutomationList,
  type AutomationListItem,
  type AutomationListProps,
} from "./components/AutomationList.js";

export {
  ModelPickerPanel,
  type ModelPickerAutoOption,
  type ModelPickerPanelProps,
  type ModelPickerProvider,
  type ModelPickerRow,
} from "./components/ModelPickerPanel.js";

export {
  AgentOptionRow,
  type AgentOptionRowProps,
} from "./components/AgentOptionRow.js";

export {
  AgentSwitcherTrigger,
  type AgentSwitcherTriggerProps,
  type AgentSwitcherTriggerVariant,
} from "./components/AgentSwitcherTrigger.js";

export {
  TaskContextSources,
  contextFileName,
  type TaskContextSourcesProps,
} from "./components/TaskContextSources.js";

export {
  TaskSkillChips,
  type TaskSkillChipsProps,
  type TaskSkillOption,
} from "./components/TaskSkillChips.js";

export {
  PromptTemplateGrid,
  type PromptTemplate,
  type PromptTemplateGridProps,
} from "./components/PromptTemplateGrid.js";

export {
  SurfaceFilteredEmpty,
  type SurfaceFilteredEmptyProps,
} from "./components/SurfaceFilteredEmpty.js";

export {
  AutomationScheduleFields,
  localDateTimeValue,
  type AutomationScheduleFieldsProps,
  type AutomationScheduleMode,
} from "./components/AutomationScheduleFields.js";

export {
  ComposerFollowupBar,
  type ComposerFollowupBarProps,
} from "./components/ComposerFollowupBar.js";

export {
  AiComposerFollowupControl,
  type AiComposerFollowupControlProps,
} from "./components/AiComposerFollowupControl.js";

export {
  SurfaceFilterToolbar,
  type SurfaceFilterTab,
  type SurfaceFilterToolbarProps,
} from "./components/SurfaceFilterToolbar.js";

export {
  CreateFormActions,
  type CreateFormActionsProps,
} from "./components/CreateFormActions.js";

export {
  PromptEditorSection,
  type PromptEditorSectionProps,
} from "./components/PromptEditorSection.js";

export {
  ComposerConfigRow,
  type ComposerConfigRowProps,
} from "./components/ComposerConfigRow.js";

export {
  ComposerPrimaryRow,
  type ComposerPrimaryRowProps,
} from "./components/ComposerPrimaryRow.js";

export {
  ComposerTextArea,
  type ComposerTextAreaProps,
} from "./components/ComposerTextArea.js";

export {
  ComposerShell,
  type ComposerShellProps,
} from "./components/ComposerShell.js";

export {
  AiComposer,
  type AiComposerProps,
} from "./components/AiComposer.js";

export {
  TaskRunConfigSection,
  type TaskRunConfigSectionProps,
} from "./components/TaskRunConfigSection.js";

export {
  RunDetailsHeader,
  type RunDetailsHeaderProps,
  type RunDetailsStatus,
} from "./components/RunDetailsHeader.js";

export {
  RunOverviewCard,
  type RunOverviewCardProps,
  type RunOverviewMetric,
} from "./components/RunOverviewCard.js";

export {
  RunBlockedBanner,
  type RunBlockedBannerProps,
} from "./components/RunBlockedBanner.js";

export {
  SurfacePrimaryAction,
  SurfaceSecondaryAction,
  type SurfacePrimaryActionProps,
  type SurfaceSecondaryActionProps,
} from "./components/SurfacePrimaryAction.js";

export {
  TranscriptToolGroup,
  type TranscriptToolGroupProps,
} from "./components/TranscriptToolGroup.js";

export {
  TranscriptReadPaths,
  type TranscriptReadPathsProps,
} from "./components/TranscriptReadPaths.js";

export {
  TranscriptReadRow,
  type TranscriptReadRowProps,
} from "./components/TranscriptReadRow.js";

export {
  AssistantBrandLabel,
  type AssistantBrandLabelProps,
} from "./components/AssistantBrandLabel.js";

export {
  TranscriptConversationEmpty,
  type TranscriptConversationEmptyProps,
} from "./components/TranscriptConversationEmpty.js";

export {
  TranscriptRunError,
  type TranscriptRunErrorProps,
  type TranscriptRunErrorVariant,
} from "./components/TranscriptRunError.js";

export { stripUserContextBlocks } from "./lib/userContextBlocks.js";

export {
  ALTAI_CMD_RE,
  ALTAI_COMMAND_MARKER_RE,
  indexOfLastTextPart,
  parseCommandMarkerPrefix,
  prepareUserTurnDisplay,
  resolveStreamingAssistantMessageId,
  wrapWithCommandMarker,
  type UserTurnDisplay,
} from "./lib/userTurnDisplay.js";

export {
  appendMetaMessage,
  appendUserMessage,
  applyAgentEventToMessages,
  displayMessagesFromSession,
  extractEditDiff,
  extractTodoToolItems,
  extractToolFileTarget,
  newDisplayMessageId,
  shouldShowChatEmptyHome,
  textFromAgentEvent,
  type ChatDisplayMessage,
  type ChatDisplayRole,
  type SessionMessageLike,
} from "./lib/chatDisplayTranscript.js";

export {
  AT_MENTION_MIN_QUERY,
  detectAtMention,
  detectSlashOrSnippetTrigger,
  nextAtMentionIndex,
  pathForSuggestionList,
  removeAtMentionToken,
  shouldSearchAtMention,
  type AtMentionRange,
  type ComposerTokenTrigger,
} from "./lib/composerTriggers.js";

export {
  nextSuggestionActiveIndex,
  resolveComposerSuggestionKeyAction,
  resolveComposerSuggestionOpen,
  type ComposerSuggestionKeyAction,
} from "./lib/composerSuggestionKeyboard.js";

export {
  getComposerActionAvailability,
  remainingTextAfterAcceptedDispatch,
  resolveComposerEnterAction,
  type ComposerAction,
  type ComposerActionAvailability,
  type ComposerActionAvailabilityInput,
} from "./lib/composerEnterAction.js";

export {
  canEnableComposerSend,
  canEnableComposerStop,
  composerSubmitChromeMode,
  type ComposerSubmitChromeMode,
} from "./lib/composerSubmitChrome.js";

export {
  composerFollowupVisibility,
  resolveComposerSubmitMode,
  type ComposerFollowupMode,
  type ComposerFollowupPolicyInput,
  type ComposerFollowupVisibility,
} from "./lib/composerFollowup.js";

export {
  resolveSidePanelChromeLayout,
  SIDE_PANEL_HISTORY_SIDEBAR_MIN_WIDTH,
  SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH,
  type SidePanelChromeLayout,
  type SidePanelChromeLayoutInput,
  type SidePanelVariant,
} from "./lib/sidePanelLayout.js";

export {
  AiSidePanelFrame,
  type AiSidePanelFrameProps,
} from "./components/AiSidePanelFrame.js";

export {
  AiChatMainColumn,
  type AiChatMainColumnProps,
} from "./components/AiChatMainColumn.js";

export {
  canRetryLastAssistantTurn,
  isRecoverableAttentionMessage,
  isRetryableRunOutcome,
  joinMessageTextParts,
  resolveChatAriaLive,
  resolveTranscriptRunErrorVariant,
  type TranscriptAriaLivePref,
} from "./lib/chatTranscriptChrome.js";

export {
  isRecoverableRunOutcome,
  describeTerminalOutcomeAttention,
  continueStuckPrompt,
  continueBudgetSegmentPrompt,
  describeRunWarning,
  MAX_BUDGET_SEGMENT_AUTO_CONTINUES,
  nextBudgetSegmentAutoContinueCount,
  type SharedRunBudgetSnapshot,
  type SharedRunBudgetWarning,
  type SharedRunOutcome,
} from "./lib/runContinueChrome.js";
export { projectAgentMetaFromRun } from "./lib/agentMetaFromRun.js";
export type {
  SharedRunTokenSnapshot,
  SharedRunOutcomeLike,
  SharedRunStateLike,
  ProjectedAgentRunStatus,
  ProjectedAgentTokens,
} from "./lib/agentMetaFromRun.js";
export {
  parseNativeMethodList,
  nativeMethodAvailable,
} from "./lib/nativeMethodList.js";

export {
  agentRequiresTools,
  supportsAgentModel,
  describeModelConstraint,
  pickAutoModel,
  type SharedModelRoutingAgent,
  type SharedModelRoutingCapabilities,
  type SharedModelRoutingInfo,
  type PickAutoModelInput,
} from "./lib/modelRouting.js";

export {
  parseMcpToolName,
  activityKindForTool,
  RESEARCH_TOOL_NAMES,
  type McpToolInfo,
  type ToolActivityKind,
} from "./lib/mcpToolName.js";

export {
  parseSkillInstallSource,
  type ParsedSkillInstallSource,
} from "./lib/skillInstallSource.js";

export {
  normalizeTodoStatus,
  type SharedTodoStatus,
} from "./lib/todoStatus.js";

export {
  CLEARED_OUTPUT,
  CLEARED_TOOL_OUTPUT_TEXT,
  isClearedOutput,
  estimateTokens,
} from "./lib/tokenEstimate.js";
export { pruneOldToolOutputs } from "./lib/pruneToolOutputs.js";
export type { PruneableMessage } from "./lib/pruneToolOutputs.js";
export { proposalKindFromPlanEdit } from "./lib/proposalKind.js";

export {
  COMPOSER_PLACEHOLDERS,
  pickPlaceholder,
} from "./lib/composerPlaceholders.js";
export {
  stripChatTitleNoise,
  deriveChatTitleFromMessages,
  type ChatTitleMessage,
  type ChatTitleMessagePart,
} from "./lib/chatTitle.js";
export { redactSensitive } from "./lib/redactSensitive.js";
export {
  PROJECT_INSTRUCTIONS_FILE,
  MAX_PROJECT_INSTRUCTIONS_CHARS,
  projectInstructionsPath,
  combineAgentInstructions,
  clampProjectInstructions,
} from "./lib/projectInstructions.js";

export { newSessionId } from "./lib/sessionId.js";
export { newSnippetId } from "./lib/snippetId.js";
export {
  filterSlashCommands,
  resolveSlashCommandInIndex,
} from "./lib/slashCommandIndex.js";
export type { SlashCommandSearchFields } from "./lib/slashCommandIndex.js";
export {
  WORKSPACE_SLASH_COMMAND_PATH,
  SLASH_COMMAND_NAME,
  isWorkspaceSlashCommandPath,
  workspaceSlashCommandStem,
  isValidSlashCommandName,
} from "./lib/workspaceSlashPath.js";
export {
  parseWorkflowAliases,
  parseWorkspaceWorkflowCommand,
} from "./lib/workspaceWorkflowCommand.js";
export type { ParsedWorkspaceWorkflow } from "./lib/workspaceWorkflowCommand.js";
export type { SafetyResult } from "./lib/pathSafety.js";
export {
  resolveCompactionSpecFromContext,
} from "./lib/compactionSpec.js";
export type { CompactionPrefs, CompactionSpec } from "./lib/compactionSpec.js";
export {
  CONFIGURED_LOCAL_CATALOG_IDS,
  isConfiguredLocalCatalogId,
  toChatCompletionsUrl,
  describeUnresolvedIsanAgentTarget,
} from "./lib/isanagentTargetChrome.js";
export type { ConfiguredLocalCatalogId } from "./lib/isanagentTargetChrome.js";
export { hasAnyProviderKey } from "./lib/providerKeysChrome.js";
export type { ProviderKeySupport } from "./lib/providerKeysChrome.js";
export { joinWorkspaceRelativePath } from "./lib/joinWorkspacePath.js";
export { parseTodoWriteItems } from "./lib/todoWriteItems.js";
export type { ParsedTodoWriteItem } from "./lib/todoWriteItems.js";
export { newNativeMessageId } from "./lib/nativeMessageId.js";
export { mapBackendMessageToTranscript } from "./lib/backendMessageMap.js";
export type {
  BackendChatMessage,
  BackendTranscriptMessage,
  TranscriptPart,
  TranscriptTextPart,
  TranscriptToolPart,
} from "./lib/backendMessageMap.js";
export { resolveConfiguredLocalTargetCandidate } from "./lib/configuredLocalTarget.js";
export { mergeRecoveredSessions } from "./lib/mergeRecoveredSessions.js";
export {
  backendSessionTitle,
  displaySessionTitle,
  DEFAULT_SESSION_TITLE,
} from "./lib/backendSessionTitle.js";
export { newUntitledSessionMeta } from "./lib/newSessionMeta.js";
export { isUntitledSessionTitle } from "./lib/isUntitledSessionTitle.js";
export {
  filterDeletedSessions,
  appendDeletedSessionId,
} from "./lib/filterDeletedSessions.js";
export {
  resolveActiveSessionOnHydrate,
  createUntitledSessionMeta,
} from "./lib/resolveActiveSessionOnHydrate.js";
export { insertSessionAfterActive } from "./lib/insertSessionAfterActive.js";
export { renameSessionInList } from "./lib/renameSessionInList.js";
export type { SessionTitleMeta } from "./lib/renameSessionInList.js";
export { applySessionWorkspaceTarget } from "./lib/sessionWorkspaceTarget.js";
export {
  removeSessionFromList,
  nextActiveIdAfterDelete,
} from "./lib/removeSessionFromList.js";
export { maybeDeriveSessionTitleList } from "./lib/maybeDeriveSessionTitle.js";
export { cutThroughNthUserMessage } from "./lib/cutThroughNthUser.js";
export {
  formatEnvBlock,
  buildEnvBlockFromFacts,
  buildIsolatedWorktreeEnvBlock,
  prependEnvBlockToText,
} from "./lib/envBlock.js";
export type { LiveEnvFacts } from "./lib/envBlock.js";
export type { MessageWithRole } from "./lib/cutThroughNthUser.js";
export type { DerivableSession } from "./lib/maybeDeriveSessionTitle.js";
export type {
  SessionWorkspaceTarget,
  SessionWorkspaceFields,
} from "./lib/sessionWorkspaceTarget.js";
export type { SessionIdTitle } from "./lib/resolveActiveSessionOnHydrate.js";
export type {
  UntitledSessionMeta,
  UntitledSessionMetaSeed,
} from "./lib/newSessionMeta.js";
export {
  sessionListWorkspaceTargets,
  sessionWorkspacePathForId,
} from "./lib/sessionWorkspaceTargets.js";
export type { SessionWorkspacePathLike } from "./lib/sessionWorkspaceTargets.js";
export { fallbackSpecFromTarget } from "./lib/fallbackSpec.js";
export type {
  ResolvedProviderTarget,
  FallbackProviderSpec,
} from "./lib/fallbackSpec.js";
export { headersInitToRecord } from "./lib/headersInit.js";
export type { FlatHeaders } from "./lib/headersInit.js";
export { resolveCloudModelTarget } from "./lib/cloudModelTarget.js";
export type { CloudModelCatalogEntry } from "./lib/cloudModelTarget.js";
export {
  toggleIdInList,
  pushRecentId,
  sameIdSequence,
} from "./lib/modelListChrome.js";
export { appendSlashCommandFocus } from "./lib/slashCommandFocus.js";
export {
  INIT_WORKSPACE_PROMPT,
  SLASH_COMMAND_PROMPTS,
  promptForSlashCommand,
} from "./lib/slashCommandPrompt.js";
export { parseComposerSlashLead } from "./lib/parseComposerSlashLead.js";
export type { ComposerSlashLead } from "./lib/parseComposerSlashLead.js";
export { findAgentByIdOrName } from "./lib/findAgentByName.js";
export type { NamedAgent } from "./lib/findAgentByName.js";
export { isPlanModeOffTail } from "./lib/planModeTail.js";
export {
  planModeOnToast,
  planModeOffToast,
  planModeToggleToast,
} from "./lib/planModeToast.js";
export {
  startedNewChatToast,
  openedChatSessionsToast,
  renameUsageToast,
  renamedActiveChatToast,
  retryingLastRequestToast,
  cancellationRequestedToast,
  compactionRequestedToast,
  openedRunDetailsToast,
  openedChangeReviewToast,
} from "./lib/slashSessionToast.js";
export {
  openedOperationsWorkToast,
  openedOperationsInboxToast,
  openedOperationsScheduledToast,
  openedModelSettingsToast,
  openedPermissionSettingsToast,
  openedMcpSettingsToast,
  openedSkillsToast,
  openedContextSettingsToast,
} from "./lib/slashSettingsToast.js";
export {
  utf8StringToBytes,
  uint8ArrayToBytes,
  arrayBufferToBytes,
  arrayBufferViewToBytes,
} from "./lib/bodyBytes.js";
export {
  requestUrlToString,
  requestMethodFromInit,
} from "./lib/requestInitChrome.js";
export { toIsanAgentTargetResolution } from "./lib/isanAgentTargetResult.js";
export type { IsanAgentTargetResolution } from "./lib/isanAgentTargetResult.js";
export {
  switchedAgentToast,
  agentSettingsToast,
} from "./lib/slashToastChrome.js";
export { hasSlashCommandTail } from "./lib/slashRenameTail.js";









export type {
  RecoverableSessionMeta,
  BackendSessionRow,
} from "./lib/mergeRecoveredSessions.js";

export type {
  ConfiguredLocalTargetCandidate,
  ResolvedConfiguredLocalTarget,
} from "./lib/configuredLocalTarget.js";







export {
  checkReadable,
  checkWritable,
  checkReadableCanonical,
  checkWritableCanonical,
  checkShellCommand,
} from "./lib/pathSafety.js";



export {
  newAgentId,
  findAgentById,
  applyAgentOverride,
  diffAgentAgainstBase,
  type AgentEditableFields,
} from "./lib/agentChrome.js";






export {
  AiChatTranscriptFrame,
  type AiChatTranscriptFrameProps,
} from "./components/AiChatTranscriptFrame.js";

export {
  AiDisplayTranscriptList,
  type AiDisplayTranscriptListProps,
} from "./components/AiDisplayTranscriptList.js";

export {
  AiDisplayMessageBubble,
  displayMessageElementId,
  type AiDisplayMessageBubbleProps,
} from "./components/AiDisplayMessageBubble.js";

export {
  AiDisplayMessageEditForm,
  type AiDisplayMessageEditFormProps,
} from "./components/AiDisplayMessageEditForm.js";

export {
  AiDisplayMessageBodyExtras,
  type AiDisplayMessageBodyExtrasProps,
} from "./components/AiDisplayMessageBodyExtras.js";

export {
  AiDisplayMessageContent,
  type AiDisplayMessageContentProps,
} from "./components/AiDisplayMessageContent.js";

export {
  AiDisplayMessageActions,
  type AiDisplayMessageActionsProps,
} from "./components/AiDisplayMessageActions.js";

export {
  AiUserTurnBody,
  type AiUserTurnBodyProps,
} from "./components/AiUserTurnBody.js";

export {
  AiSdkAssistantGroups,
  type AiSdkAssistantGroupsProps,
} from "./components/AiSdkAssistantGroups.js";

export {
  AiSdkUiPartSwitch,
  type AiSdkUiPartSwitchProps,
} from "./components/AiSdkUiPartSwitch.js";

export {
  AiSdkToolPartSwitch,
  type AiSdkToolPartSwitchProps,
} from "./components/AiSdkToolPartSwitch.js";

export {
  isStandaloneReadToolPart,
  resolveAssistantRunActionMode,
  shouldShowAssistantRunActions,
  type AssistantRunActionMode,
} from "./lib/chatSdkAssistantChrome.js";

export {
  buildAssistantSdkGroupsState,
  type AssistantSdkGroupsState,
} from "./lib/assistantSdkGroupsState.js";

export {
  AiAssistantRunActions,
  type AiAssistantRunActionsProps,
} from "./components/AiAssistantRunActions.js";

export {
  buildAiChatViewRowMeta,
  type AiChatViewMessageLike,
  type AiChatViewRowMeta,
} from "./lib/aiChatViewModel.js";

export {
  AiChatViewFrame,
  type AiChatViewFrameProps,
} from "./components/AiChatViewFrame.js";

export {
  isSdkToolPart,
  mapSdkToolApprovalPart,
  mapSdkToolCardPart,
  sdkToolName,
  type SdkToolApprovalView,
  type SdkToolCardView,
  type SdkToolPartLike,
} from "./lib/sdkToolPartMap.js";

export {
  classifySdkUiPart,
  sdkPartText,
  type SdkUiPartKind,
  type SdkUiPartLike,
} from "./lib/sdkUiPartKind.js";

export {
  mapSdkUiPartView,
  type SdkUiPartReasoningView,
  type SdkUiPartTextView,
  type SdkUiPartToolView,
  type SdkUiPartUnknownView,
  type SdkUiPartView,
} from "./lib/sdkUiPartView.js";

export {
  chatDisplayBubbleClassName,
  chatDisplayBubbleModifier,
  chatDisplayRoleLabel,
} from "./lib/chatDisplayChrome.js";

export {
  canCopyDisplayMessage,
  hasDisplayMessageActions,
  lastAssistantMessageId,
  resolveDisplayMessageActions,
  type DisplayMessageActionFlags,
  type DisplayMessageActionInput,
} from "./lib/chatDisplayActions.js";

export {
  displayCopyActionLabel,
  displayDiffReviewTitle,
  displayOpenDiffActionTitle,
  displayOpenFileActionTitle,
  displayOpeningActionLabel,
} from "./lib/chatDisplayActionCopy.js";

export {
  displayToolGroupIconKey,
  type DisplayToolGroupIconKey,
} from "./lib/displayToolGroupIcon.js";

export {
  ACCEPTED_COMPOSER_FILES,
  ACCEPTED_FILES,
  boundContextText,
  buildTextContextAttachment,
  estimateComposerContextTokens,
  hasComposerDraft,
  hasNativeBinaryAttachment,
  MAX_CONTEXT_TEXT_CHARS,
  MAX_TEXT_INLINE,
  upsertComposerAttachment,
  type ComposerFileAttachment,
  type ComposerFileKind,
} from "./lib/composerAttachments.js";

export {
  addPickedSnippet,
  composePromptWithSnippets,
  expandSnippetTokens,
  findSnippets,
  insertSnippetHandle,
  isValidHandle,
  mergeSnippetCatalogs,
  normalizeHandle,
  parseWorkspaceSnippetsJson,
  removePickedSnippet,
  type ComposerSnippet,
  type PickedComposerSnippet,
} from "./lib/composerSnippets.js";

export {
  composeComposerSubmitText,
  extractComposerMultimodalParts,
  formatComposerFileBlocks,
  mergeSnippetBlocks,
  type ComposerMultimodalParts,
} from "./lib/composerSubmitCompose.js";

export {
  MAX_PDF_INLINE_BYTES,
  appendUniqueByKey,
  applyComposerSlashOutcome,
  basenameForAttach,
  browserFileToAttachment,
  buildComposerCommandSource,
  classifyBrowserFile,
  removeAcceptedItems,
  selectionToComposerAttachment,
  type BrowserFileClass,
  type ComposerSlashOutcome,
} from "./lib/composerDraft.js";

export {
  planComposerSubmit,
  type ComposerSlashResolver,
  type ComposerSubmitPlan,
  type ComposerSubmitSnapshot,
} from "./lib/composerSubmitPlan.js";

export {
  clearComposerDraftAfterAccept,
  type ComposerDraftState,
} from "./lib/composerDraftClear.js";

export {
  mapComposerSubmitPlanToHostIntent,
  type ComposerSubmitHostContext,
  type ComposerSubmitHostIntent,
} from "./lib/composerSubmitHostIntent.js";

export {
  executeComposerSubmit,
  type ComposerSubmitExecuteResult,
  type ComposerSubmitHostHandlers,
} from "./lib/composerSubmitExecute.js";

export {
  useComposerController,
  type ComposerCommandPick,
  type ComposerController,
  type UseComposerControllerOptions,
} from "./hooks/useComposerController.js";

export {
  useComposerSuggestionList,
  type ComposerSuggestionListController,
  type UseComposerSuggestionListOptions,
} from "./hooks/useComposerSuggestionList.js";

export {
  useDisplayMessageInteractionState,
  type DisplayMessageInteractionState,
} from "./hooks/useDisplayMessageInteractionState.js";

export { autoresizeTextarea } from "./lib/autoresizeTextarea.js";

export {
  hrefToFilePath,
  isWebHref,
  looksLikePath,
  pathToFileUri,
  resolveWorkspacePath,
  toolBubbleContent,
} from "./lib/chatHref.js";

export {
  recoveryCopy,
  runBlockedMessageFromEvent,
  runWarningMessageFromEvent,
  shouldShowChangeReviewBanner,
  shouldShowRunRecovery,
  type RecoveryChromeFlags,
} from "./lib/runLifecycleChrome.js";

export {
  countPendingEditDiffs,
  isEditDiffMessage,
  lastEditDiffMessage,
  lastEditDiffMessageIndex,
  type EditDiffMessageLike,
} from "./lib/editDiffMessagePolicy.js";

export {
  activityFromMessages,
  buildRunInspectorSections,
  changesFromMessages,
  hasRunInspectorContent,
  latestTodosFromMessages,
  mapApprovalsToInspectorItems,
  type ActivityInspectorEventView,
  type ApprovalsInspectorItemView,
  type ChangesInspectorItemView,
  type InspectorTodosModelView,
  type PendingApprovalLike,
  type RunInspectorMessageLike,
  type RunInspectorSectionsModelView,
  type TodosInspectorItemView,
} from "./lib/runInspectorSections.js";

export {
  dismissAllChangeReviewIds,
  dismissChangeReviewId,
  listChangeReviewItems,
  planLineDiffStats,
  type ChangeReviewItem,
  type ChangeReviewSourceMessage,
} from "./lib/changeReviewPolicy.js";

export {
  buildRunOverviewMetrics,
  canShowRunDetailsChrome,
  countToolMessages,
  runDetailsStatus,
  runDetailsStepLabel,
  runDetailsSubtitle,
  runDetailsTokenLabel,
  type RunDetailsChromeInput,
  type RunDetailsMessage,
  // RunDetailsStatus / RunOverviewMetric already exported from components.
} from "./lib/runDetailsChrome.js";

export {
  deriveAgentStatusMeta,
  formatAgentStepLabel,
  isRecoverableRunAttention,
  type AgentStatusChromeInput,
  type AgentStatusMessage,
} from "./lib/agentStatusChrome.js";

export {
  ZERO_RUN_USAGE,
  accumulateRunUsage,
  formatRunTokenLabel,
  formatTokenCount,
  usageDeltaFromPayload,
  type RunUsageTotals,
  type UsageDelta,
} from "./lib/usageMeterChrome.js";

export {
  ALTAI_SURFACES,
  nextAltaiSurface,
  type AltaiSurfaceId,
} from "./lib/surfaceTabsChrome.js";

export {
  compactHostStatusLabel,
  nextSurfaceAfterSettingsToggle,
  settingsGearPressed,
  shouldShowSurfaceTextTabs,
  type ShellSurface,
} from "./lib/shellChrome.js";

export {
  isEscapeDismissKey,
  isTextEditingKeyboardTarget,
  shouldDismissSidePanelOnEscape,
} from "./lib/chatKeyboardChrome.js";

export { canMountReplayControl } from "./lib/replayChrome.js";

export { formatDiagnosticClipboardText } from "./lib/waitShellChrome.js";

export {
  canMountPermissionModeSwitcher,
  type PermissionSwitcherFlags,
} from "./lib/permissionModeChrome.js";

export {
  canMountSkillsStatus,
  skillsSummaryCopy,
  sortSkillsForDisplay,
  type SkillView,
} from "./lib/skillsStatusChrome.js";

export {
  canMountMcpStatus,
  mcpServerStatusCopy,
  mcpSummaryCopy,
  sortMcpServersForDisplay,
  type McpServerView,
} from "./lib/mcpStatusChrome.js";

export {
  resolveSessionRemoveMode,
  sessionRemoveErrorMessage,
  type SessionRemoveMode,
} from "./lib/sessionMutateChrome.js";

export {
  advanceCaretAfterDraftChange,
  resolveComposerCaret,
} from "./lib/composerCaretChrome.js";

export {
  SETTINGS_HUB_SECTION_DEFS,
  listSettingsHubNav,
  listSettingsHubSections,
  normalizeSettingsHubSection,
  type SettingsHubCapabilityFlags,
  type SettingsHubNavItem,
  type SettingsHubSectionId,
} from "./lib/settingsHubChrome.js";

export {
  filterSettingsNav,
  settingsSectionSearchScore,
} from "./lib/settingsSearchChrome.js";

export {
  formatTranscriptForCopy,
  roleLabelForCopy,
  type TranscriptCopyLine,
} from "./lib/transcriptCopyChrome.js";

export {
  basenamePath,
  canMountProjectTarget,
  projectTargetFromWorkspace,
  retainPreferredRoot,
  type ProjectTargetView,
  type WorkspaceTargetInfo,
} from "./lib/projectTargetChrome.js";

export {
  AUTO_MODEL_ID,
  canMountModelPicker,
  filterModels,
  modelIdForStartRun,
  modelTriggerLabel,
  resolveSelectedModelId,
  type ModelPickerFlags,
} from "./lib/modelPickerChrome.js";

export {
  COMPOSER_AGENT_PROFILES,
  DEFAULT_COMPOSER_AGENT_ID,
  applyAgentPromptPrefix,
  canMountAgentPicker,
  resolveComposerAgent,
  type ComposerAgentIconId,
  type ComposerAgentProfile,
} from "./lib/agentPickerChrome.js";

export {
  filterModelsByProviderKeys,
  mergeModelCatalog,
  providerIdForModel,
  type CatalogModelEntry,
} from "./lib/modelCatalogChrome.js";

export {
  canMountProviderStatus,
  displayProviderLabel,
  firstConnectableProvider,
  hasConnectedProvider,
  isKeylessProvider,
  mergeProviderCatalog,
  providerConsoleUrl,
  providerRequiresBaseUrl,
  providerStatusCopy,
  shouldShowProviderConnectBanner,
  sortProvidersForDisplay,
  type KnownProviderEntry,
  type ProviderStatusFlags,
} from "./lib/providerStatusChrome.js";

export {
  mergeSnippetCatalogFromPrefs,
  prefsToComposerSnippets,
  type SnippetPrefEntry,
} from "./lib/settingsSnippetsChrome.js";

export {
  canMountCheckpointChrome,
  canRestoreCheckpoint,
  preferredCheckpointLabel,
  toCheckpointMenuItems,
  type CheckpointChromeFlags,
} from "./lib/checkpointChrome.js";

export {
  isPlanPermissionMode,
  latestTodoItemsFromDisplayMessages,
  permissionModeAfterExitPlan,
} from "./lib/chatPlanChrome.js";

export {
  formatGitDiffSummary,
  formatTerminalAttachText,
  type GitDiffFileLine,
} from "./lib/attachFormatChrome.js";

export {
  canMountWorkspaceTopbar,
  workspaceTopbarInboxOpen,
  workspaceTopbarWorkOpen,
  type WorkspaceTopbarFlags,
} from "./lib/workspaceTopbarChrome.js";

export {
  chatLineKind,
  shouldShowFlatLogEmptyHome,
  type ChatLineKind,
} from "./lib/chatLineKind.js";

export {
  SLASH_COMMAND_INDEX,
  findSlashCommands,
  formatSlashHelpDigest,
  resolveSlashCommand,
  tryRunSlashCommand,
  type SlashCommandBehavior,
  type SlashCommandCategory,
  type SlashCommandMeta,
  type SlashHostAction,
  type SlashOutcome,
} from "./lib/composerSlashCommands.js";

export {
  addContextItem,
  clipContextText,
  composeRunPrompt,
  countLines,
  formatTextContextBlocks,
  listOpenableContextItems,
  newContextItemId,
  removeContextItem,
  resolveContextOpenUri,
  toComposerAttachFiles,
  toContextChips,
  toRunAttachments,
  type ComposerContextItem,
  type OpenableContextItem,
} from "./lib/composerContextChrome.js";

export {
  buildDiffContextItem,
  buildFileContextItem,
  buildSelectionContextItem,
  buildTerminalContextItem,
} from "./lib/composerAttachChrome.js";

export {
  buildOpenChatFocus,
  chatFocusStatusLine,
  type OpenChatFocus,
  type OpenChatTarget,
} from "./lib/openChatDeepLink.js";

export {
  composeTaskPromptWithSkills,
  toggleTaskSkillSelection,
  validateTaskRunDraft,
  type TaskRunDraft,
  type TaskRunDraftResult,
} from "./lib/taskRunDraft.js";

export {
  AUTOMATION_INTERVAL_PRESETS,
  newAutomationOwnerChatId,
  validateAutomationDraft,
  type AutomationDraft,
  type AutomationDraftResult,
  type AutomationScheduleDraft,
} from "./lib/automationDraft.js";

export {
  formatSessionMessageLine,
  transcriptLinesFromMessages,
  type TranscriptMessage,
} from "./lib/sessionTranscript.js";

export {
  EMPTY_OPERATIONS_DATA,
  buildOperationsOverview,
  countOperationsAttention,
  destinationForOverviewMetric,
  destinationForOverviewRowId,
  overviewActiveRunId,
  overviewFailedRunId,
  overviewUnreadInboxId,
  withOverviewMetricNavigation,
  withOverviewRowNavigation,
  type OperationsOverviewData,
  type OperationsOverviewViewModel,
  type OverviewNavFlags,
  type OverviewRowDestination,
} from "./lib/operationsOverview.js";

export {
  fetchOperationsAttentionCount,
  shouldRefreshAttentionOnEvent,
  type AttentionPollFlags,
  type AttentionPollSources,
} from "./lib/operationsAttentionPoll.js";

export {
  ALTAI_RECOVERY_COMMANDS,
  isAltaiRecoveryCommand,
  listRecoveryActions,
  type AltaiRecoveryCommand,
  type RecoveryAction,
} from "./lib/hostRecoveryCommands.js";

export {
  attentionStatusBarCommand,
  parseAttentionReportParams,
} from "./lib/attentionReport.js";

export {
  joinSelectionTexts,
  type SelectionRangeInput,
} from "./lib/selectionJoin.js";

export {
  hostStatusPillLabel,
  shouldShowHostSubtitle,
} from "./lib/hostChromeLabels.js";

export { hostConnectingProgressPresentation } from "./lib/hostConnectingProgress.js";

export {
  extractHostErrorCode,
  formatHostUserError,
  isJournalUnavailableError,
} from "./lib/hostUserError.js";

export type { HostLifecycleStatus } from "./lib/hostStatusNotify.js";
export { shouldNotifyHostRecovered } from "./lib/hostStatusNotify.js";

export type { HostErrorAction } from "./lib/hostErrorActions.js";
export {
  HOST_ERROR_ACTION_LABELS,
  hostErrorActionCommands,
  hostRecoveredActionCommands,
  shouldPromptHostErrorActions,
} from "./lib/hostErrorActions.js";

export {
  enabledExcludePatterns,
  searchExcludeGlobFromSettings,
} from "./lib/searchExcludeGlobs.js";

export {
  PREFERRED_HOST_ROOT_STATE_KEY,
  readPreferredHostRootFromState,
  retainPreferredHostRootUri,
} from "./lib/preferredHostRoot.js";

export {
  MAX_PROVIDER_BASE_URL_CHARS,
  normalizeProviderBaseUrl,
} from "./lib/providerBaseUrl.js";
// providerRequiresBaseUrl is exported once via providerStatusChrome (A6.90);
// providerBaseUrl.ts keeps the same pure implementation for host mirrors (A6.116).

export type { DiagnosticLike, FileDiagnosticsBundle } from "./lib/problemsContext.js";
export {
  formatProblemsBundles,
  formatProblemsContextText,
} from "./lib/problemsContext.js";

export { isVirtualOnlyWorkspace } from "./lib/virtualWorkspace.js";

export type { HostStatusBarInput, HostStatusBarPresentation } from "./lib/hostStatusBar.js";
export { hostStatusBarPresentation } from "./lib/hostStatusBar.js";

export type { OpenChatWithFilePayload } from "./lib/fileDeepLink.js";

export type { OpenChatWithSelectionPayload } from "./lib/selectionDeepLink.js";

export {
  includeUriInWorkspaceProblemsAttach,
  isWorkspaceNotTrustedError,
} from "./lib/workspaceTrustAttach.js";

export type { OpenSettingsPayload } from "./lib/settingsDeepLink.js";

export type {
  OpenOperationsPayload,
  OperationsDeepLinkView,
  OperationsDeepLinkWorkHubView,
} from "./lib/operationsDeepLink.js";

export {
  COMPOSER_DRAFT_DEBOUNCE_MS,
  shouldPersistComposerDraftImmediately,
} from "./lib/composerDraftPersist.js";

export { withAssetCacheBust } from "./lib/assetCacheBust.js";
export { createSecureId, getSecureRandomBytes } from "./lib/secureRandom.js";
export { createNonce } from "./lib/cspNonce.js";
export { createMessageId } from "./lib/messageId.js";
export { recoveryHintForDiagnosticCode } from "./lib/hostRecovery.js";

export type {
  PersistedAltaiSurface,
  PersistedHostStatus,
  PersistedOperationsView,
  PersistedWebviewState,
  PersistedWorkHubView,
} from "./lib/webviewState.js";

export type {
  ChatAnnouncePref,
  ExtensionPreferences,
  ExtensionSettingKey,
  FocusRingPref,
  ReduceMotionPref,
  SnippetPref,
} from "./lib/extensionPreferences.js";
export {
  EXTENSION_SETTING_KEYS,
  coerceExtensionPreferences,
  defaultExtensionPreferences,
  isExtensionSettingKey,
  isValidSettingValue,
  parseSnippetsJson,
  serializeSnippets,
} from "./lib/extensionPreferences.js";

export {
  HISTORY_PANEL_MAX_WIDTH,
  HISTORY_PANEL_MIN_WIDTH,
  HISTORY_PANEL_WIDTH_KEY,
  INSPECTOR_PANEL_MAX_WIDTH,
  INSPECTOR_PANEL_MIN_WIDTH,
  INSPECTOR_PANEL_WIDTH_KEY,
  clampPanelWidth,
  parsePanelWidth,
  serializePanelWidth,
} from "./lib/sidePanelWidth.js";

export type {
  SidePanelChromeSurface,
  SidePanelOpenEventDetail,
  SidePanelOpenResolution,
} from "./lib/sidePanelSurface.js";

export type { StringKeyStorage } from "./lib/sidePanelWidthStorage.js";

export type {
  OperationsOpenIntent,
  OperationsOpenView,
  OperationsOpenWorkHubView,
} from "./lib/operationsOpenEvent.js";
export { buildOperationsOpenIntent } from "./lib/operationsOpenEvent.js";
export {
  readPanelWidthFromStorage,
  writePanelWidthToStorage,
} from "./lib/sidePanelWidthStorage.js";
export {
  closeChatTabSelection,
  openIdsAfterNewChat,
  reconcileOpenChatTabIds,
  resolveSidePanelOpenEvent,
  toggleSidePanelChromeSurface,
} from "./lib/sidePanelSurface.js";
export {
  MAX_COMPOSER_DRAFT_CHARS,
  MAX_PREFERRED_ROOT_URI_CHARS,
  mergePersistedWebviewState,
  normalizeComposerDraft,
  normalizePreferredRootUri,
  parsePersistedWebviewState,
} from "./lib/webviewState.js";
export {
  buildOpenOperationsPayload,
  parseOpenOperationsPayload,
} from "./lib/operationsDeepLink.js";
export {
  buildOpenSettingsPayload,
  parseOpenSettingsPayload,
} from "./lib/settingsDeepLink.js";
export {
  buildOpenChatWithSelectionPayload,
  parseOpenChatWithSelectionPayload,
} from "./lib/selectionDeepLink.js";
export {
  buildOpenChatWithFilePayload,
  parseOpenChatWithFilePayload,
} from "./lib/fileDeepLink.js";

export {
  fileUriToPath,
  isHttpUrl,
  segmentChatContent,
  segmentTextWithLinks,
  type ChatContentSegment,
} from "./lib/chatContentSegments.js";

export {
  applyInteractivePrompt,
  interactivePromptFromAgentEvent,
  normalizeAgentEventType,
  type InteractivePrompt,
  type PendingClarificationPrompt,
  type PendingEditDiff,
  type PendingToolApproval,
} from "./lib/interactivePrompt.js";

export {
  buildTranscriptPartGroups,
  cmdSummaryForToolPart,
  formatGroupPreview,
  groupKindFor,
  groupKindFromToolName,
  normalizeToolName,
  pathBasename,
  readPathFromToolPart,
  toolNameOf,
  transcriptPartKey,
  uniqueReadPaths,
  uniqueSummaries,
  webSummaryForToolPart,
  type ToolLikePart,
  type TranscriptGroupKind,
  type TranscriptPartGroup,
} from "./lib/transcriptToolGroups.js";

export {
  buildDisplayTranscriptBlocks,
  groupCountLabel,
  groupLabel,
  groupPreview,
  toolGroupKindFor,
  type DisplayToolGroupKind,
  type DisplayTranscriptBlock,
  type TranscriptDisplayMessage,
} from "./lib/displayTranscriptBlocks.js";

export {
  ConversationOwnerSection,
  type ConversationOwnerSectionProps,
} from "./components/ConversationOwnerSection.js";

export {
  SurfaceInlineError,
  type SurfaceInlineErrorProps,
} from "./components/SurfaceInlineError.js";

export {
  SurfaceLoadingState,
  type SurfaceLoadingStateProps,
} from "./components/SurfaceLoadingState.js";

export {
  SurfaceListGroup,
  type SurfaceListGroupProps,
} from "./components/SurfaceListGroup.js";

export {
  RunActionRequiredSection,
  type RunActionRequiredSectionProps,
} from "./components/RunActionRequiredSection.js";

export {
  formatRelativeTime,
  humanize,
} from "./lib/inboxFormat.js";

/** Re-export contract types so consumers can depend primarily on agent-ui. */
export type {
  Capabilities,
  CapabilityId,
  HostPorts,
} from "@altai/host-contract";

export {
  createCapabilities,
  isCapabilityEnabled,
  capabilityForAction,
} from "@altai/host-contract";

export {
  compactionRequestedLabel,
  compactionRequestedDetail,
  compactionFailedLabel,
  compactionFailedDetail,
} from "./lib/compactionToast.js";

export {
  byNewestCreatedAt,
  isWaitingTicketStatus,
  isTerminalJobState,
  buildNotificationInboxView,
} from "./lib/notificationInboxView.js";
export type {
  InboxNotificationRow,
  InboxBackgroundJobRow,
  InboxClarificationTicketRow,
  NotificationInboxViewModel,
} from "./lib/notificationInboxView.js";

export {
  editProposalInputFromQueued,
  markPlanEditAppliedState,
} from "./lib/planQueueChrome.js";
export type { QueuedPlanEditLike } from "./lib/planQueueChrome.js";

export {
  errorMessageFromUnknown,
  mutationKey,
  normalizedWorkspacePath,
} from "./lib/errorMessage.js";

export {
  sortAutomationItemsById,
  indexLatestCronJobsByAutomationId,
} from "./lib/automationListChrome.js";
export type { BackgroundJobLike } from "./lib/automationListChrome.js";

export {
  withPendingStarted,
  withPendingEnded,
} from "./lib/pendingIdsChrome.js";
export type { PendingIdMap } from "./lib/pendingIdsChrome.js";
