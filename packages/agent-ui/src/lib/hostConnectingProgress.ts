/**
 * Pure title/message for host-connect progress chrome (A6.110).
 */

export function hostConnectingProgressPresentation(input: {
  status: string;
  message?: string;
}): { show: boolean; title: string } {
  if (input.status !== "connecting") {
    return { show: false, title: "" };
  }
  const detail = input.message?.trim();
  return {
    show: true,
    title: detail
      ? `ALTAI agent host: ${detail}`
      : "ALTAI agent host is starting…",
  };
}
