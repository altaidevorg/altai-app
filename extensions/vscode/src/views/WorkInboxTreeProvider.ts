import * as vscode from "vscode";
import { chatRunDeepLink, type ProjectedRun, type RunProjection, type RunProjectionStore } from "./runProjection.js";

type WorkGroup = "active" | "history" | "attention";
type WorkElement = WorkGroupItem | WorkRunItem;

const IN_MEMORY_LIMITATION = "In-memory only; reload and restart do not restore runs until host replay is available.";

export class WorkTreeProvider implements vscode.TreeDataProvider<WorkElement>, vscode.Disposable {
  private readonly changed = new vscode.EventEmitter<WorkElement | undefined>();
  private projection: RunProjection;
  private readonly unsubscribe: () => void;
  readonly onDidChangeTreeData = this.changed.event;

  constructor(store: RunProjectionStore) {
    this.projection = store.snapshot();
    this.unsubscribe = store.subscribe((projection) => {
      this.projection = projection;
      this.changed.fire(undefined);
    });
  }

  dispose(): void {
    this.unsubscribe();
    this.changed.dispose();
  }

  getTreeItem(element: WorkElement): vscode.TreeItem {
    return element;
  }

  getChildren(element?: WorkElement): WorkElement[] {
    if (!element) return [group("active", this.projection.active.length), group("attention", this.projection.attention.length), group("history", this.projection.history.length)];
    if (element instanceof WorkGroupItem) return runsForGroup(element.group, this.projection).map((run) => new WorkRunItem(run));
    return [];
  }
}

export class InboxTreeProvider implements vscode.TreeDataProvider<InboxRunItem>, vscode.Disposable {
  private readonly changed = new vscode.EventEmitter<InboxRunItem | undefined>();
  private projection: RunProjection;
  private readonly unsubscribe: () => void;
  readonly onDidChangeTreeData = this.changed.event;

  constructor(store: RunProjectionStore) {
    this.projection = store.snapshot();
    this.unsubscribe = store.subscribe((projection) => {
      this.projection = projection;
      this.changed.fire(undefined);
    });
  }

  dispose(): void {
    this.unsubscribe();
    this.changed.dispose();
  }

  getTreeItem(element: InboxRunItem): vscode.TreeItem {
    return element;
  }

  getChildren(): InboxRunItem[] {
    return this.projection.attention.map((run) => new InboxRunItem(run));
  }
}

class WorkGroupItem extends vscode.TreeItem {
  constructor(readonly group: WorkGroup, count: number) {
    super(group === "active" ? "Active" : group === "attention" ? "Needs attention" : "History", vscode.TreeItemCollapsibleState.Expanded);
    this.description = count === 0 ? "None" : String(count);
    this.contextValue = `altai.work.${group}`;
    this.tooltip = IN_MEMORY_LIMITATION;
  }
}

class WorkRunItem extends vscode.TreeItem {
  constructor(run: ProjectedRun) {
    super(run.title, vscode.TreeItemCollapsibleState.None);
    this.description = run.detail;
    this.tooltip = `${run.chatId} / ${run.runId}\n${IN_MEMORY_LIMITATION}`;
    this.contextValue = `altai.run.${run.phase}`;
    this.command = chatRunDeepLink(run);
  }
}

class InboxRunItem extends vscode.TreeItem {
  constructor(run: ProjectedRun) {
    super(run.title, vscode.TreeItemCollapsibleState.None);
    this.description = run.detail;
    this.tooltip = `${run.chatId} / ${run.runId}\nVisibility only: approval and steering resolution are not supported by the current host protocol.\n${IN_MEMORY_LIMITATION}`;
    this.contextValue = `altai.inbox.${run.attention}`;
    this.command = chatRunDeepLink(run);
  }
}

function group(kind: WorkGroup, count: number): WorkGroupItem {
  return new WorkGroupItem(kind, count);
}

function runsForGroup(group: WorkGroup, projection: RunProjection): readonly ProjectedRun[] {
  if (group === "active") return projection.active;
  if (group === "attention") return projection.attention;
  return projection.history;
}
