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
  type ChatAnnouncePref,
} from "./lib/chatTranscriptChrome.js";

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

export { isEscapeDismissKey } from "./lib/chatKeyboardChrome.js";

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
