import { describe, expect, it } from "vitest";
import {
  firstConnectableProvider,
  hasConnectedProvider,
  mergeProviderCatalog,
  providerStatusCopy,
  shouldShowProviderConnectBanner,
} from "../lib/providerStatusChrome.js";

const KNOWN = [
  { id: "openai", label: "OpenAI" },
  { id: "anthropic", label: "Anthropic" },
  { id: "lmstudio", label: "LM Studio", keyless: true },
] as const;

describe("providerStatusChrome", () => {
  it("surfaces the full known catalog when host is empty", () => {
    const merged = mergeProviderCatalog([], KNOWN);
    expect(merged.some((p) => p.providerId === "openai")).toBe(true);
    expect(merged.some((p) => p.providerId === "anthropic")).toBe(true);
    expect(hasConnectedProvider(merged, KNOWN)).toBe(false);
  });

  it("marks host-connected providers while keeping the rest", () => {
    const merged = mergeProviderCatalog(
      [{ providerId: "anthropic", connected: true, label: "Anthropic" }],
      KNOWN,
    );
    expect(merged.find((p) => p.providerId === "anthropic")?.connected).toBe(
      true,
    );
    expect(merged.find((p) => p.providerId === "openai")?.connected).toBe(
      false,
    );
    expect(hasConnectedProvider(merged, KNOWN)).toBe(true);
    expect(
      providerStatusCopy(
        { providerId: "anthropic", connected: true },
        KNOWN,
      ),
    ).toBe("API key saved");
  });

  it("keyless providers do not count as connected credentials", () => {
    const merged = mergeProviderCatalog([], KNOWN);
    expect(
      merged.find((p) => p.providerId === "lmstudio")?.connected,
    ).toBe(true);
    expect(hasConnectedProvider(merged, KNOWN)).toBe(false);
  });

  it("picks a connectable cloud provider first", () => {
    const merged = mergeProviderCatalog(
      [{ providerId: "lmstudio", connected: true, label: "LM Studio" }],
      KNOWN,
    );
    const first = firstConnectableProvider(merged, KNOWN);
    expect(first?.providerId).toBe("openai");
  });

  it("gates the connect banner", () => {
    expect(
      shouldShowProviderConnectBanner({
        providerStatus: true,
        ready: true,
        providers: mergeProviderCatalog([], KNOWN),
        knownProviders: KNOWN,
      }),
    ).toBe(true);
  });
});
