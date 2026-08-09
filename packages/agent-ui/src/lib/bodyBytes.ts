/**
 * Pure sync body → byte array helpers for proxy fetch (A6.176+A6.183).
 * Host still awaits Blob.arrayBuffer(); package maps the result.
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

/** Copy a Uint8Array into a number[] payload (e.g. after Blob.arrayBuffer()). */
export function uint8ArrayToBytes(bytes: Uint8Array): number[] {
  return Array.from(bytes);
}
