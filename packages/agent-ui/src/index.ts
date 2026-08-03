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
