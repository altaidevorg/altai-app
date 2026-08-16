import { describe, expect, test } from "vitest";
import {
  declaresAction,
  MAX_DEPTH,
  MAX_NODES_PER_SURFACE,
  MAX_SURFACES,
  MAX_TABLE_COLUMNS,
  MAX_TABLE_ROWS,
  validatePluginUi,
  type PluginUiDeclaration,
  type PluginUiNode,
  type PluginUiSurface,
} from "../plugin-ui.js";

const uiCapabilities = ["plugin_ui", "jobs"] as const;

const surface = (surface_id: string, root: PluginUiNode): PluginUiSurface => ({
  surface_id,
  title: "Surface",
  root,
});

const declaration = (roots: readonly PluginUiNode[]): PluginUiDeclaration => ({
  surfaces: roots.map((root, index) => surface(`surface_${index}`, root)),
});

const text = (): PluginUiNode => ({ type: "text", label: "Hi", value: "there" });

const jobAction = (job_id: string): PluginUiNode => ({
  type: "action",
  label: "Run",
  action: { type: "invoke_job", job_id },
});

const nested = (depth: number, leaf: PluginUiNode): PluginUiNode => {
  let node = leaf;
  for (let level = 1; level < depth; level += 1) {
    node = { type: "section", title: "Level", children: [node] };
  }
  return node;
};

describe("plugin ui declaration", () => {
  test("a sound declaration validates", () => {
    expect(validatePluginUi(declaration([nested(4, jobAction("job_1"))]), [...uiCapabilities])).toBeNull();
  });

  test("a declaration without the plugin_ui capability is refused", () => {
    expect(validatePluginUi(declaration([text()]), ["jobs"])).toEqual({
      type: "missing_plugin_ui_capability",
    });
  });

  test("empty and oversized declarations are refused", () => {
    expect(validatePluginUi({ surfaces: [] }, [...uiCapabilities])).toEqual({
      type: "empty_declaration",
    });
    const oversized: PluginUiDeclaration = {
      surfaces: Array.from({ length: MAX_SURFACES + 1 }, (_, index) => surface(`s_${index}`, text())),
    };
    expect(validatePluginUi(oversized, [...uiCapabilities])).toEqual({
      type: "too_many_surfaces",
      count: MAX_SURFACES + 1,
    });
  });

  test("surface ids must be present and unique; titles non-empty", () => {
    expect(
      validatePluginUi({ surfaces: [surface("  ", text())] }, [...uiCapabilities]),
    ).toEqual({ type: "empty_surface_id" });
    expect(
      validatePluginUi({ surfaces: [surface("same", text()), surface("same", text())] }, [...uiCapabilities]),
    ).toEqual({ type: "duplicate_surface_id", surface_id: "same" });
    expect(
      validatePluginUi(
        { surfaces: [{ surface_id: "main", title: "  ", root: text() }] },
        [...uiCapabilities],
      ),
    ).toEqual({ type: "empty_title" });
  });

  test("depth and node budgets are enforced", () => {
    expect(
      validatePluginUi(declaration([nested(MAX_DEPTH + 1, text())]), [...uiCapabilities]),
    ).toEqual({ type: "depth_exceeded", depth: MAX_DEPTH + 1 });
    expect(
      validatePluginUi(declaration([nested(MAX_DEPTH, text())]), [...uiCapabilities]),
    ).toBeNull();

    const wide: PluginUiNode = {
      type: "section",
      title: "Wide",
      children: Array.from({ length: MAX_NODES_PER_SURFACE }, text),
    };
    expect(validatePluginUi(declaration([wide]), [...uiCapabilities])).toEqual({
      type: "too_many_nodes",
      count: MAX_NODES_PER_SURFACE + 1,
    });
    const fits: PluginUiNode = {
      type: "section",
      title: "Wide",
      children: Array.from({ length: MAX_NODES_PER_SURFACE - 1 }, text),
    };
    expect(validatePluginUi(declaration([fits]), [...uiCapabilities])).toBeNull();
  });

  test("tables must match their columns and their bounds", () => {
    expect(
      validatePluginUi(
        declaration([
          { type: "table", columns: ["A", "B"], rows: [["only-a"]] },
        ]),
        [...uiCapabilities],
      ),
    ).toEqual({ type: "row_width_mismatch", row_index: 0, expected: 2, found: 1 });
    expect(
      validatePluginUi(
        declaration([
          {
            type: "table",
            columns: Array.from({ length: MAX_TABLE_COLUMNS + 1 }, () => "C"),
            rows: [],
          },
        ]),
        [...uiCapabilities],
      ),
    ).toEqual({ type: "too_many_table_columns", count: MAX_TABLE_COLUMNS + 1 });
    expect(
      validatePluginUi(
        declaration([
          {
            type: "table",
            columns: ["C"],
            rows: Array.from({ length: MAX_TABLE_ROWS + 1 }, () => ["r"]),
          },
        ]),
        [...uiCapabilities],
      ),
    ).toEqual({ type: "too_many_table_rows", count: MAX_TABLE_ROWS + 1 });
  });

  test("the declaration is the action whitelist", () => {
    const declaration: PluginUiDeclaration = {
      surfaces: [surface("panel", jobAction("job_refresh")), surface("other", text())],
    };
    const action = jobAction("job_refresh").action;
    expect(declaresAction(declaration, "panel", action)).toBe(true);
    // Right action, wrong surface.
    expect(declaresAction(declaration, "other", action)).toBe(false);
    // Right surface, undeclared job.
    expect(declaresAction(declaration, "panel", { type: "invoke_job", job_id: "nope" })).toBe(false);
    // Unknown surface.
    expect(declaresAction(declaration, "nowhere", action)).toBe(false);
    // A nested action is still declared.
    const nestedDeclaration: PluginUiDeclaration = {
      surfaces: [surface("panel", nested(3, jobAction("job_deep")))],
    };
    expect(declaresAction(nestedDeclaration, "panel", { type: "invoke_job", job_id: "job_deep" })).toBe(true);
  });

  test("actions need a label, a target, and their capability", () => {
    expect(
      validatePluginUi(
        declaration([
          { type: "action", label: " ", action: { type: "invoke_job", job_id: "job_1" } },
        ]),
        [...uiCapabilities],
      ),
    ).toEqual({ type: "empty_action_label" });
    expect(
      validatePluginUi(
        declaration([{ type: "action", label: "Run", action: { type: "invoke_job", job_id: "" } }]),
        [...uiCapabilities],
      ),
    ).toEqual({ type: "empty_action_target" });
    expect(validatePluginUi(declaration([jobAction("job_1")]), ["plugin_ui"])).toEqual({
      type: "action_capability_missing",
      capability: "jobs",
    });
  });
});
