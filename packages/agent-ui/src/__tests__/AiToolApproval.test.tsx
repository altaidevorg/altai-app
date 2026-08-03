import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AiToolApproval } from "../components/AiToolApproval.js";

describe("AiToolApproval", () => {
  it("renders tool label and actions", () => {
    const onRespond = vi.fn();
    const html = renderToStaticMarkup(
      createElement(AiToolApproval, {
        toolName: "write_file",
        onRespond,
        part: {
          state: "approval-requested",
          approval: { id: "a-1" },
          input: { path: "src/main.ts", content: "line\n" },
        },
      }),
    );
    expect(html).toContain("Write file");
    expect(html).toContain("src/main.ts");
    expect(html).toContain("Approve");
    expect(html).toContain("Deny");
    expect(html).toContain('role="alert"');
  });

  it("uses polite live region when assertiveAnnounce is false", () => {
    const html = renderToStaticMarkup(
      createElement(AiToolApproval, {
        toolName: "bash_run",
        assertiveAnnounce: false,
        onRespond: () => {},
        part: {
          state: "approval-requested",
          approval: { id: "a-2" },
          input: { command: "ls", cwd: "/tmp" },
        },
      }),
    );
    expect(html).toContain('role="status"');
    expect(html).toContain("ls");
  });
});
