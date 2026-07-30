import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import type { WorkspaceFolderRef } from "../workspace/WorkspaceRegistry.js";
import { HostResolver, type ResolvedHost } from "./HostResolver.js";
import { RpcClient, type ProcessLike } from "../protocol/RpcClient.js";

export type HostGate = {
  isTrusted(): boolean;
  isVirtual(folder: WorkspaceFolderRef): boolean;
};

export type HostSpawner = (executable: string, args: readonly string[]) => ProcessLike;

export type HostManagerOptions = {
  readonly resolver: HostResolver;
  readonly canonicalize: (folder: WorkspaceFolderRef) => Promise<string>;
  readonly gate: HostGate;
  readonly log: (line: string) => void;
  readonly spawn?: HostSpawner;
};

export type ManagedHost = {
  readonly workspace: string;
  readonly client: RpcClient;
  readonly resolved: ResolvedHost;
};

export class HostStartError extends Error {
  constructor(public readonly reason: "untrusted_workspace" | "virtual_workspace" | "initialize_failed", cause?: unknown) {
    super(reason, cause === undefined ? undefined : { cause });
  }
}

/** Supervises exactly one lazy child process per canonical workspace identity. */
export class HostManager {
  private readonly hosts = new Map<string, Promise<ManagedHost>>();
  private readonly spawn: HostSpawner;

  constructor(private readonly options: HostManagerOptions) {
    this.spawn = options.spawn ?? defaultSpawn;
  }

  async getOrStart(folder: WorkspaceFolderRef): Promise<ManagedHost> {
    if (!this.options.gate.isTrusted()) throw new HostStartError("untrusted_workspace");
    if (this.options.gate.isVirtual(folder)) throw new HostStartError("virtual_workspace");

    const workspace = await this.options.canonicalize(folder);
    return this.getOrStartWorkspace(workspace);
  }

  /**
   * Keep lookup and promise registration in the same synchronous turn. This
   * makes the pending promise the lease for a workspace while initialization is
   * in flight, so concurrent callers cannot spawn a second child process.
   */
  private getOrStartWorkspace(workspace: string): Promise<ManagedHost> {
    const existing = this.hosts.get(workspace);
    if (existing) return existing;

    const starting = this.start(workspace);
    this.hosts.set(workspace, starting);
    void starting.catch(() => {
      // Do not remove a newer lease if a later retry has already replaced it.
      if (this.hosts.get(workspace) === starting) this.hosts.delete(workspace);
    });
    return starting;
  }

  async shutdownAll(timeoutMs = 1_500): Promise<void> {
    const entries = [...this.hosts.values()];
    this.hosts.clear();
    await Promise.allSettled(entries.map(async (host) => (await host).client.shutdown(timeoutMs)));
  }

  activeWorkspaceCount(): number {
    return this.hosts.size;
  }

  private async start(workspace: string): Promise<ManagedHost> {
    const resolved = this.options.resolver.resolve();
    if (resolved.workspaceOverrideIgnored) {
      this.options.log("Ignored workspace-scoped altai.host.executable setting for security.");
    }
    this.options.log(`Starting ALTAI host (${resolved.source}) for ${workspace}`);
    let client: RpcClient | undefined;
    try {
      client = new RpcClient(
        this.spawn(resolved.executable, ["serve", "--stdio", "--protocol", "1", "--workspace", workspace]),
        this.options.log,
      );
      await client.request("initialize", { protocol_min: 1, protocol_max: 1 });
      this.options.log(`ALTAI host ready for ${workspace}`);
      return { workspace, client, resolved };
    } catch (error) {
      this.options.log(`ALTAI host failed to start: ${error instanceof Error ? error.message : "unknown error"}`);
      client?.close();
      throw new HostStartError("initialize_failed", error);
    }
  }
}

function defaultSpawn(executable: string, args: readonly string[]): ChildProcessWithoutNullStreams {
  return spawn(executable, [...args], { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
}
