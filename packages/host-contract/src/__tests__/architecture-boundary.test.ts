/**
 * CP-00-02: Frontend architecture boundary tests.
 *
 * These tests enforce the control-plane / execution-plane ownership split
 * (ADR 0003, parent plan §12.2) at the TypeScript import level. They:
 *
 * 1. Pass today because no Operations module exists yet.
 * 2. Self-verify that a simulated forbidden import would be detected.
 * 3. Are structured to fail if a future PR adds a forbidden import.
 *
 * The scanning logic is duplicated here (rather than imported from
 * `src/lib/architectureBoundary.ts`) so the host-contract package remains
 * self-contained and does not depend on the main application source tree.
 */

import { describe, expect, it } from "vitest";

// ---------------------------------------------------------------------------
// Boundary rule definitions (mirror of src/lib/architectureBoundary.ts).
// ---------------------------------------------------------------------------

type SurfaceId = "operations" | "ai-chat" | "github" | "renderer";

interface BoundaryRule {
  surface: SurfaceId;
  filePattern: string;
  forbiddenImports: string[];
  reason: string;
}

interface BoundaryViolation {
  rule: BoundaryRule;
  file: string;
  importSpecifier: string;
}

const BOUNDARY_RULES: BoundaryRule[] = [
  {
    surface: "operations",
    filePattern: "operations/**/*.{ts,tsx}",
    forbiddenImports: [
      "@/modules/ai/stores/assignmentsStore",
      "@/modules/ai/stores/automationStore",
      "@/modules/ai/stores/notificationStore",
      "@/modules/orchestration/store",
      "@/modules/orchestration/OrchestrationController",
    ],
    reason:
      "Operations components must not import legacy assignment, automation, " +
      "notification, todo, or orchestration mutation stores.",
  },
  {
    surface: "ai-chat",
    filePattern: "ai/components/**/*.{ts,tsx}",
    forbiddenImports: [
      "@/modules/ai/stores/workStore",
      "@/modules/ai/stores/routineStore",
    ],
    reason:
      "AI chat components must not import Work/Routine lifecycle mutations.",
  },
  {
    surface: "github",
    filePattern: "github/**/*.{ts,tsx}",
    forbiddenImports: ["@/modules/ai/lib/native"],
    reason:
      "GitHub components must not start IsanAgent or transition Work directly.",
  },
];

