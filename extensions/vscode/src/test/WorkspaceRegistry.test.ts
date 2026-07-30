import { describe, expect, it } from "vitest";
import { WorkspaceRegistry, WorkspaceSelectionError, type WorkspaceFolderRef } from "../workspace/WorkspaceRegistry.js";

const first: WorkspaceFolderRef = { name: "one", fsPath: "/one", scheme: "file", uri: "file:///one" };
const second: WorkspaceFolderRef = { name: "two", fsPath: "/two", scheme: "file", uri: "file:///two" };

describe("WorkspaceRegistry", () => {
  it("requires an explicit folder selection in a multi-root workspace", async () => {
    const registry = new WorkspaceRegistry(
      () => [first, second],
      { pick: async (items) => items[1] },
      async (path) => `/canonical${path}`,
    );
    const folder = await registry.chooseFolder();
    expect(folder).toBe(second);
    expect(await registry.hostIdentity(folder)).toBe("/canonical/two");
  });

  it("rejects virtual filesystem folders before they get a host identity", async () => {
    const registry = new WorkspaceRegistry(() => [], { pick: async () => undefined });
    await expect(registry.chooseFolder({ ...first, scheme: "memfs" })).rejects.toEqual(expect.objectContaining({ reason: "virtual_workspace" } satisfies Partial<WorkspaceSelectionError>));
  });

  it("binds an explicit editor resource to its owning multi-root folder", async () => {
    const registry = new WorkspaceRegistry(
      () => [first, second],
      { pick: async () => undefined },
      async (path) => `/canonical${path}`,
    );
    await expect(registry.folderForResource({ fsPath: "/two/src/index.ts", scheme: "file" })).resolves.toBe(second);
    await expect(registry.folderForResource({ fsPath: "/elsewhere/a.ts", scheme: "file" })).rejects.toEqual(expect.objectContaining({ reason: "outside_workspace" }));
  });
});
