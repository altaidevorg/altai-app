import type { PluginCapability } from "./plugin.js";

// Schema-driven plugin UI declarations (package 073, PR 1). Mirrors
// src-tauri/crates/altai-control-protocol/src/plugin_ui.rs — keep both
// sides in sync, including the validation order (both sides must report
// the same first error for the same declaration).

export const MAX_SURFACES = 16;
export const MAX_DEPTH = 8;
export const MAX_NODES_PER_SURFACE = 256;
export const MAX_TABLE_COLUMNS = 16;
export const MAX_TABLE_ROWS = 500;

export type PluginUiDeclaration = {
  readonly surfaces: readonly PluginUiSurface[];
};

export type PluginUiSurface = {
  readonly surface_id: string;
  readonly title: string;
  readonly root: PluginUiNode;
};

export type PluginUiNode =
  | { readonly type: "section"; readonly title: string; readonly children: readonly PluginUiNode[] }
  | { readonly type: "text"; readonly label: string; readonly value: string }
  | {
      readonly type: "table";
      readonly columns: readonly string[];
      readonly rows: readonly (readonly string[])[];
    }
  | { readonly type: "action"; readonly label: string; readonly action: PluginUiAction };

export type PluginUiAction =
  | { readonly type: "invoke_job"; readonly job_id: string };

export type PluginUiError =
  | { readonly type: "missing_plugin_ui_capability" }
  | { readonly type: "empty_declaration" }
  | { readonly type: "too_many_surfaces"; readonly count: number }
  | { readonly type: "empty_surface_id" }
  | { readonly type: "duplicate_surface_id"; readonly surface_id: string }
  | { readonly type: "empty_title" }
  | { readonly type: "depth_exceeded"; readonly depth: number }
  | { readonly type: "too_many_nodes"; readonly count: number }
  | {
      readonly type: "row_width_mismatch";
      readonly row_index: number;
      readonly expected: number;
      readonly found: number;
    }
  | { readonly type: "too_many_table_columns"; readonly count: number }
  | { readonly type: "too_many_table_rows"; readonly count: number }
  | { readonly type: "empty_action_label" }
  | { readonly type: "empty_action_target" }
  | { readonly type: "action_capability_missing"; readonly capability: PluginCapability };

export const requiredCapability = (action: PluginUiAction): PluginCapability => {
  switch (action.type) {
    case "invoke_job":
      return "jobs";
  }
};

export const declaresAction = (
  declaration: PluginUiDeclaration,
  surface_id: string,
  action: PluginUiAction,
): boolean =>
  declaration.surfaces.some(
    (surface) => surface.surface_id === surface_id && nodeDeclares(surface.root, action),
  );

const actionsEqual = (a: PluginUiAction, b: PluginUiAction): boolean => {
  if (a.type !== b.type) return false;
  if (a.type === "invoke_job" && b.type === "invoke_job") return a.job_id === b.job_id;
  return false;
};

const nodeDeclares = (node: PluginUiNode, action: PluginUiAction): boolean => {
  switch (node.type) {
    case "section":
      return node.children.some((child) => nodeDeclares(child, action));
    case "action":
      return actionsEqual(node.action, action);
    case "text":
    case "table":
      return false;
  }
};

export const validatePluginUi = (
  declaration: PluginUiDeclaration,
  capabilities: readonly PluginCapability[],
): PluginUiError | null => {
  if (!capabilities.includes("plugin_ui")) {
    return { type: "missing_plugin_ui_capability" };
  }
  if (declaration.surfaces.length === 0) return { type: "empty_declaration" };
  if (declaration.surfaces.length > MAX_SURFACES) {
    return { type: "too_many_surfaces", count: declaration.surfaces.length };
  }
  const seen = new Set<string>();
  for (const surface of declaration.surfaces) {
    if (surface.surface_id.trim() === "") return { type: "empty_surface_id" };
    if (seen.has(surface.surface_id)) {
      return { type: "duplicate_surface_id", surface_id: surface.surface_id };
    }
    seen.add(surface.surface_id);
    if (surface.title.trim() === "") return { type: "empty_title" };
    const count = nodeCount(surface.root);
    if (count > MAX_NODES_PER_SURFACE) return { type: "too_many_nodes", count };
    const error = validateNode(surface.root, 1, capabilities);
    if (error) return error;
  }
  return null;
};

const nodeCount = (node: PluginUiNode): number =>
  1 + (node.type === "section" ? node.children.reduce((sum, child) => sum + nodeCount(child), 0) : 0);

const validateNode = (
  node: PluginUiNode,
  depth: number,
  capabilities: readonly PluginCapability[],
): PluginUiError | null => {
  if (depth > MAX_DEPTH) return { type: "depth_exceeded", depth };
  switch (node.type) {
    case "section":
      for (const child of node.children) {
        const error = validateNode(child, depth + 1, capabilities);
        if (error) return error;
      }
      return null;
    case "text":
      return null;
    case "table": {
      if (node.columns.length > MAX_TABLE_COLUMNS) {
        return { type: "too_many_table_columns", count: node.columns.length };
      }
      if (node.rows.length > MAX_TABLE_ROWS) {
        return { type: "too_many_table_rows", count: node.rows.length };
      }
      for (const [row_index, row] of node.rows.entries()) {
        if (row.length !== node.columns.length) {
          return {
            type: "row_width_mismatch",
            row_index,
            expected: node.columns.length,
            found: row.length,
          };
        }
      }
      return null;
    }
    case "action": {
      if (node.label.trim() === "") return { type: "empty_action_label" };
      const capability = requiredCapability(node.action);
      if (!capabilities.includes(capability)) {
        return { type: "action_capability_missing", capability };
      }
      if (node.action.type === "invoke_job" && node.action.job_id.trim() === "") {
        return { type: "empty_action_target" };
      }
      return null;
    }
  }
};
