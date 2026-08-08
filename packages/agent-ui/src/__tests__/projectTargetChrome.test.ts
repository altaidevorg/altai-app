import { describe, expect, it } from "vitest";
import {
  basenamePath,
  canMountProjectTarget,
  projectTargetFromWorkspace,
  retainPreferredRoot,
} from "../lib/projectTargetChrome.js";

describe("canMountProjectTarget", () => {
  it("requires workspace.info", () => {
    expect(canMountProjectTarget({ workspaceInfo: true })).toBe(true);
    expect(canMountProjectTarget({ workspaceInfo: false })).toBe(false);
  });
});

describe("projectTargetFromWorkspace", () => {
  it("returns empty target without roots", () => {
    expect(projectTargetFromWorkspace(null)).toEqual({
      name: "Choose a project",
      path: null,
      kind: null,
      rootUri: null,
      multiRoot: false,
    });
  });

  it("prefers currentDir for path and name on single root", () => {
    expect(
      projectTargetFromWorkspace({
        roots: ["file:///Users/me/altai-vscode"],
        currentDir: "/Users/me/altai-vscode",
        trusted: true,
      }),
    ).toEqual({
      name: "altai-vscode",
      path: "/Users/me/altai-vscode",
      kind: "local",
      rootUri: "file:///Users/me/altai-vscode",
      multiRoot: false,
    });
  });

  it("derives path from file URI when currentDir missing", () => {
    const view = projectTargetFromWorkspace({
      roots: ["file:///tmp/demo%20app"],
    });
    expect(view.name).toBe("demo app");
    expect(view.path).toBe("/tmp/demo app");
    expect(view.kind).toBe("local");
  });

  it("selects preferred multi-root uri", () => {
    const view = projectTargetFromWorkspace(
      {
        roots: ["file:///Users/me/a", "file:///Users/me/b"],
        currentDir: "/Users/me/a",
      },
      "file:///Users/me/b",
    );
    expect(view.rootUri).toBe("file:///Users/me/b");
    expect(view.name).toBe("b");
    expect(view.multiRoot).toBe(true);
  });
});

describe("retainPreferredRoot", () => {
  it("drops preference when root removed", () => {
    expect(retainPreferredRoot("file:///a", ["file:///b"])).toBeNull();
    expect(retainPreferredRoot("file:///a", ["file:///a"])).toBe("file:///a");
  });
});

describe("basenamePath", () => {
  it("handles trailing separators", () => {
    expect(basenamePath("/a/b/")).toBe("b");
    expect(basenamePath("C:\\foo\\bar\\")).toBe("bar");
  });
});
