/** Title-case a snake/kebab kind or state label for inbox cards. */
export function humanize(value: string): string {
  const normalized = value.trim().replace(/[_-]+/g, " ");
  return normalized
    ? normalized.charAt(0).toUpperCase() + normalized.slice(1)
    : "";
}

/** Compact relative timestamp for inbox rows. */
export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  const deltaMs = now - timestamp;
  if (!Number.isFinite(deltaMs) || deltaMs < 0) return "just now";
  const minutes = Math.floor(deltaMs / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(timestamp).toLocaleDateString();
}