// ---------------------------------------------------------------------------
// Scanning logic (mirror of src/lib/architectureBoundary.ts).
// ---------------------------------------------------------------------------

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function globMatch(pattern: string, path: string): boolean {
  const p = pattern.replace(/^\.\//, "");
  const f = path.replace(/^\.\//, "");
  // Expand brace alternatives: {ts,tsx} -> (ts|tsx)
  const expanded = expandBraces(p);
  for (const pat of expanded) {
    const regexStr = pat
      .replace(/[.+^${}()|[\]\\]/g, "\\$&")
      .replace(/\/\*\*\//g, "/\x00") // **/  -> keep leading slash, drop trailing
      .replace(/\*\*/g, "\x00")
      .replace(/\*/g, "[^/]*")
      .replace(/\x00/g, "(?:[^/]+/)*");
    if (new RegExp(`^${regexStr}$`).test(f)) return true;
  }
  return false;
}

function expandBraces(pattern: string): string[] {
  const start = pattern.indexOf("{");
  if (start === -1) return [pattern];
  const end = pattern.indexOf("}", start);
  if (end === -1) return [pattern];
  const prefix = pattern.slice(0, start);
  const suffix = pattern.slice(end + 1);
  const options = pattern.slice(start + 1, end).split(",");
  const results: string[] = [];
  for (const opt of options) {
    for (const sub of expandBraces(suffix)) {
      results.push(prefix + opt.trim() + sub);
    }
  }
  return results;
}

function importMatchesForbidden(line: string, forbidden: string): boolean {
  const patterns = [
    new RegExp(`from\\s+["']${escapeRegex(forbidden)}["']`),
    new RegExp(`import\\s+["']${escapeRegex(forbidden)}["']`),
    new RegExp(`import\\s*\\(\\s*["']${escapeRegex(forbidden)}["']`),
  ];
  return patterns.some((re) => re.test(line));
}

function scanText(
  sourceText: string,
  rule: BoundaryRule,
  filePath: string,
): BoundaryViolation[] {
  const violations: BoundaryViolation[] = [];
  for (const line of sourceText.split("\n")) {
    for (const forbidden of rule.forbiddenImports) {
      if (importMatchesForbidden(line, forbidden)) {
        violations.push({ rule, file: filePath, importSpecifier: forbidden });
      }
    }
  }
  return violations;
}

function scanFiles(
  files: Array<{ path: string; content: string }>,
  rules: BoundaryRule[] = BOUNDARY_RULES,
): BoundaryViolation[] {
  const violations: BoundaryViolation[] = [];
  for (const file of files) {
    for (const rule of rules) {
      if (globMatch(rule.filePattern, file.path)) {
        violations.push(...scanText(file.content, rule, file.path));
      }
    }
  }
  return violations;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("architecture boundary — glob matching", () => {
  it("matches operations files with ** and *", () => {
    expect(globMatch("operations/**/*.{ts,tsx}", "operations/WorkBoard.tsx")).toBe(true);
    expect(globMatch("operations/**/*.{ts,tsx}", "operations/sub/Detail.ts")).toBe(true);
    expect(globMatch("operations/**/*.{ts,tsx}", "ai/components/Chat.tsx")).toBe(false);
  });

  it("matches ai-chat components", () => {
    expect(globMatch("ai/components/**/*.{ts,tsx}", "ai/components/Chat.tsx")).toBe(true);
    expect(globMatch("ai/components/**/*.{ts,tsx}", "operations/Work.tsx")).toBe(false);
  });

  it("matches github files", () => {
    expect(globMatch("github/**/*.{ts,tsx}", "github/Overview.tsx")).toBe(true);
    expect(globMatch("github/**/*.{ts,tsx}", "ai/lib/native.ts")).toBe(false);
  });
});

describe("architecture boundary — import detection", () => {
  it("detects static import from forbidden module", () => {
    const line = 'import { assignmentsStore } from "@/modules/ai/stores/assignmentsStore";';
    expect(importMatchesForbidden(line, "@/modules/ai/stores/assignmentsStore")).toBe(true);
  });

  it("detects side-effect import from forbidden module", () => {
    const line = 'import "@/modules/orchestration/store";';
    expect(importMatchesForbidden(line, "@/modules/orchestration/store")).toBe(true);
  });

  it("detects dynamic import of forbidden module", () => {
    const line = 'await import("@/modules/ai/stores/workStore")';
    expect(importMatchesForbidden(line, "@/modules/ai/stores/workStore")).toBe(true);
  });

  it("does not match unrelated imports", () => {
    const line = 'import { useChatStore } from "@/modules/ai/stores/chatStore";';
    expect(importMatchesForbidden(line, "@/modules/ai/stores/assignmentsStore")).toBe(false);
  });
});

describe("architecture boundary — scanFiles", () => {
  it("passes with no files (current state: no Operations module exists)", () => {
    const violations = scanFiles([]);
    expect(violations).toEqual([]);
  });

  it("passes with clean operations file (no forbidden imports)", () => {
    const files = [
      {
        path: "operations/WorkBoard.tsx",
        content: 'import { useWorkStore } from "@/modules/operations/workStore";',
      },
    ];
    expect(scanFiles(files)).toEqual([]);
  });

  it("detects violation when operations file imports legacy store", () => {
    const files = [
      {
        path: "operations/WorkBoard.tsx",
        content:
          'import { assignmentsStore } from "@/modules/ai/stores/assignmentsStore";\n' +
          'export function Board() { return null; }',
      },
    ];
    const violations = scanFiles(files);
    expect(violations).toHaveLength(1);
    expect(violations[0].importSpecifier).toBe("@/modules/ai/stores/assignmentsStore");
    expect(violations[0].file).toBe("operations/WorkBoard.tsx");
  });

  it("detects violation when ai-chat file imports work lifecycle", () => {
    const files = [
      {
        path: "ai/components/Chat.tsx",
        content: 'import { workStore } from "@/modules/ai/stores/workStore";',
      },
    ];
    const violations = scanFiles(files);
    expect(violations).toHaveLength(1);
    expect(violations[0].rule.surface).toBe("ai-chat");
  });

  it("detects violation when github file imports native invoke", () => {
    const files = [
      {
        path: "github/Overview.tsx",
        content: 'import { invoke } from "@/modules/ai/lib/native";',
      },
    ];
    const violations = scanFiles(files);
    expect(violations).toHaveLength(1);
    expect(violations[0].rule.surface).toBe("github");
  });

  it("does not flag non-matching surfaces", () => {
    const files = [
      {
        path: "settings/General.tsx",
        content: 'import { assignmentsStore } from "@/modules/ai/stores/assignmentsStore";',
      },
    ];
    // settings/ does not match any rule's filePattern, so no violation.
    expect(scanFiles(files)).toEqual([]);
  });

  it("reports multiple violations in one file", () => {
    const files = [
      {
        path: "operations/Overview.tsx",
        content:
          'import { assignmentsStore } from "@/modules/ai/stores/assignmentsStore";\n' +
          'import { automationStore } from "@/modules/ai/stores/automationStore";',
      },
    ];
    const violations = scanFiles(files);
    expect(violations).toHaveLength(2);
  });
});
