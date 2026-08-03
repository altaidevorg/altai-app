type TauriWindowMetadata = {
  currentWindow?: { label?: unknown };
  currentWebview?: { label?: unknown };
};

type TauriRuntimeGlobal = {
  isTauri?: unknown;
  __TAURI_INTERNALS__?: { metadata?: TauriWindowMetadata };
};

/**
 * Tauri's JavaScript package is importable from a normal Vite browser tab, but
 * current-window helpers synchronously dereference metadata that only the
 * native WebView injects. Check that exact prerequisite before calling them.
 */
export function hasTauriWindowMetadata(
  target: TauriRuntimeGlobal = globalThis as TauriRuntimeGlobal,
): boolean {
  const metadata = target.__TAURI_INTERNALS__?.metadata;
  return Boolean(
    target.isTauri &&
      typeof metadata?.currentWindow?.label === "string" &&
      typeof metadata.currentWebview?.label === "string",
  );
}
