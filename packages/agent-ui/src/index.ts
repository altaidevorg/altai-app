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
export { parseComposerSlashLead } from "./lib/parseComposerSlashLead.js";
export type { ComposerSlashLead } from "./lib/parseComposerSlashLead.js";
export { findAgentByIdOrName } from "./lib/findAgentByName.js";
export type { NamedAgent } from "./lib/findAgentByName.js";
export { isPlanModeOffTail } from "./lib/planModeTail.js";
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
