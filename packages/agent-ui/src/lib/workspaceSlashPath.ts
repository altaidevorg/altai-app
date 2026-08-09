/**
 * Pure workspace slash-command path / name validators (A6.156).
 */

/** Relative path of a workspace workflow command under `.altai/commands/`. */
export const WORKSPACE_SLASH_COMMAND_PATH = /^\.altai\/commands\/([^/]+)\.md$/i;

/** Allowed slash command name / alias: starts with alnum, then alnum or hyphen. */
export const SLASH_COMMAND_NAME = /^[a-z0-9][a-z0-9-]*$/;

export function isWorkspaceSlashCommandPath(path: string): boolean {
  return WORKSPACE_SLASH_COMMAND_PATH.test(path);
}

/** Basename stem for `.altai/commands/<name>.md` (lowercased), or null. */
export function workspaceSlashCommandStem(path: string): string | null {
  const match = path.match(WORKSPACE_SLASH_COMMAND_PATH);
  return match?.[1] ? match[1].toLowerCase() : null;
}

export function isValidSlashCommandName(name: string): boolean {
  return SLASH_COMMAND_NAME.test(name);
}
