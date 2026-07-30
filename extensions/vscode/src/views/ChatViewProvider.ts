import * as vscode from "vscode";
import { randomBytes, randomUUID } from "node:crypto";
import type { HostManager } from "../host/HostManager.js";
import type { ManagedHost } from "../host/HostManager.js";
import type { ProtocolNotification } from "../protocol/RpcClient.js";
import type { WorkspaceRegistry } from "../workspace/WorkspaceRegistry.js";
import type { WorkspaceFolderRef } from "../workspace/WorkspaceRegistry.js";
import { canonicalizePath } from "../workspace/WorkspaceRegistry.js";
import { ContextCollector, ContextError, type ContextDiagnostic, type ContextItem, type ContextResource } from "../context/ContextCollector.js";
import { serializePromptWithContext, PromptContextError } from "../context/promptContext.js";
import { parseChatRunEvent, parseChatWebviewMessage, type ChatHostMessage } from "./chatMessages.js";
import { type RunIdentity, type RunProjectionStore } from "./runProjection.js";

type ActiveChat = { readonly host: ManagedHost; readonly chatId: string; runId?: string; awaitingRun: boolean };

export class ChatViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = "altai.chat";
  private view: vscode.WebviewView | undefined;
  private active: ActiveChat | undefined;
  private readonly hostSubscriptions = new Map<ManagedHost, () => void>();
  private contextItems: readonly ContextItem[] = [];
  private contextFolder: WorkspaceFolderRef | undefined;
  private readonly context = new ContextCollector({
    canonicalize: canonicalizePath,
    readFile: async (resource) => vscode.workspace.fs.readFile(vscode.Uri.parse(resource.uri)),
  });

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly registry: WorkspaceRegistry,
    private readonly hosts: HostManager,
    private readonly output: vscode.OutputChannel,
    private readonly runs: RunProjectionStore,
  ) {}

  resolveWebviewView(view: vscode.WebviewView): void {
    this.view = view;
    view.webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this.extensionUri, "dist")],
    };
    view.webview.html = renderHtml(view.webview, this.extensionUri);
    view.webview.onDidReceiveMessage((value: unknown) => void this.handleIntent(value));
    view.onDidDispose(() => { this.view = undefined; });
    this.post({ type: "chat/status", tone: "info", message: "Ready. Session history, replay, and steering are not available in this MVP." });
    this.postContext();
  }

  reveal(): void {
    this.view?.show(true);
  }

  /** Work and Inbox use this command to return to the same in-memory run. */
  revealRun(identity: RunIdentity): void {
    this.reveal();
    this.post({
      type: "chat/status",
      tone: "info",
      message: `Run ${identity.runId} selected. Run history is in-memory only; replay is not available after reload.`,
    });
  }

  private async handleIntent(value: unknown): Promise<void> {
    const intent = parseChatWebviewMessage(value);
    if (!intent) {
      this.output.appendLine("Ignored invalid webview message.");
      return;
    }
    if (intent.type === "chat/openLogs") {
      this.output.show(true);
      return;
    }
    if (intent.type === "chat/stop") {
      await this.stop(intent.runId);
      return;
    }
    if (intent.type === "chat/removeContext") {
      this.removeContext(intent.id);
      return;
    }
    await this.start(intent.prompt);
  }

  private async start(prompt: string): Promise<void> {
    if (this.active?.awaitingRun || this.active?.runId) {
      this.post({ type: "chat/status", tone: "info", message: "ALTAI is already working. Stop the current run before sending another message." });
      return;
    }
    try {
      const active = this.active ?? await this.createActiveChat(this.contextFolder);
      active.awaitingRun = true;
      this.post({ type: "chat/status", tone: "info", message: "Starting ALTAI…" });
      const promptWithContext = serializePromptWithContext(prompt, this.contextItems);
      await active.host.client.request("run/start", { chat_id: active.chatId, prompt: promptWithContext });
      // The actual run identity is supplied by a validated run_started event.
      // It is deliberately not fabricated from the request response.
      this.post({ type: "chat/status", tone: "info", message: "ALTAI is starting…" });
    } catch (error) {
      if (this.active) this.active.awaitingRun = false;
      const message = error instanceof PromptContextError ? contextErrorMessage(error) : userFacingError(error);
      this.output.appendLine(message);
      this.post({ type: "chat/status", tone: "error", message });
      void vscode.window.showWarningMessage(message, "Open ALTAI Logs").then((choice) => {
        if (choice) this.output.show(true);
      });
    }
  }

  private async stop(runId: string): Promise<void> {
    const active = this.active;
    if (!active || active.runId !== runId) {
      this.post({ type: "chat/status", tone: "error", message: "That run is no longer active." });
      return;
    }
    try {
      this.post({ type: "chat/status", tone: "info", message: "Stopping ALTAI…" });
      await active.host.client.request("run/cancel", { run_id: runId });
    } catch (error) {
      const message = "ALTAI could not stop this run. Check the ALTAI output for details.";
      this.output.appendLine(`${message} ${error instanceof Error ? error.message : "unknown error"}`);
      this.post({ type: "chat/status", tone: "error", message });
    }
  }

  /** Called by editor/explorer commands. Context never comes from the webview. */
  async addSelectionContext(instruction: string, includeDiagnostics = false): Promise<void> {
    try {
      this.assertContextAllowed();
      const editor = vscode.window.activeTextEditor;
      if (!editor) throw new ContextError("missing_file");
      const resource = toResource(editor.document.uri, editor.document.isUntitled);
      const folder = await this.registry.folderForResource(resource);
      const range = { startLine: editor.selection.start.line + 1, endLine: editor.selection.end.line + 1 };
      const items: ContextItem[] = [await this.context.selection(resource, toWorkspace(folder), editor.document.getText(editor.selection), range)];
      if (includeDiagnostics) {
        const diagnostics = await this.context.diagnostics(resource, toWorkspace(folder), toDiagnostics(vscode.languages.getDiagnostics(editor.document.uri)));
        if (diagnostics) items.push(diagnostics);
      }
      await this.addContext(folder, items);
      this.post({ type: "chat/draft", prompt: instruction });
    } catch (error) {
      this.reportContextError(error);
    }
  }

  async reviewFile(uri?: vscode.Uri): Promise<void> {
    try {
      this.assertContextAllowed();
      const target = uri ?? vscode.window.activeTextEditor?.document.uri;
      if (!target) throw new ContextError("missing_file");
      const resource = toResource(target, target.scheme === "untitled");
      const folder = await this.registry.folderForResource(resource);
      const file = await this.context.file(resource, toWorkspace(folder), `Review ${vscode.workspace.asRelativePath(target, false)}`);
      const diagnostics = await this.context.diagnostics(resource, toWorkspace(folder), toDiagnostics(vscode.languages.getDiagnostics(target)));
      await this.addContext(folder, diagnostics ? [file, diagnostics] : [file]);
      this.post({ type: "chat/draft", prompt: "Review this file for correctness, reliability, security, and maintainability." });
    } catch (error) {
      this.reportContextError(error);
    }
  }

  async addFileContext(uri?: vscode.Uri): Promise<void> {
    try {
      this.assertContextAllowed();
      const target = uri ?? vscode.window.activeTextEditor?.document.uri;
      if (!target) throw new ContextError("missing_file");
      const resource = toResource(target, target.scheme === "untitled");
      const folder = await this.registry.folderForResource(resource);
      const file = await this.context.file(resource, toWorkspace(folder), vscode.workspace.asRelativePath(target, false));
      await this.addContext(folder, [file]);
    } catch (error) {
      this.reportContextError(error);
    }
  }

  private async createActiveChat(preferred?: WorkspaceFolderRef): Promise<ActiveChat> {
    const folder = await this.registry.chooseFolder(preferred);
    const host = await this.hosts.getOrStart(folder);
    this.subscribeToHost(host);
    const active: ActiveChat = { host, chatId: randomUUID(), awaitingRun: false };
    this.active = active;
    this.post({ type: "chat/hostReady", workspace: host.workspace });
    return active;
  }

  private subscribeToHost(host: ManagedHost): void {
    if (this.hostSubscriptions.has(host)) return;
    const dispose = host.client.onNotification((message) => this.onHostNotification(host, message));
    this.hostSubscriptions.set(host, dispose);
  }

  private onHostNotification(host: ManagedHost, notification: ProtocolNotification): void {
    if (notification.method !== "run/event") return;
    const event = parseChatRunEvent(notification.params);
    if (event) this.runs.ingest(event);
    const active = this.active;
    if (!event || !active || active.host !== host || event.chatId !== active.chatId) return;
    if (event.type === "run_started") {
      active.runId = event.runId;
      active.awaitingRun = false;
    }
    if (event.type === "run_terminated") {
      active.runId = undefined;
      active.awaitingRun = false;
    }
    this.post({ type: "chat/run-event", event });
  }

  private post(message: ChatHostMessage): void {
    void this.view?.webview.postMessage(message);
  }

  private async addContext(folder: WorkspaceFolderRef, items: readonly ContextItem[]): Promise<void> {
    const identity = await this.registry.hostIdentity(folder);
    if ((this.active && this.active.host.workspace !== identity) || (this.contextFolder && (await this.registry.hostIdentity(this.contextFolder)) !== identity)) {
      throw new ContextError("different_workspace");
    }
    const retained = this.contextItems.filter((existing) => !items.some((item) => item.id === existing.id));
    const next = [...retained, ...items];
    this.context.assertCollection(next);
    this.contextItems = next;
    this.contextFolder = folder;
    this.postContext();
  }

  private removeContext(id: string): void {
    this.contextItems = this.contextItems.filter((item) => item.id !== id);
    if (this.contextItems.length === 0 && !this.active) this.contextFolder = undefined;
    this.postContext();
  }

  private postContext(): void {
    this.post({ type: "chat/context", items: this.contextItems.map(({ id, kind, label, uri, range }) => range === undefined ? { id, kind, label, uri } : { id, kind, label, uri, range }) });
  }

  private reportContextError(error: unknown): void {
    const message = contextErrorMessage(error);
    this.output.appendLine(message);
    this.post({ type: "chat/status", tone: "error", message });
    void vscode.window.showWarningMessage(message, "Open ALTAI Logs").then((choice) => {
      if (choice) this.output.show(true);
    });
  }

  private assertContextAllowed(): void {
    if (!vscode.workspace.isTrusted) throw new ContextError("untrusted_workspace");
  }
}

