import { describe, expect, it } from "vitest";
import {
  TASK_RUNS_EMPTY_ACTION_LABEL,
  TASK_RUNS_EMPTY_TITLE,
  TASK_RUNS_FILTERED_EMPTY_MESSAGE,
  TASK_RUNS_SEARCH_PLACEHOLDER,
} from "../lib/taskRunsListChrome.js";

describe("taskRunsListChrome", () => {
  it("exposes empty and filtered list copy", () => {
    expect(TASK_RUNS_EMPTY_TITLE).toContain("background");
    expect(TASK_RUNS_EMPTY_ACTION_LABEL).toContain("Start");
    expect(TASK_RUNS_FILTERED_EMPTY_MESSAGE).toContain("No tasks");
    expect(TASK_RUNS_SEARCH_PLACEHOLDER).toContain("Search");
  });
});
