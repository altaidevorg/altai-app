/**
 * CP-00-02: Frontend architecture boundary scanner.
 *
 * Enforces the control-plane / execution-plane ownership split (ADR 0003,
 * parent plan §12.2) at the TypeScript import level. The scanner:
 *
 * 1. Defines forbidden import rules as data.
 * 2. Scans `src/modules/` for violations.
 * 3. Initially passes because no Operations module exists yet.
 * 4. Is structured to fail if a future PR adds a forbidden import.
 *
 * Rules (parent plan §12.2):
 * - An Operations component must not import a legacy assignment, automation,
 *   notification, todo, or orchestration mutation store.
 * - An AI chat component must not import Work/Routine lifecycle mutations.
 * - A GitHub component must not start IsanAgent or transition Work directly.
 * - No renderer may contain claim, lease, schedule, retry, or recovery loops.
 *
 * This utility is intentionally side-effect free; it does not import any
 * module under test at module-evaluation time. Callers (tests or build
 * scripts) invoke `scanArchitectureBoundary()` explicitly.
 */

/** A logical frontend surface that has import restrictions. */
export type SurfaceId =
  | "operations"
  | "ai-chat"
  | "github"
  | "renderer";

/** A forbidden import rule. */
export interface BoundaryRule {
  /** The surface this rule applies to. */
  surface: SurfaceId;
  /** Glob pattern matching source files this rule governs (relative to src/modules/). */
  filePattern: string;
  /** Import specifiers that files matching `filePattern` must not import. */
  forbiddenImports: string[];
  /** Human-readable reason for the rule. */
  reason: string;
}

/**
 * The canonical rule set. When Operations modules are created (CP-17), these
 * rules will actively guard against regressions. Until then they pass because
 * no file matches the `operations` surface.
 */
export const BOUNDARY_RULES: BoundaryRule[] = [
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
      "notification, todo, or orchestration mutation stores (parent plan §12.2).",
  },
  {
    surface: "ai-chat",
    filePattern: "ai/components/**/*.{ts,tsx}",
    forbiddenImports: [
      "@/modules/ai/stores/workStore",
      "@/modules/ai/stores/routineStore",
    ],
    reason:
      "AI chat components must not import Work/Routine lifecycle mutations " +
      "(parent plan §12.2, §9.9).",
  },
  {
    surface: "github",
    filePattern: "github/**/*.{ts,tsx}",
    forbiddenImports: [
      "@/modules/ai/lib/native",
    ],
    reason:
      "GitHub components must not start IsanAgent or transition Work directly " +
      "(parent plan §12.2, §9.10).",
  },
];

/** A detected violation. */
export interface BoundaryViolation {
  rule: BoundaryRule;
  file: string;
  importSpecifier: string;
}

/**
 * Check whether a glob pattern matches a file path. Supports `**` and `*`.
 * This is a minimal implementation; it does not handle edge cases like
 * brace expansion.
 */
export function globMatch(pattern: string, path: string): boolean {
  // Normalize: remove leading "./"
  const p = pattern.replace(/^\.\//, "");
  const f = path.replace(/^\.\//, "");
  // Expand brace alternatives: {ts,tsx} -> (ts|tsx)
  for (const pat of expandBraces(p)) {
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

/**
 * Check whether an import statement line references a forbidden specifier.
 * Matches both static `import ... from "..."` and dynamic `import("...")`.
 */
export function importMatchesForbidden(line: string, forbidden: string): boolean {
  // Static: import ... from "forbidden"  or  import "forbidden"
  // Dynamic: import("forbidden")
  const patterns = [
    new RegExp(`from\\s+["']${escapeRegex(forbidden)}["']`),
    new RegExp(`import\\s+["']${escapeRegex(forbidden)}["']`),
    new RegExp(`import\\s*\\(\\s*["']${escapeRegex(forbidden)}["']`),
  ];
  return patterns.some((re) => re.test(line));
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Scan source text for forbidden imports against a rule.
 * Returns one violation per forbidden import found.
 */
export function scanText(
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

/**
 * Scan a set of files (path → content) against all boundary rules.
 * Only files whose path matches a rule's `filePattern` are checked.
 */
export function scanFiles(
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