function renderHtml(webview: vscode.Webview, extensionUri: vscode.Uri): string {
  const nonce = createNonce();
  const script = webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, "dist", "chat.js"));
  const csp = `default-src 'none'; style-src ${webview.cspSource} 'nonce-${nonce}'; script-src ${webview.cspSource} 'nonce-${nonce}';`;
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${csp}"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>ALTAI Chat</title><style nonce="${nonce}">
body{font-family:var(--vscode-font-family);font-size:var(--vscode-font-size);padding:0;margin:0;color:var(--vscode-foreground);background:var(--vscode-sideBar-background)}
#app{display:flex;min-height:100vh;flex-direction:column}.header{padding:12px 12px 8px}.header h2{font-size:1em;margin:0}.hint{color:var(--vscode-descriptionForeground);font-size:.9em;margin:5px 0 0}.transcript{flex:1;padding:0 12px 12px;overflow:auto}.message{border-left:2px solid var(--vscode-editorWidget-border);margin:10px 0;padding:6px 8px}.message-user{border-left-color:var(--vscode-button-background)}.message-thinking,.message-tool,.message-terminal{color:var(--vscode-descriptionForeground);font-size:.92em}.message-label{font-weight:600;font-size:.82em;margin-bottom:3px}.message-content{white-space:pre-wrap;overflow-wrap:anywhere}.composer{border-top:1px solid var(--vscode-editorWidget-border);padding:10px 12px}.context{display:flex;flex-wrap:wrap;gap:5px;margin:7px 0}.chip{display:inline-flex;align-items:center;gap:5px;max-width:100%;padding:3px 5px;border:1px solid var(--vscode-editorWidget-border);border-radius:3px;font-size:.86em}.chip span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.chip button{padding:0 3px;background:transparent;border:0;color:inherit;cursor:pointer}.composer textarea{box-sizing:border-box;width:100%;resize:vertical;min-height:64px;background:var(--vscode-input-background);border:1px solid var(--vscode-input-border);color:var(--vscode-input-foreground);font:inherit;padding:7px}.actions{display:flex;gap:8px;margin-top:7px}.actions button{font:inherit;padding:5px 9px}.actions button.primary{background:var(--vscode-button-background);color:var(--vscode-button-foreground);border:0}.status{min-height:1.4em;margin:7px 0 0;color:var(--vscode-descriptionForeground)}.status[data-tone="error"]{color:var(--vscode-errorForeground)}</style></head><body><main id="app"><header class="header"><h2>ALTAI Chat</h2><p class="hint">One run at a time. Sessions, replay, and steering are not available yet.</p></header><section id="transcript" class="transcript" aria-label="ALTAI conversation" aria-live="polite"></section><section class="composer" aria-label="Message composer"><div id="context" class="context" aria-label="Attached context"></div><label for="composer">Message</label><textarea id="composer" rows="3" placeholder="Ask ALTAI about this workspace"></textarea><div class="actions"><button id="send" class="primary" type="button">Send</button><button id="stop" type="button" disabled>Stop</button><button id="logs" type="button">Open Logs</button></div><p id="status" class="status" role="status" aria-live="polite"></p></section></main><script nonce="${nonce}" src="${script}"></script></body></html>`;
}

