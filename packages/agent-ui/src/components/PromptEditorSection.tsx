import { cn } from "../lib/cn.js";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";
import {
  PromptTemplateGrid,
  type PromptTemplate,
} from "./PromptTemplateGrid.js";

export type PromptEditorSectionProps = {
  title: string;
  description: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  templates: PromptTemplate[];
  textareaId?: string;
  ariaLabel?: string;
  maxLength?: number;
  rows?: number;
  templateColumns?: 2 | 3;
  templateDensity?: "default" | "compact";
  /** Larger task-create textarea vs compact automation instruction. */
  size?: "task" | "automation";
};

/**
 * Create-form instruction section: header, textarea, and template chips.
 * Host owns prompt text state and template copy.
 */
export function PromptEditorSection({
  title,
  description,
  value,
  onChange,
  placeholder,
  templates,
  textareaId,
  ariaLabel,
  maxLength,
  rows,
  templateColumns = 2,
  templateDensity = "default",
  size = "task",
}: PromptEditorSectionProps) {
  return (
    <section className="altai-prompt-editor-section border-b border-border-subtle px-3.5 py-3.5">
      <SurfaceSectionHeader title={title} description={description} />
      <textarea
        id={textareaId}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        maxLength={maxLength}
        rows={rows}
        aria-label={ariaLabel}
        placeholder={placeholder}
        className={cn(
          "mt-3 w-full resize-y rounded-lg border border-border bg-muted/55 px-3 py-2.5 leading-relaxed outline-none focus:border-ring",
          size === "task"
            ? "min-h-28 text-[11px] text-foreground placeholder:text-muted-foreground/65 focus:ring-2 focus:ring-ring/20"
            : "text-[10.5px] placeholder:text-muted-foreground/70",
        )}
      />
      <PromptTemplateGrid
        templates={templates}
        onSelect={onChange}
        columns={templateColumns}
        density={templateDensity}
      />
    </section>
  );
}
