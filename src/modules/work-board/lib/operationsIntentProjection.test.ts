import { describe, expect, it } from "vitest";
import { homeSurfaceFromOperationsIntent } from "./operationsIntentProjection";

describe("homeSurfaceFromOperationsIntent", () => {
  it("lands scheduled work on the Routines surface", () => {
    expect(homeSurfaceFromOperationsIntent("work", "scheduled")).toBe(
      "routines",
    );
    expect(
      homeSurfaceFromOperationsIntent("overview", "scheduled"),
    ).toBe("routines");
  });

  it("lands every other view on the Work surface, whose column carries inbox and runs", () => {
    expect(homeSurfaceFromOperationsIntent("overview")).toBe("work");
    expect(homeSurfaceFromOperationsIntent("work")).toBe("work");
    expect(homeSurfaceFromOperationsIntent("work", "runs")).toBe("work");
    expect(homeSurfaceFromOperationsIntent("runs")).toBe("work");
    expect(homeSurfaceFromOperationsIntent("inbox")).toBe("work");
    expect(homeSurfaceFromOperationsIntent("inbox", null)).toBe("work");
  });
});