function createNonce(): string {
  // CSP nonces are security tokens, not display identifiers. `base64url`
  // keeps the generated value safe for both the header and HTML attribute.
  return randomBytes(24).toString("base64url");
}

function userFacingError(error: unknown): string {
  const reason = error instanceof Error ? error.message : "unknown error";
  if (reason === "untrusted_workspace") return "ALTAI is disabled in Restricted Mode. Trust this workspace to start the agent.";
  if (reason === "virtual_workspace") return "ALTAI cannot run in this virtual workspace.";
  if (reason === "no_workspace") return "Open a folder before starting ALTAI.";
  return "ALTAI host could not start. Check the ALTAI output for recovery details.";
}

function toResource(uri: vscode.Uri, isUntitled: boolean): ContextResource {
  return { uri: uri.toString(), fsPath: uri.fsPath, scheme: uri.scheme, isUntitled };
}

function toWorkspace(folder: WorkspaceFolderRef): { name: string; fsPath: string; scheme: string } {
  return { name: folder.name, fsPath: folder.fsPath, scheme: folder.scheme };
}

function toDiagnostics(diagnostics: readonly vscode.Diagnostic[]): ContextDiagnostic[] {
  return diagnostics.map((diagnostic) => ({
    severity: vscode.DiagnosticSeverity[diagnostic.severity] ?? "Unknown",
    message: diagnostic.message,
    ...(diagnostic.code === undefined ? {} : { code: typeof diagnostic.code === "object" ? diagnostic.code.value : diagnostic.code }),
    range: { startLine: diagnostic.range.start.line + 1, endLine: diagnostic.range.end.line + 1 },
  }));
}

function contextErrorMessage(error: unknown): string {
  const reason = error instanceof ContextError || error instanceof PromptContextError ? error.reason : error instanceof Error ? error.message : "unknown";
  switch (reason) {
    case "empty_selection": return "Select code before using an ALTAI selection command.";
    case "untitled_file": return "Save this file before adding it to ALTAI context.";
    case "virtual_file": return "ALTAI context only supports files in a trusted filesystem workspace.";
    case "outside_workspace": return "ALTAI only accepts context inside the selected workspace folder.";
    case "binary_file": return "This file looks binary and cannot be added to ALTAI context.";
    case "file_too_large": return "This selection or file is too large to add to ALTAI context.";
    case "context_too_large": return "Attached context exceeds ALTAI's prompt limit. Remove a context item and try again.";
    case "too_many_items": return "Too many context items are attached. Remove one before adding another.";
    case "different_workspace": return "All context in an ALTAI chat must belong to the same workspace folder.";
    case "untrusted_workspace": return "Trust this workspace before adding files or code to ALTAI context.";
    case "missing_file": return "ALTAI could not read this file. Save it and try again.";
    default: return "ALTAI could not add that context. Check the ALTAI output for details.";
  }
}
