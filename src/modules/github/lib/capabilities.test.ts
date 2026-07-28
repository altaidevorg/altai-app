import { describe, expect, it } from "vitest";
import { githubCapabilities } from "./capabilities";

describe("githubCapabilities", () => {
  it("keeps the local workspace available without a GitHub connection", () => {
    expect(
      githubCapabilities({ connected: false, repoState: "ready" }),
    ).toEqual({
      localWorkspace: true,
      remoteItems: false,
      remoteMutations: false,
    });
  });

  it("keeps local features available when the repository has no GitHub origin", () => {
    expect(
      githubCapabilities({ connected: true, repoState: "none" }),
    ).toEqual({
      localWorkspace: true,
      remoteItems: false,
      remoteMutations: false,
    });
  });

  it("enables remote capabilities only when connected to a GitHub repository", () => {
    expect(
      githubCapabilities({ connected: true, repoState: "ready" }),
    ).toEqual({
      localWorkspace: true,
      remoteItems: true,
      remoteMutations: true,
    });
  });
});
