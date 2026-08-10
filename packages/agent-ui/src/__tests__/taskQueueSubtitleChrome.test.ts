import { describe, expect, it } from "vitest";
import {
  TASK_CREATE_SURFACE_SUBTITLE,
  taskQueueSurfaceSubtitle,
} from "../lib/taskQueueSubtitleChrome.js";

describe("taskQueueSubtitleChrome", () => {
  it("formats queue and create subtitles", () => {
    expect(taskQueueSurfaceSubtitle(2, 1)).toBe(
      "2 working · 1 need attention",
    );
    expect(TASK_CREATE_SURFACE_SUBTITLE).toContain("Delegate");
  });
});
