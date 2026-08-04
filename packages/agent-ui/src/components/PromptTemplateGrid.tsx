import { cn } from "../lib/cn.js";

export type PromptTemplate = {
  label: string;
  value: string;
};

export type PromptTemplateGridProps = {
  templates: PromptTemplate[];
  onSelect: (value: string) => void;
  /** Column count for the chip grid. Default 2. */
  columns?: 2 | 3;
  /** Compact chips for denser automation create forms. */
  density?: "default" | "compact";
  className?: string;
};

/**
 * Quick-fill template chip grid for create-task / create-automation prompts.
 * Host owns template copy and the resulting text state.
 */
export function PromptTemplateGrid({
  templates,
  onSelect,
  columns = 2,
  density = "default",
  className,
}: PromptTemplateGridProps) {
  if (templates.length === 0) return null;

  return (
    <div
      className={cn(
        "altai-prompt-template-grid mt-2 gap-1.5",
        columns === 3 ? "grid grid-cols-3" : "grid grid-cols-2",
        className,
      )}
    >
      {templates.map((template) => (
        <button
          key={template.label}
          type="button"
          onClick={() => onSelect(template.value)}
          className={cn(
            "rounded-md border border-border bg-card text-left font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground",
            density === "compact"
              ? "px-2 py-1.5 text-[9px]"
              : "min-h-8 px-2 py-1.5 text-[9.5px]",
          )}
        >
          {template.label}
        </button>
      ))}
    </div>
  );
}
