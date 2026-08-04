import { Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type TaskSkillOption = {
  name: string;
  description?: string | null;
};

export type TaskSkillChipsProps = {
  skills: TaskSkillOption[];
  selected: string[];
  onToggle: (skillName: string) => void;
};

/**
 * Optional skills multi-select for create-task. Host owns installed skill
 * discovery and selected-name state.
 */
export function TaskSkillChips({
  skills,
  selected,
  onToggle,
}: TaskSkillChipsProps) {
  if (skills.length === 0) return null;

  const selectedSet = new Set(selected);

  return (
    <section className="altai-task-skill-chips border-t border-border-subtle px-3.5 py-3.5">
      <SurfaceSectionHeader
        title="Skills"
        description="Optional playbooks the agent should follow for this run."
        count={selected.length}
      />
      <div className="mt-3 flex flex-wrap gap-1.5">
        {skills.map((skill) => {
          const isSelected = selectedSet.has(skill.name);
          return (
            <button
              key={skill.name}
              type="button"
              title={skill.description ?? skill.name}
              aria-pressed={isSelected}
              onClick={() => onToggle(skill.name)}
              className={cn(
                "inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-[9.5px] font-medium transition-colors",
                isSelected
                  ? "border-foreground/15 bg-accent text-foreground"
                  : "border-border bg-card text-muted-foreground hover:bg-accent hover:text-foreground",
              )}
            >
              {isSelected ? (
                <HugeiconsIcon icon={Tick02Icon} size={10} strokeWidth={2} />
              ) : null}
              {skill.name}
            </button>
          );
        })}
      </div>
    </section>
  );
}
