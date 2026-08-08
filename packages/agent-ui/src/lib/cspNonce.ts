/**
 * CSP nonce using cryptographically secure randomness (A6.128).
 */

import { getSecureRandomBytes } from "./secureRandom.js";

const ALPHABET =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

export function createNonce(length = 32): string {
  const bytes = getSecureRandomBytes(length);
  let nonce = "";
  for (let i = 0; i < length; i += 1) {
    const byte = bytes[i];
    if (byte === undefined) {
      throw new Error("Failed to generate secure nonce bytes");
    }
    nonce += ALPHABET.charAt(byte % ALPHABET.length);
  }
  return nonce;
}
