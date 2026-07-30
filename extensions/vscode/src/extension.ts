import * as vscode from "vscode";
import { registerCommands } from "./commands/registerCommands.js";
import { HostManager } from "./host/HostManager.js";
import { HostResolver } from "./host/HostResolver.js";
import { ChatViewProvider } from "./views/ChatViewProvider.js";
import { InboxTreeProvider, WorkTreeProvider } from "./views/WorkInboxTreeProvider.js";
import { RunProjectionStore } from "./views/runProjection.js";
import { WorkspaceRegistry, type WorkspaceFolderRef } from "./workspace/WorkspaceRegistry.js";

let activeHosts: HostManager | undefined;

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("ALTAI");
  const registry = new WorkspaceRegistry(
    () => (vscode.workspace.workspaceFolders ?? []).map(toFolderRef),
    { pick: async (items, options) => vscode.window.showQuickPick(items, options) },
  );
  const resolver = new HostResolver({
    extensionPath: context.extensionPath,
    platform: process.platform,
    configuration: () => {
      const inspected = vscode.workspace.getConfiguration("altai.host").inspect<string>("executable");
      return {
        globalValue: inspected?.globalValue,
        workspaceValue: inspected?.workspaceValue,
        workspaceFolderValue: inspected?.workspaceFolderValue,
      };
    },
  });
  const hosts = new HostManager({
    resolver,
    canonicalize: (folder) => registry.hostIdentity(folder),
    gate: {
      isTrusted: () => vscode.workspace.isTrusted,
      isVirtual: (folder) => folder.scheme !== "file" && folder.scheme !== "vscode-remote",
    },
    log: (line) => output.appendLine(line),
  });
  activeHosts = hosts;
  const runs = new RunProjectionStore();
  const chat = new ChatViewProvider(context.extensionUri, registry, hosts, output, runs);
  const work = new WorkTreeProvider(runs);
  const inbox = new InboxTreeProvider(runs);

  context.subscriptions.push(
    output,
    vscode.window.registerWebviewViewProvider(ChatViewProvider.viewType, chat),
    work,
    inbox,
    vscode.window.registerTreeDataProvider("altai.work", work),
    vscode.window.registerTreeDataProvider("altai.inbox", inbox),
  );
  registerCommands(context.subscriptions, chat, registry, hosts, output);
}

export function deactivate(): Thenable<void> {
  const hosts = activeHosts;
  activeHosts = undefined;
  return hosts?.shutdownAll() ?? Promise.resolve();
}

function toFolderRef(folder: vscode.WorkspaceFolder): WorkspaceFolderRef {
  return { name: folder.name, fsPath: folder.uri.fsPath, scheme: folder.uri.scheme, uri: folder.uri.toString() };
}
