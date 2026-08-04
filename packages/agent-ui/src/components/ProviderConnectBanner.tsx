import { Key01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type ProviderConnectBannerProps = {
  onAdd: () => void;
  message?: string;
  actionLabel?: string;
};

/**
 * Compact “connect a provider” strip shown when the host has no usable model
 * credentials. Presentational; the host owns navigation into settings.
 */
export function ProviderConnectBanner({
  onAdd,
  message = "Connect any AI provider (or use local models) - your key stays in your OS keychain.",
  actionLabel = "Connect provider",
}: ProviderConnectBannerProps) {
  return (
    <div className="altai-provider-connect-banner shrink-0 border-t border-border-subtle bg-raised">
      <div className="flex h-10 items-center justify-between gap-3 px-3 text-xs">
        <span className="text-muted-foreground">{message}</span>
        <button
          type="button"
          onClick={onAdd}
          className="inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md bg-primary px-2.5 text-[11px] font-medium text-primary-foreground transition-colors hover:bg-primary/90"
        >
          <HugeiconsIcon icon={Key01Icon} size={13} strokeWidth={1.75} />
          {actionLabel}
        </button>
      </div>
    </div>
  );
}
