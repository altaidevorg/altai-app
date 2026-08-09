import { describe, expect, it } from "vitest";
import {
  cancellationRequestedToast,
  compactionRequestedToast,
  openedChangeReviewToast,
  openedChatSessionsToast,
  openedRunDetailsToast,
  renamedActiveChatToast,
  renameUsageToast,
  retryingLastRequestToast,
  startedNewChatToast,
} from "../lib/slashSessionToast.js";

describe("slashSessionToast", () => {
  it("returns stable copy", () => {
    expect(startedNewChatToast()).toBe("Started a new chat");
    expect(openedChatSessionsToast()).toBe("Opened chat sessions");
    expect(renameUsageToast()).toMatch(/rename/);
    expect(renamedActiveChatToast()).toBe("Renamed active chat");
    expect(retryingLastRequestToast()).toMatch(/Retrying/);
    expect(cancellationRequestedToast()).toMatch(/Cancellation/);
    expect(compactionRequestedToast()).toMatch(/Compaction/);
    expect(openedRunDetailsToast()).toMatch(/run details/);
    expect(openedChangeReviewToast()).toMatch(/change review/);
  });
});
