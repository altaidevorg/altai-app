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
