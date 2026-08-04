import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  contextFileName,
  TaskContextSources,
} from "../components/TaskContextSources.js";

describe("contextFileName", () => {
  it("returns the basename for posix and windows paths", () => {
    expect(contextFileName("/tmp/foo/bar.ts")).toBe("bar.ts");
    expect(contextFileName("C:\\proj\\main.rs")).toBe("main.rs");
  });
});

describe("TaskContextSources", () => {
  it("renders file chips and source toggles", () => {
    const html = renderToStaticMarkup(
      createElement(TaskContextSources, {
        files: ["/workspace/src/app.ts"],
        onAddActiveFile: () => {},
        onChooseFiles: () => {},
        onRemoveFile: () => {},
        activeFileDisabled: false,
        activeFileSelected: false,
        includeTerminal: true,
        onIncludeTerminalChange: () => {},
        terminalDetail: "Latest visible output from the active terminal",
        terminalDisabled: false,
        includeDiff: false,
        onIncludeDiffChange: () => {},
        diffDetail: "Open a workspace to include Git changes",
        diffDisabled: true,
      }),
    );
    expect(html).toContain("Context");
    expect(html).toContain("app.ts");
    expect(html).toContain("Remove app.ts");
    expect(html).toContain("Active file");
    expect(html).toContain("Choose files");
    expect(html).toContain("Terminal output");
    expect(html).toContain("Working tree changes");
  });

  it("shows Active added when the active file is already selected", () => {
    const html = renderToStaticMarkup(
      createElement(TaskContextSources, {
        files: [],
        onAddActiveFile: () => {},
        onChooseFiles: () => {},
        onRemoveFile: () => {},
        activeFileDisabled: true,
        activeFileSelected: true,
        includeTerminal: false,
        onIncludeTerminalChange: () => {},
        terminalDetail: "No terminal output available",
        terminalDisabled: true,
        includeDiff: false,
        onIncludeDiffChange: () => {},
        diffDetail: "Current unstaged Git diff",
        diffDisabled: false,
      }),
    );
    expect(html).toContain("Active added");
  });
});
