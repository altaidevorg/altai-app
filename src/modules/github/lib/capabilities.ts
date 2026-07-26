export type GitHubRepoState = "loading" | "ready" | "none";

export type GitHubCapabilities = {
  /** Local Git, todos, assignments, and the Overview board never need GitHub. */
  localWorkspace: true;
  remoteItems: boolean;
  linkedProjects: boolean;
  remoteMutations: boolean;
};

/**
 * GitHub authentication unlocks remote capabilities; it is never a gate for
 * local project management or local Git.
 */
export function githubCapabilities(input: {
  connected: boolean;
  repoState: GitHubRepoState;
}): GitHubCapabilities {
  const remoteAvailable = input.connected && input.repoState === "ready";
  return {
    localWorkspace: true,
    remoteItems: remoteAvailable,
    linkedProjects: remoteAvailable,
    remoteMutations: remoteAvailable,
  };
}
