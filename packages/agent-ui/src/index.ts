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
