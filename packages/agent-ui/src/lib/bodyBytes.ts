/**
 * Pure sync body → byte array helpers for proxy fetch (A6.176).
 * Host still handles Blob / stream async paths.
 */

/** Encode a UTF-8 string to a number[] byte payload. */
export function utf8StringToBytes(body: string): number[] {
  return Array.from(new TextEncoder().encode(body));
}

/** Copy an ArrayBuffer into a number[] payload. */
export function arrayBufferToBytes(body: ArrayBuffer): number[] {
  return Array.from(new Uint8Array(body));
}

/** Copy an ArrayBufferView (TypedArray / DataView) into a number[] payload. */
export function arrayBufferViewToBytes(view: ArrayBufferView): number[] {
  return Array.from(
    new Uint8Array(view.buffer, view.byteOffset, view.byteLength),
  );
}
