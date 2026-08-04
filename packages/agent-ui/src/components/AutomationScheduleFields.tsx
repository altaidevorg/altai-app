import { cn } from "../lib/cn.js";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type AutomationScheduleMode = "at" | "every";

export type AutomationScheduleFieldsProps = {
  mode: AutomationScheduleMode;
  onModeChange: (mode: AutomationScheduleMode) => void;
  atValue: string;
  onAtValueChange: (value: string) => void;
  everyMinutes: string;
  onEveryMinutesChange: (value: string) => void;
  /** Injectable clock for quick-set presets in tests. */
  nowMs?: number;
};

/** Local `datetime-local` string for a unix timestamp. */
export function localDateTimeValue(timestamp: number): string {
  const value = new Date(timestamp);
  const local = new Date(value.getTime() - value.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

/**
 * Create-automation schedule section: once/repeat mode, value inputs, and
 * quick-set presets. Host owns create submit and store writes.
 */
export function AutomationScheduleFields({
  mode,
  onModeChange,
  atValue,
  onAtValueChange,
  everyMinutes,
  onEveryMinutesChange,
  nowMs = Date.now(),
}: AutomationScheduleFieldsProps) {
  return (
    <section className="altai-automation-schedule-fields border-b border-border-subtle px-3.5 py-3.5">
      <SurfaceSectionHeader
        title="Schedule"
        description="Choose when this instruction should return to its chat."
      />
      <div className="mt-3 flex items-center gap-2">
        <span className="text-[10px] text-muted-foreground">Schedule</span>
        <div
          role="group"
          aria-label="Schedule mode"
          className="inline-flex overflow-hidden rounded-md border border-border bg-card"
        >
          {(
            [
              { id: "at", label: "Once" },
              { id: "every", label: "Repeat" },
            ] as const
          ).map((option) => (
            <button
              key={option.id}
              type="button"
              aria-pressed={mode === option.id}
              onClick={() => onModeChange(option.id)}
              className={cn(
                "px-2 py-1 text-[10px] outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/25",
                mode === option.id
                  ? "bg-foreground/[0.085] text-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground",
              )}
            >
              {option.label}
            </button>
          ))}
        </div>
        {mode === "at" ? (
          <input
            type="datetime-local"
            value={atValue}
            onChange={(event) => onAtValueChange(event.target.value)}
            aria-label="Automation run time"
            className="min-w-0 flex-1 border border-border bg-card px-1.5 py-1 text-[10px] outline-none focus:border-ring"
          />
        ) : (
          <label className="flex min-w-0 flex-1 items-center gap-1 text-[10px] text-muted-foreground">
            Every
            <input
              type="number"
              min="1"
              value={everyMinutes}
              onChange={(event) => onEveryMinutesChange(event.target.value)}
              aria-label="Repeat interval in minutes"
              className="w-14 border border-border bg-card px-1.5 py-1 text-[10px] outline-none focus:border-ring"
            />
            min
          </label>
        )}
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-1">
        <span className="mr-1 text-[9px] font-medium uppercase tracking-wide text-muted-foreground">
          Quick set
        </span>
        <button
          type="button"
          onClick={() => {
            onModeChange("at");
            onAtValueChange(localDateTimeValue(nowMs + 15 * 60_000));
          }}
          className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          In 15 min
        </button>
        <button
          type="button"
          onClick={() => {
            onModeChange("at");
            const tomorrow = new Date(nowMs);
            tomorrow.setDate(tomorrow.getDate() + 1);
            tomorrow.setHours(9, 0, 0, 0);
            onAtValueChange(localDateTimeValue(tomorrow.getTime()));
          }}
          className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          Tomorrow 09:00
        </button>
        <button
          type="button"
          onClick={() => {
            onModeChange("every");
            onEveryMinutesChange("1440");
          }}
          className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          Daily
        </button>
        <button
          type="button"
          onClick={() => {
            onModeChange("every");
            onEveryMinutesChange("10080");
          }}
          className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          Weekly
        </button>
      </div>
    </section>
  );
}
