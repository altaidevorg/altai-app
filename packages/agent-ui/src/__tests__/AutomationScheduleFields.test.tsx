import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  AutomationScheduleFields,
  localDateTimeValue,
} from "../components/AutomationScheduleFields.js";

describe("localDateTimeValue", () => {
  it("returns a datetime-local length string", () => {
    expect(localDateTimeValue(Date.parse("2026-08-04T12:00:00Z"))).toHaveLength(
      16,
    );
  });
});

describe("AutomationScheduleFields", () => {
  it("renders once mode inputs and quick sets", () => {
    const html = renderToStaticMarkup(
      createElement(AutomationScheduleFields, {
        mode: "at",
        onModeChange: () => {},
        atValue: "2026-08-04T15:00",
        onAtValueChange: () => {},
        everyMinutes: "60",
        onEveryMinutesChange: () => {},
        nowMs: Date.parse("2026-08-04T12:00:00Z"),
      }),
    );
    expect(html).toContain("Schedule");
    expect(html).toContain("Once");
    expect(html).toContain("Repeat");
    expect(html).toContain("Automation run time");
    expect(html).toContain("In 15 min");
    expect(html).toContain("Daily");
  });

  it("renders repeat interval input", () => {
    const html = renderToStaticMarkup(
      createElement(AutomationScheduleFields, {
        mode: "every",
        onModeChange: () => {},
        atValue: "",
        onAtValueChange: () => {},
        everyMinutes: "45",
        onEveryMinutesChange: () => {},
      }),
    );
    expect(html).toContain("Repeat interval in minutes");
    expect(html).toContain('value="45"');
  });
});
