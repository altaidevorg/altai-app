/**
 * Pure native/transcript message id generator (A6.164).
 * Used by background and host-mirrored transcript append paths.
 */

export function newNativeMessageId(
  now: () => number = Date.now,
  random: () => number = Math.random,
): string {
  return `native-${now()}-${random().toString(36).slice(2, 8)}`;
}
