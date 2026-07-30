import { EventEmitter } from "node:events";
import { describe, expect, it } from "vitest";
import { encodeFrame, FrameDecoder } from "@altai/agent-protocol";
import { HostManager, HostStartError } from "../host/HostManager.js";
import { HostResolver } from "../host/HostResolver.js";
import type { ProcessLike } from "../protocol/RpcClient.js";
import type { WorkspaceFolderRef } from "../workspace/WorkspaceRegistry.js";

const folderA: WorkspaceFolderRef = { name: "alpha", fsPath: "/real/alpha", scheme: "file", uri: "file:///real/alpha" };
const folderB: WorkspaceFolderRef = { name: "beta", fsPath: "/real/beta", scheme: "file", uri: "file:///real/beta" };

describe("HostManager", () => {
  it("does not spawn a host when constructed (activation stays lazy)", () => {
    let spawned = 0;
    const manager = createManager(() => { spawned += 1; return new FakeProcess(); });
    expect(manager.activeWorkspaceCount()).toBe(0);
    expect(spawned).toBe(0);
  });

  it("blocks host startup in untrusted and virtual workspaces", async () => {
    let spawned = 0;
    const untrusted = createManager(() => { spawned += 1; return new FakeProcess(); }, { trusted: false });
    await expect(untrusted.getOrStart(folderA)).rejects.toMatchObject({ reason: "untrusted_workspace" } satisfies Partial<HostStartError>);

    const virtual = createManager(() => { spawned += 1; return new FakeProcess(); }, { virtual: true });
    await expect(virtual.getOrStart({ ...folderA, scheme: "memfs" })).rejects.toMatchObject({ reason: "virtual_workspace" } satisfies Partial<HostStartError>);
    expect(spawned).toBe(0);
  });

  it("uses one isolated host per canonical multi-root folder", async () => {
    const calls: string[][] = [];
    const manager = createManager((executable, args) => {
      calls.push([executable, ...args]);
      return new FakeProcess();
    });
    const first = await manager.getOrStart(folderA);
    const same = await manager.getOrStart({ ...folderA, fsPath: "/symlink/alpha" });
    const second = await manager.getOrStart(folderB);

    expect(first).toBe(same);
    expect(first.workspace).toBe("/canonical/alpha");
    expect(second.workspace).toBe("/canonical/beta");
    expect(calls).toEqual([
      ["altai-cli", "serve", "--stdio", "--protocol", "1", "--workspace", "/canonical/alpha"],
      ["altai-cli", "serve", "--stdio", "--protocol", "1", "--workspace", "/canonical/beta"],
    ]);
    await manager.shutdownAll();
  });

  it("shares the pending host lease across concurrent starts", async () => {
    let spawned = 0;
    const manager = createManager(() => {
      spawned += 1;
      return new FakeProcess();
    });

    const [first, second] = await Promise.all([
      manager.getOrStart(folderA),
      manager.getOrStart({ ...folderA, fsPath: "/symlink/alpha" }),
    ]);

    expect(first).toBe(second);
    expect(spawned).toBe(1);
    await manager.shutdownAll();
  });

  it("never accepts a workspace-scoped executable override", () => {
    const resolver = new HostResolver({
      extensionPath: "/extension",
      platform: "linux",
      exists: () => false,
      configuration: () => ({ globalValue: "/user/altai-cli", workspaceValue: "/workspace/evil", workspaceFolderValue: "/workspace/also-evil" }),
    });
    expect(resolver.resolve()).toEqual({ executable: "/user/altai-cli", source: "global-override", workspaceOverrideIgnored: true });
  });
});

function createManager(spawner: (executable: string, args: readonly string[]) => ProcessLike, state: { trusted?: boolean; virtual?: boolean } = {}): HostManager {
  return new HostManager({
    resolver: new HostResolver({ extensionPath: "/extension", platform: "linux", exists: () => false, configuration: () => ({}) }),
    canonicalize: async (folder) => folder.name === "alpha" ? "/canonical/alpha" : "/canonical/beta",
    gate: { isTrusted: () => state.trusted ?? true, isVirtual: () => state.virtual ?? false },
    log: () => undefined,
    spawn: spawner,
  });
}

class FakeReadable extends EventEmitter {}

class FakeProcess extends EventEmitter implements ProcessLike {
  readonly stdout = new FakeReadable();
  readonly stderr = new FakeReadable();
  private readonly decoder = new FrameDecoder();
  readonly stdin = {
    write: (data: Uint8Array, callback?: (error?: Error | null) => void): boolean => {
      for (const body of this.decoder.push(data)) {
        const request = JSON.parse(new TextDecoder().decode(body)) as { id: string; method: string };
        const response = new TextEncoder().encode(JSON.stringify({ jsonrpc: "2.0", id: request.id, result: request.method === "initialize" ? { protocol: 1 } : {} }));
        queueMicrotask(() => this.stdout.emit("data", encodeFrame(response)));
      }
      callback?.(null);
      return true;
    },
    end: (): void => undefined,
  };

  kill(): boolean { return true; }
}
