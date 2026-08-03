import {
  createContext,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import type {
  Capabilities,
  CapabilityId,
  HostPorts,
} from "@altai/host-contract";
import { isCapabilityEnabled } from "@altai/host-contract";

export type HostPortsContextValue = {
  ports: HostPorts;
  /** Negotiated capabilities; null until initialize completes. */
  capabilities: Capabilities | null;
};

const HostPortsContext = createContext<HostPortsContextValue | null>(null);

export type HostPortsProviderProps = {
  ports: HostPorts;
  capabilities?: Capabilities | null;
  children: ReactNode;
};

/**
 * Inject host adapters into shared UI. Desktop and VS Code each provide their
 * own `HostPorts` implementation; shared components never import Tauri/VS Code.
 */
export function HostPortsProvider({
  ports,
  capabilities = null,
  children,
}: HostPortsProviderProps) {
  const value = useMemo<HostPortsContextValue>(
    () => ({ ports, capabilities }),
    [ports, capabilities],
  );
  return (
    <HostPortsContext.Provider value={value}>
      {children}
    </HostPortsContext.Provider>
  );
}

export function useHostPortsContext(): HostPortsContextValue {
  const ctx = useContext(HostPortsContext);
  if (!ctx) {
    throw new Error(
      "useHostPortsContext requires a HostPortsProvider ancestor",
    );
  }
  return ctx;
}

export function useHostPorts(): HostPorts {
  return useHostPortsContext().ports;
}

/**
 * True when negotiated capabilities mark `id` as `available`.
 * When capabilities are not yet loaded, returns false (fail closed).
 */
export function useCapability(id: CapabilityId): boolean {
  const { capabilities } = useHostPortsContext();
  if (!capabilities) {
    return false;
  }
  return isCapabilityEnabled(capabilities, id);
}
