/**
 * Pure storage I/O helpers for side panel widths (A6.135).
 * Hosts inject localStorage / sessionStorage / in-memory maps.
 */

import {
  parsePanelWidth,
  serializePanelWidth,
} from "./sidePanelWidth.js";

export type StringKeyStorage = {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
};

export function readPanelWidthFromStorage(
  storage: StringKeyStorage,
  key: string,
  fallback: number,
  min: number,
  max: number,
): number {
  try {
    return parsePanelWidth(storage.getItem(key), fallback, min, max);
  } catch {
    return parsePanelWidth(null, fallback, min, max);
  }
}

export function writePanelWidthToStorage(
  storage: StringKeyStorage,
  key: string,
  width: number,
  min: number,
  max: number,
): void {
  const serialized = serializePanelWidth(width, min, max);
  if (!serialized) {
    return;
  }
  try {
    storage.setItem(key, serialized);
  } catch {
    // Storage can be unavailable; layout still works without persistence.
  }
}
