import { describe, expect, it } from "vitest";
import { redactSensitive } from "../lib/redactSensitive.js";

describe("redactSensitive", () => {
  it("redacts openai keys and env assigns", () => {
    expect(redactSensitive("key sk-abcdefghijklmnopqrstuvwxyz1234 ok")).toContain(
      "<REDACTED:openai-key>",
    );
    expect(redactSensitive("API_KEY=supersecretvalue123")).toBe(
      "API_KEY=<REDACTED>",
    );
  });

  it("leaves ordinary text alone", () => {
    expect(redactSensitive("hello world")).toBe("hello world");
  });
});
