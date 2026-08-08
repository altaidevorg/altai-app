/**
 * Pure helpers for skills status chrome (list-only; no install UI) (A6.79).
 */

export type SkillView = {
  name: string;
  description?: string;
  enabled?: boolean;
};

export function canMountSkillsStatus(flags: { skillsList: boolean }): boolean {
  return flags.skillsList;
}

export function sortSkillsForDisplay(skills: readonly SkillView[]): SkillView[] {
  return [...skills].sort((a, b) => a.name.localeCompare(b.name));
}

export function skillsSummaryCopy(skills: readonly SkillView[]): string {
  if (skills.length === 0) {
    return "No skills";
  }
  const enabled = skills.filter((s) => s.enabled !== false).length;
  if (enabled === skills.length) {
    return `${skills.length} skill${skills.length === 1 ? "" : "s"}`;
  }
  return `${enabled}/${skills.length} skills enabled`;
}
