import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SessionRow } from "../components/SessionRow.js";

describe("SessionRow", () => {
  const baseProps = {
    title: "My chat session",
    active: false,
    renaming: false,
    renameValue: "",
    onPick: () => {},
    onStartRename: () => {},
    onCommitRename: () => {},
    onCancelRename: () => {},
    onRenameValueChange: () => {},
    onDelete: () => {},
    renameInputRef: { current: null } as React.RefObject<HTMLInputElement | null>,
  };

  it("renders title and snippet", () => {
    const html = renderToStaticMarkup(
      createElement(SessionRow, { ...baseProps, snippet: "Latest message preview" }),
    );
    expect(html).toContain("My chat session");
    expect(html).toContain("Latest message preview");
    expect(html).toContain('role="button"');
  });

  it("falls back to New chat when title is empty", () => {
    const html = renderToStaticMarkup(
      createElement(SessionRow, { ...baseProps, title: "" }),
    );
    expect(html).toContain("New chat");
  });

  it("applies active styling", () => {
    const html = renderToStaticMarkup(
      createElement(SessionRow, { ...baseProps, active: true }),
    );
    expect(html).toContain("bg-accent text-foreground");
  });

  it("renders rename input when renaming", () => {
    const html = renderToStaticMarkup(
      createElement(SessionRow, {
        ...baseProps,
        renaming: true,
        renameValue: "Renamed session",
      }),
    );
    expect(html).toContain('<input');
    expect(html).toContain('value="Renamed session"');
    expect(html).toContain('tabindex="-1"');
  });
});
