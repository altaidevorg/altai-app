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
  AutomationCard,
  automationLastRunLabel,
  automationNextRunLabel,
  automationScheduleLabel,
  type AutomationCardProps,
  type AutomationSchedule,
} from "./components/AutomationCard.js";

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
  TaskRunConfigSection,
  type TaskRunConfigSectionProps,
} from "./components/TaskRunConfigSection.js";

export {
  RunDetailsHeader,
  type RunDetailsHeaderProps,
  type RunDetailsStatus,
} from "./components/RunDetailsHeader.js";

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
