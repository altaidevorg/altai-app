import { isAbsolute, relative, resolve, sep } from "node:path";
import { MAX_CONTEXT_BYTES, MAX_CONTEXT_ITEM_BYTES, MAX_CONTEXT_ITEMS, MAX_DIAGNOSTICS, utf8Bytes } from "./limits.js";

export type ContextResource = {
  readonly uri: string;
  readonly fsPath: string;
  readonly scheme: string;
  readonly isUntitled?: boolean;
};

export type ContextWorkspace = {
  readonly name: string;
  readonly fsPath: string;
  readonly scheme: string;
};

export type ContextRange = { readonly startLine: number; readonly endLine: number };

export type ContextItem = {
  readonly id: string;
  readonly kind: "selection" | "file" | "diagnostics";
  readonly label: string;
  readonly uri: string;
  readonly range?: ContextRange;
  /** Reference material only. It never crosses the webview boundary. */
  readonly content: string;
};

export type ContextChip = Pick<ContextItem, "id" | "kind" | "label" | "uri" | "range">;

export type ContextFileSystem = {
  canonicalize(path: string): Promise<string>;
  readFile(resource: ContextResource): Promise<Uint8Array>;
};

export class ContextError extends Error {
  constructor(
    public readonly reason:
      | "untitled_file"
      | "virtual_file"
      | "outside_workspace"
      | "missing_file"
      | "binary_file"
      | "file_too_large"
      | "empty_selection"
      | "too_many_items"
      | "context_too_large"
      | "different_workspace"
      | "untrusted_workspace",
  ) {
    super(reason);
  }
}

/**
 * Collects context from VS Code-owned data. The Webview only receives the
 * display-safe chips, never URIs/content it gets to choose for a run.
 */
export class ContextCollector {
  constructor(private readonly fs: ContextFileSystem) {}

  async selection(resource: ContextResource, workspace: ContextWorkspace, text: string, range: ContextRange, label = "Selection"): Promise<ContextItem> {
    if (text.trim().length === 0) throw new ContextError("empty_selection");
    await this.assertResourceInWorkspace(resource, workspace);
    this.assertSourceSize(text);
    return {
      id: `selection:${resource.uri}:${range.startLine}-${range.endLine}`,
      kind: "selection",
      label: `${label} (lines ${range.startLine}-${range.endLine})`,
      uri: resource.uri,
      range,
      content: text,
    };
  }

  async file(resource: ContextResource, workspace: ContextWorkspace, label = "File"): Promise<ContextItem> {
    await this.assertResourceInWorkspace(resource, workspace);
    let bytes: Uint8Array;
    try {
      bytes = await this.fs.readFile(resource);
    } catch {
      throw new ContextError("missing_file");
    }
    if (bytes.byteLength > MAX_CONTEXT_ITEM_BYTES) throw new ContextError("file_too_large");
    const content = decodeText(bytes);
    if (content === undefined) throw new ContextError("binary_file");
    return { id: `file:${resource.uri}`, kind: "file", label, uri: resource.uri, content };
  }

  async diagnostics(resource: ContextResource, workspace: ContextWorkspace, diagnostics: readonly ContextDiagnostic[]): Promise<ContextItem | undefined> {
    await this.assertResourceInWorkspace(resource, workspace);
    const bounded = diagnostics.slice(0, MAX_DIAGNOSTICS).map((diagnostic) => ({
      severity: diagnostic.severity,
      message: diagnostic.message,
      code: diagnostic.code,
      range: diagnostic.range,
    }));
    if (bounded.length === 0) return undefined;
    const content = JSON.stringify(bounded);
    this.assertSourceSize(content);
    return { id: `diagnostics:${resource.uri}`, kind: "diagnostics", label: `${bounded.length} diagnostic${bounded.length === 1 ? "" : "s"}`, uri: resource.uri, content };
  }

  assertCollection(items: readonly ContextItem[]): void {
    if (items.length > MAX_CONTEXT_ITEMS) throw new ContextError("too_many_items");
    const total = items.reduce((sum, item) => sum + utf8Bytes(item.content), 0);
    if (total > MAX_CONTEXT_BYTES) throw new ContextError("context_too_large");
  }

  private async assertResourceInWorkspace(resource: ContextResource, workspace: ContextWorkspace): Promise<void> {
    if (resource.isUntitled) throw new ContextError("untitled_file");
    if (!isRunnableScheme(resource.scheme) || resource.scheme !== workspace.scheme) throw new ContextError("virtual_file");
    let root: string;
    let target: string;
    try {
      [root, target] = await Promise.all([this.fs.canonicalize(workspace.fsPath), this.fs.canonicalize(resource.fsPath)]);
    } catch {
      throw new ContextError("missing_file");
    }
    if (!isInside(root, target)) throw new ContextError("outside_workspace");
  }

  private assertSourceSize(content: string): void {
    if (utf8Bytes(content) > MAX_CONTEXT_ITEM_BYTES) throw new ContextError("file_too_large");
  }
}

export type ContextDiagnostic = {
  readonly severity: string;
  readonly message: string;
  readonly code?: string | number;
  readonly range: ContextRange;
};

export function toChip(item: ContextItem): ContextChip {
  const { id, kind, label, uri, range } = item;
  return range === undefined ? { id, kind, label, uri } : { id, kind, label, uri, range };
}

function isRunnableScheme(scheme: string): boolean {
  return scheme === "file" || scheme === "vscode-remote";
}

function isInside(root: string, target: string): boolean {
  const rel = relative(resolve(root), resolve(target));
  // Only `..` as a path segment escapes the root. A file named `..notes` is a
  // legitimate child and must not be confused with traversal.
  return rel === "" || (rel !== ".." && !rel.startsWith(`..${sep}`) && !isAbsolute(rel));
}

function decodeText(bytes: Uint8Array): string | undefined {
  if (bytes.includes(0)) return undefined;
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return undefined;
  }
}
