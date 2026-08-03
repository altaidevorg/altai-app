import { hasTauriWindowMetadata } from "@/lib/tauriWindow";
import {
  emit as emitTauriEvent,
  listen as listenTauriEvent,
} from "@tauri-apps/api/event";

export type UnlistenFn = () => void;

export type AppEvent<T> = {
  event: string;
  id: number;
  payload: T;
};

type BrowserListener = (event: AppEvent<unknown>) => void;
const browserListeners = new Map<string, Set<BrowserListener>>();

/** Emit through Tauri in a native WebView or through a local browser bus. */
export async function emitAppEvent<T>(event: string, payload?: T): Promise<void> {
  if (hasTauriWindowMetadata()) {
    await emitTauriEvent(event, payload);
    return;
  }
  const envelope: AppEvent<T | undefined> = {
    event,
    id: -1,
    payload,
  };
  for (const listener of browserListeners.get(event) ?? []) {
    listener(envelope as AppEvent<unknown>);
  }
}

/** Listen without touching Tauri callback transforms in a regular browser. */
export async function listenAppEvent<T>(
  event: string,
  handler: (event: AppEvent<T>) => void,
): Promise<UnlistenFn> {
  if (hasTauriWindowMetadata()) {
    return listenTauriEvent<T>(event, handler);
  }
  const listener: BrowserListener = (envelope) =>
    handler(envelope as AppEvent<T>);
  const listeners = browserListeners.get(event) ?? new Set();
  listeners.add(listener);
  browserListeners.set(event, listeners);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) browserListeners.delete(event);
  };
}
