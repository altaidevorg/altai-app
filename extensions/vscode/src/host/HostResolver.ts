import { existsSync } from "node:fs";
import { join } from "node:path";

export type HostExecutableSetting = {
  readonly globalValue?: string;
  readonly workspaceValue?: string;
  readonly workspaceFolderValue?: string;
};

export type HostResolverOptions = {
  readonly extensionPath: string;
  readonly platform: NodeJS.Platform;
  readonly configuration: () => HostExecutableSetting;
  readonly exists?: (path: string) => boolean;
};

export type ResolvedHost = {
  readonly executable: string;
  readonly source: "bundled" | "global-override" | "path";
  readonly workspaceOverrideIgnored: boolean;
};

/** Resolves only package or user/global host binaries; never workspace config. */
export class HostResolver {
  private readonly exists: (path: string) => boolean;

  constructor(private readonly options: HostResolverOptions) {
    this.exists = options.exists ?? existsSync;
  }

  resolve(): ResolvedHost {
    const suffix = this.options.platform === "win32" ? ".exe" : "";
    const bundled = join(this.options.extensionPath, "bin", `${this.options.platform}-${process.arch}`, `altai-cli${suffix}`);
    const setting = this.options.configuration();
    const workspaceOverrideIgnored = Boolean(setting.workspaceValue?.trim() || setting.workspaceFolderValue?.trim());

    if (this.exists(bundled)) {
      return { executable: bundled, source: "bundled", workspaceOverrideIgnored };
    }
    if (setting.globalValue?.trim()) {
      return { executable: setting.globalValue.trim(), source: "global-override", workspaceOverrideIgnored };
    }
    return { executable: `altai-cli${suffix}`, source: "path", workspaceOverrideIgnored };
  }
}
