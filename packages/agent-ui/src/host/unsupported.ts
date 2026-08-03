/**
 * Helpers for building partial HostPorts adapters during extraction.
 * Full Desktop/VS Code adapters live in their host repos, not here.
 */

import type { HostPorts } from "@altai/host-contract";

export class HostPortUnsupportedError extends Error {
  readonly port: string;
  readonly method: string;

  constructor(port: string, method: string) {
    super(`Host port ${port}.${method} is not available on this host`);
    this.name = "HostPortUnsupportedError";
    this.port = port;
    this.method = method;
  }
}

export function unsupported(port: string, method: string): () => Promise<never> {
  return async () => {
    throw new HostPortUnsupportedError(port, method);
  };
}

type PortKey = keyof HostPorts;

/**
 * Fill every method on a port with `unsupported` throwers, then overlay
 * concrete implementations. Useful while adapters are built incrementally.
 */
export function withUnsupportedDefaults<K extends PortKey>(
  port: K,
  methods: (keyof HostPorts[K])[],
  impl: Partial<HostPorts[K]>,
): HostPorts[K] {
  const base = {} as Record<string, unknown>;
  for (const method of methods) {
    base[method as string] = unsupported(port, String(method));
  }
  return { ...base, ...impl } as unknown as HostPorts[K];
}
