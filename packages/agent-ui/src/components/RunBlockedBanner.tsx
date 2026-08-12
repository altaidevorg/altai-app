export type RunBlockedBannerProps = {
  message: string;
};

/**
 * Inline blocked notice under the Details strip.
 */
export function RunBlockedBanner({ message }: RunBlockedBannerProps) {
  return (
    <section className="border-b border-destructive/20 bg-destructive/[0.05] px-2.5 py-2 text-[11px] leading-relaxed text-destructive">
      <div className="mb-0.5 text-[10px] font-medium uppercase tracking-wide">
        Run blocked
      </div>
      {message}
    </section>
  );
}
