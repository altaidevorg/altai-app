/**
 * Pure slash-command index filter/resolve helpers (A6.155).
 * Hosts own the command registry; package only filters/matches.
 */

export type SlashCommandSearchFields = {
  name: string;
  label: string;
  description: string;
  aliases?: readonly string[];
  category?: string;
  source?: string;
};

/** Filter an index by free-text query (name, label, description, aliases, category, source). */
export function filterSlashCommands<T extends SlashCommandSearchFields>(
  index: readonly T[],
  query = "",
): readonly T[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return index;
  return index.filter((command) =>
    [
      command.name,
      command.label,
      command.description,
      ...(command.aliases ?? []),
      command.category ?? "",
      command.source ?? "",
    ].some((value) => value.toLowerCase().includes(normalized)),
  );
}

/** Resolve a command by exact name or alias (case-insensitive). */
export function resolveSlashCommandInIndex<T extends SlashCommandSearchFields>(
  index: readonly T[],
  name: string,
): T | undefined {
  const normalized = name.trim().toLowerCase();
  return index.find(
    (command) =>
      command.name === normalized || command.aliases?.includes(normalized),
  );
}
