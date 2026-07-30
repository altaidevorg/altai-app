/** Limits are deliberately byte based: protocol framing and host resources are
 * bytes, not JavaScript characters. Keep source material below the final
 * prompt limit so serialization has room for labels and boundaries. */
export const MAX_CONTEXT_ITEMS = 12;
export const MAX_CONTEXT_ITEM_BYTES = 48 * 1024;
export const MAX_CONTEXT_BYTES = 96 * 1024;
export const MAX_RUN_PROMPT_BYTES = 128 * 1024;
export const MAX_DIAGNOSTICS = 100;

export function utf8Bytes(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}
