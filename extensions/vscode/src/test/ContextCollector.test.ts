import { describe, expect, it } from "vitest";
import { ContextCollector, ContextError, type ContextResource, type ContextWorkspace } from "../context/ContextCollector.js";
import { MAX_CONTEXT_ITEM_BYTES } from "../context/limits.js";

const workspace: ContextWorkspace = { name: "app", fsPath: "/canonical/app", scheme: "file" };
const resource: ContextResource = { uri: "file:///canonical/app/src/main.ts", fsPath: "/canonical/app/src/main.ts", scheme: "file" };

function collector(bytes = new TextEncoder().encode("export const ready = true;")): ContextCollector {
  return new ContextCollector({
    canonicalize: async (path) => path.replace("/linked", "/canonical"),
    readFile: async () => bytes,
  });
}

describe("ContextCollector", () => {
  it("collects only a bounded selection rooted in the chosen canonical workspace", async () => {
    const item = await collector().selection({ ...resource, fsPath: "/linked/app/src/main.ts" }, workspace, "const answer = 42;", { startLine: 2, endLine: 2 });
    expect(item).toMatchObject({ kind: "selection", range: { startLine: 2, endLine: 2 } });
    await expect(collector().selection({ ...resource, fsPath: "/canonical/other/nope.ts" }, workspace, "x", { startLine: 1, endLine: 1 }))
      .rejects.toEqual(expect.objectContaining({ reason: "outside_workspace" } satisfies Partial<ContextError>));
  });

  it("accepts a contained filename beginning with two dots", async () => {
    const item = await collector().file(
      { ...resource, fsPath: "/canonical/app/..notes/plan.md" },
      workspace,
    );
    expect(item.uri).toBe(resource.uri);
  });

  it("rejects virtual, binary, oversized, and excessive context", async () => {
    await expect(collector().file({ ...resource, scheme: "memfs" }, workspace)).rejects.toEqual(expect.objectContaining({ reason: "virtual_file" } satisfies Partial<ContextError>));
    await expect(collector(new Uint8Array([1, 0, 2])).file(resource, workspace)).rejects.toEqual(expect.objectContaining({ reason: "binary_file" } satisfies Partial<ContextError>));
    await expect(collector(new Uint8Array(MAX_CONTEXT_ITEM_BYTES + 1)).file(resource, workspace)).rejects.toEqual(expect.objectContaining({ reason: "file_too_large" } satisfies Partial<ContextError>));
    const item = await collector().selection(resource, workspace, "x", { startLine: 1, endLine: 1 });
    try {
      collector().assertCollection(Array.from({ length: 13 }, () => item));
      throw new Error("expected collection to be rejected");
    } catch (error) {
      expect(error).toEqual(expect.objectContaining({ reason: "too_many_items" } satisfies Partial<ContextError>));
    }
  });
});
