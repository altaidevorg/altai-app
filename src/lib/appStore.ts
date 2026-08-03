import { hasTauriWindowMetadata } from "@/lib/tauriWindow";
import {
  LazyStore,
  type StoreOptions,
} from "@tauri-apps/plugin-store";

type ChangeListener = (key: string, value: unknown) => void;

const browserStores = new Map<string, Record<string, unknown>>();
const browserListeners = new Map<string, Set<ChangeListener>>();
const STORAGE_PREFIX = "altai.browser.store:";

function readBrowserStore(
  path: string,
  defaults: Record<string, unknown>,
): Record<string, unknown> {
  const cached = browserStores.get(path);
  if (cached) return cached;

  let data = { ...defaults };
  if (typeof window !== "undefined") {
    try {
      const raw = window.localStorage.getItem(`${STORAGE_PREFIX}${path}`);
      const parsed = raw ? (JSON.parse(raw) as unknown) : null;
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        data = { ...defaults, ...(parsed as Record<string, unknown>) };
      }
    } catch {
      // Private browsing and storage quotas may make localStorage unavailable.
    }
  }
  browserStores.set(path, data);
  return data;
}

function persistBrowserStore(path: string, data: Record<string, unknown>): void {
  browserStores.set(path, data);
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(`${STORAGE_PREFIX}${path}`, JSON.stringify(data));
  } catch {
    // Keep the in-memory copy usable when persistence is unavailable.
  }
}

function notifyBrowserStore(path: string, key: string, value: unknown): void {
  for (const listener of browserListeners.get(path) ?? []) {
    listener(key, value);
  }
}

/**
 * Minimal store facade used by ALTAI. Native WebViews delegate to
 * tauri-plugin-store; a regular Vite browser persists the same keys in
 * localStorage and falls back to memory when storage is unavailable.
 */
export class AppStore {
  private readonly nativeStore: LazyStore | null;
  private readonly defaults: Record<string, unknown>;

  constructor(
    private readonly path: string,
    options: StoreOptions = { defaults: {} },
  ) {
    this.defaults = options.defaults;
    this.nativeStore = hasTauriWindowMetadata()
      ? new LazyStore(path, options)
      : null;
  }

  async get<T>(key: string): Promise<T | undefined> {
    if (this.nativeStore) return this.nativeStore.get<T>(key);
    return readBrowserStore(this.path, this.defaults)[key] as T | undefined;
  }

  async set(key: string, value: unknown): Promise<void> {
    if (this.nativeStore) {
      await this.nativeStore.set(key, value);
      return;
    }
    const next = {
      ...readBrowserStore(this.path, this.defaults),
      [key]: value,
    };
    persistBrowserStore(this.path, next);
    notifyBrowserStore(this.path, key, value);
  }

  async delete(key: string): Promise<boolean> {
    if (this.nativeStore) return this.nativeStore.delete(key);
    const current = readBrowserStore(this.path, this.defaults);
    if (!Object.prototype.hasOwnProperty.call(current, key)) return false;
    const next = { ...current };
    delete next[key];
    persistBrowserStore(this.path, next);
    notifyBrowserStore(this.path, key, undefined);
    return true;
  }

  async entries<T = unknown>(): Promise<Array<[string, T]>> {
    if (this.nativeStore) return this.nativeStore.entries<T>();
    return Object.entries(readBrowserStore(this.path, this.defaults)) as Array<
      [string, T]
    >;
  }

  async save(): Promise<void> {
    if (this.nativeStore) await this.nativeStore.save();
  }

  async onChange<T>(
    callback: (key: string, value: T | undefined) => void,
  ): Promise<() => void> {
    if (this.nativeStore) return this.nativeStore.onChange<T>(callback);
    const listener: ChangeListener = (key, value) =>
      callback(key, value as T | undefined);
    const listeners = browserListeners.get(this.path) ?? new Set();
    listeners.add(listener);
    browserListeners.set(this.path, listeners);
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) browserListeners.delete(this.path);
    };
  }
}

export function createAppStore(
  path: string,
  options?: StoreOptions,
): AppStore {
  return new AppStore(path, options);
}
