/**
 * Pure helpers for capability-gating provider status chrome (A6.90).
 * Hosts supply the known-provider catalog; secrets never live here.
 */

import type { ProviderStatus } from "@altai/host-contract";

export type KnownProviderEntry = {
  id: string;
  label: string;
  /** Optional hint for expected key shape (not enforced client-side). */
  keyHint?: string;
  /** Console URL for “Get key”. */
  consoleUrl?: string;
  /** Requires an OpenAI-compatible HTTP(S) base URL before the key. */
  requiresBaseUrl?: boolean;
  /** No API key required (local runtimes). */
  keyless?: boolean;
};

export type ProviderStatusFlags = {
  providerStatus: boolean;
};

/**
 * Show provider status chrome only when the full status/connect/clear capability
 * is advertised (no dead Connect buttons).
 */
export function canMountProviderStatus(flags: ProviderStatusFlags): boolean {
  return flags.providerStatus;
}

function lookupKnown(
  catalog: readonly KnownProviderEntry[],
  id: string,
): KnownProviderEntry | undefined {
  return catalog.find((entry) => entry.id === id);
}

/**
 * Merge host provider status with the known catalog so every BYOK target is
 * visible even when the host returns a partial list.
 */
export function mergeProviderCatalog(
  host: readonly ProviderStatus[],
  knownProviders: readonly KnownProviderEntry[],
): ProviderStatus[] {
  const byId = new Map<string, ProviderStatus>();
  for (const entry of knownProviders) {
    byId.set(entry.id, {
      providerId: entry.id,
      label: entry.label,
      connected: Boolean(entry.keyless),
    });
  }
  for (const status of host) {
    const id = status.providerId.trim();
    if (!id) {
      continue;
    }
    const known = lookupKnown(knownProviders, id);
    const prev = byId.get(id);
    byId.set(id, {
      providerId: id,
      connected: status.connected,
      label: status.label?.trim() || known?.label || prev?.label || id,
      ...(status.error ? { error: status.error } : {}),
    });
  }
  return sortProvidersForDisplay([...byId.values()], knownProviders);
}

/**
 * Sort providers: disconnected and errored first so attention is visible.
 */
export function sortProvidersForDisplay(
  providers: readonly ProviderStatus[],
  knownProviders: readonly KnownProviderEntry[] = [],
): ProviderStatus[] {
  return [...providers].sort((a, b) => {
    const score = (item: ProviderStatus): number => {
      if (item.error) {
        return 0;
      }
      if (!item.connected) {
        return 1;
      }
      return 2;
    };
    const delta = score(a) - score(b);
    if (delta !== 0) {
      return delta;
    }
    return displayProviderLabel(a, knownProviders).localeCompare(
      displayProviderLabel(b, knownProviders),
    );
  });
}

export function displayProviderLabel(
  provider: ProviderStatus,
  knownProviders: readonly KnownProviderEntry[] = [],
): string {
  const label = provider.label?.trim();
  if (label && label.length > 0) {
    return label;
  }
  return (
    lookupKnown(knownProviders, provider.providerId)?.label ??
    provider.providerId
  );
}

/** Short status copy for list rows. */
export function providerStatusCopy(
  provider: ProviderStatus,
  knownProviders: readonly KnownProviderEntry[] = [],
): string {
  if (provider.error) {
    return provider.error;
  }
  if (lookupKnown(knownProviders, provider.providerId)?.keyless) {
    return provider.connected ? "Local (no key)" : "Not ready";
  }
  return provider.connected ? "API key saved" : "Not connected";
}

export function providerConsoleUrl(
  providerId: string,
  knownProviders: readonly KnownProviderEntry[],
): string | undefined {
  return lookupKnown(knownProviders, providerId)?.consoleUrl;
}

export function providerRequiresBaseUrl(
  providerId: string,
  knownProviders: readonly KnownProviderEntry[],
): boolean {
  return Boolean(lookupKnown(knownProviders, providerId)?.requiresBaseUrl);
}

export function isKeylessProvider(
  provider: {
    providerId?: string;
    keyless?: boolean;
  },
  knownProviders: readonly KnownProviderEntry[] = [],
): boolean {
  if (provider.keyless) {
    return true;
  }
  if (provider.providerId) {
    return Boolean(lookupKnown(knownProviders, provider.providerId)?.keyless);
  }
  return false;
}

/** True when at least one non-keyless provider reports connected credentials. */
export function hasConnectedProvider(
  providers: readonly ProviderStatus[],
  knownProviders: readonly KnownProviderEntry[] = [],
): boolean {
  return providers.some(
    (provider) =>
      provider.connected &&
      !lookupKnown(knownProviders, provider.providerId)?.keyless,
  );
}

/**
 * Compact connect banner belongs above the composer when status is ready and
 * no usable provider connection is available yet.
 */
export function shouldShowProviderConnectBanner(input: {
  providerStatus: boolean;
  ready: boolean;
  providers: readonly ProviderStatus[];
  knownProviders?: readonly KnownProviderEntry[];
}): boolean {
  return (
    input.providerStatus &&
    input.ready &&
    !hasConnectedProvider(input.providers, input.knownProviders ?? [])
  );
}

/**
 * Prefer a disconnected cloud provider for Connect.
 */
export function firstConnectableProvider(
  providers: readonly ProviderStatus[],
  knownProviders: readonly KnownProviderEntry[] = [],
): ProviderStatus | null {
  if (providers.length === 0) {
    return null;
  }
  const sorted = sortProvidersForDisplay(providers, knownProviders);
  return (
    sorted.find(
      (provider) =>
        !provider.connected &&
        !lookupKnown(knownProviders, provider.providerId)?.keyless,
    ) ??
    sorted.find((provider) => !provider.connected) ??
    sorted[0] ??
    null
  );
}
