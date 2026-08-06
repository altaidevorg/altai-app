import { openUrl } from "@tauri-apps/plugin-opener";
import {
  hrefToFilePath,
  isWebHref,
  resolveWorkspacePath,
} from "@altai/agent-ui";
import { currentWorkspaceFolder } from "@/modules/workspace/folder";

/** Open a workspace file in the editor via the app-wide event bus. */
export function openWorkspaceFile(path: string): void {
  const trimmed = path.trim();
  if (!trimmed) return;
  let resolved = trimmed;
  if (!trimmed.startsWith("/") && !/^[a-zA-Z]:[\\/]/.test(trimmed)) {
    const root = currentWorkspaceFolder();
    if (root) {
      try {
        resolved = resolveWorkspacePath(trimmed.replace(/^\.\//, ""), root);
      } catch {
        // Keep the original path; App's open handler will surface the miss.
      }
    }
  }
  window.dispatchEvent(
    new CustomEvent<string>("altai:open-file", { detail: resolved }),
  );
}

export { hrefToFilePath, isWebHref, resolveWorkspacePath };

/**
 * Open a chat markdown href: workspace files go to the editor, web URLs to
 * the system browser via Tauri's opener plugin.
 */
export async function openChatHref(href: string): Promise<void> {
  const trimmed = href.trim();
  if (!trimmed || trimmed === "streamdown:incomplete-link") return;

  if (isWebHref(trimmed)) {
    await openUrl(trimmed);
    return;
  }

  const filePath = hrefToFilePath(trimmed, currentWorkspaceFolder());
  if (filePath) {
    openWorkspaceFile(filePath);
    return;
  }

  // Bare host-looking strings (rare in markdown, common in tool output).
  if (/^[a-z0-9.-]+\.[a-z]{2,}([/:?]|$)/i.test(trimmed)) {
    await openUrl(`https://${trimmed}`);
  }
}
