export type RunBlockedBannerProps = {
  message: string;
};

/**
 * Destructive banner shown when the current run is blocked by an error.
 * Host decides whether to render and supplies the error text.
 */
export function RunBlockedBanner({ message }: RunBlockedBannerProps) {
  return (
    <section className="rounded-lg border border-destructive/30 bg-destructive/[0.06] p-3 text-[10.5px] leading-relaxed text-destructive">
      <div className="mb-1 text-[9px] font-semibold uppercase tracking-wide">
        Run blocked
      </div>
      {message}
    </section>
  );
}
