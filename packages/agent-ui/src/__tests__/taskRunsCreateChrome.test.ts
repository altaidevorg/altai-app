import { describe, expect, it } from "vitest";
import {
  TASK_RUNS_CREATE_TITLE,
  TASK_RUNS_START_ERROR,
} from "../lib/taskRunsCreateChrome.js";

describe("taskRunsCreateChrome", () => {
  it("exposes create form chrome", () => {
    expect(TASK_RUNS_CREATE_TITLE).toContain("outcome");
    expect(TASK_RUNS_START_ERROR).toContain("Couldn't start");
  });
});
