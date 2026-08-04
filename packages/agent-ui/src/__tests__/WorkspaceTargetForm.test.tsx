import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { WorkspaceTargetForm } from "../components/WorkspaceTargetForm.js";

const base = {
  repoUrl: "",
  onRepoUrlChange: () => {},
  onChooseLocal: () => {},
  onCloneGithub: () => {},
};

describe("WorkspaceTargetForm", () => {
  it("renders project choices", () => {
    const html = renderToStaticMarkup(
      createElement(WorkspaceTargetForm, base),
    );
    expect(html).toContain("Choose a project");
    expect(html).toContain("Local workspace");
    expect(html).toContain("GitHub repository");
    expect(html).toContain("Clone");
    expect(html).not.toContain("Continue without a project");
  });

  it("shows busy, error, and clear affordances", () => {
    const html = renderToStaticMarkup(
      createElement(WorkspaceTargetForm, {
        ...base,
        busy: "github",
        repoUrl: "https://github.com/org/repo.git",
        error: "Clone failed",
        showClearProject: true,
        onClearProject: () => {},
      }),
    );
    expect(html).toContain("Cloning…");
    expect(html).toContain("Clone failed");
    expect(html).toContain("Continue without a project");
    expect(html).toContain('role="alert"');
  });
});
