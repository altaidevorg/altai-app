import * as vscode from "vscode";
import type { HostManager } from "../host/HostManager.js";
import type { WorkspaceRegistry } from "../workspace/WorkspaceRegistry.js";
import type { ChatViewProvider } from "../views/ChatViewProvider.js";
import type { RunIdentity } from "../views/runProjection.js";

export function registerCommands(
  subscriptions: vscode.Disposable[],
  chat: ChatViewProvider,
  registry: WorkspaceRegistry,
  hosts: HostManager,
  output: vscode.OutputChannel,
): void {
  subscriptions.push(
    vscode.commands.registerCommand("altai.openChat", async () => {
      await vscode.commands.executeCommand("workbench.view.extension.altai");
      chat.reveal();
    }),
    vscode.commands.registerCommand("altai.revealRun", async (identity: RunIdentity) => {
      await vscode.commands.executeCommand("workbench.view.extension.altai");
      chat.revealRun(identity);
    }),
    vscode.commands.registerCommand("altai.newChat", async () => {
      await vscode.commands.executeCommand("workbench.view.extension.altai");
      chat.reveal();
    }),
    vscode.commands.registerCommand("altai.openLogs", () => output.show(true)),
    vscode.commands.registerCommand("altai.askAboutSelection", async () => openContextChat(chat, () => chat.addSelectionContext("Answer this question about the selected code."))),
    vscode.commands.registerCommand("altai.explainSelection", async () => openContextChat(chat, () => chat.addSelectionContext("Explain the selected code."))),
    vscode.commands.registerCommand("altai.fixSelection", async () => openContextChat(chat, () => chat.addSelectionContext("Identify and fix the selected code. Explain the proposed change before applying it.", true))),
    vscode.commands.registerCommand("altai.refactorSelection", async () => openContextChat(chat, () => chat.addSelectionContext("Suggest a focused refactor of the selected code and explain the trade-offs."))),
    vscode.commands.registerCommand("altai.reviewFile", async (uri?: vscode.Uri) => openContextChat(chat, () => chat.reviewFile(uri))),
    vscode.commands.registerCommand("altai.addFileToContext", async (uri?: vscode.Uri) => openContextChat(chat, () => chat.addFileContext(uri))),
    vscode.commands.registerCommand("altai.runDoctor", async () => {
      try {
        const folder = await registry.chooseFolder();
        await hosts.getOrStart(folder);
        void vscode.window.showInformationMessage("ALTAI host is available. Full doctor diagnostics will arrive with the Chat MVP.");
      } catch {
        void vscode.window.showWarningMessage("ALTAI doctor could not start the host. Open ALTAI Logs for details.", "Open ALTAI Logs").then((choice) => {
          if (choice) output.show(true);
        });
      }
    }),
  );
}

async function openContextChat(chat: ChatViewProvider, action: () => Promise<void>): Promise<void> {
  await vscode.commands.executeCommand("workbench.view.extension.altai");
  chat.reveal();
  await action();
}
