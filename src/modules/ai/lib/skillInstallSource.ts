/**
 * Parse skill install source for HostPorts.installSkill.
 *
 * Accepts:
 * - `owner/repo`
 * - GitHub URL
 * - `owner/repo#skillName` (optional single-skill filter)
 * - `owner/repo skillName` (whitespace-separated fallback)
 */

export type ParsedSkillInstallSource = {
  repo: string;
  skill?: string;
};

export function parseSkillInstallSource(source: string): ParsedSkillInstallSource {
  const trimmed = source.trim();
  if (!trimmed) {
    return { repo: "" };
  }
  const hash = trimmed.indexOf("#");
  if (hash >= 0) {
    const repo = trimmed.slice(0, hash).trim();
    const skill = trimmed.slice(hash + 1).trim();
    return skill ? { repo, skill } : { repo };
  }
  const parts = trimmed.split(/\s+/);
  if (parts.length >= 2 && !parts[0]!.includes("://") && parts[0]!.includes("/")) {
    return { repo: parts[0]!, skill: parts.slice(1).join(" ") };
  }
  return { repo: trimmed };
}
