/**
 * Append a cache-bust query so hosts reload fresh webview assets after rebuild (A6.126).
 */
export function withAssetCacheBust(
  uriString: string,
  bust: string | number,
): string {
  const value = String(bust).trim();
  if (!value || !uriString) {
    return uriString;
  }
  const sep = uriString.includes("?") ? "&" : "?";
  return `${uriString}${sep}v=${encodeURIComponent(value)}`;
}
