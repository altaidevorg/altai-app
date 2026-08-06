#!/usr/bin/env bash
# Publish @altai/host-contract then @altai/agent-ui from packages/ (requires NPM_TOKEN / npm login).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY="${1:-}"
order=(host-contract agent-ui)
for name in "${order[@]}"; do
  dir="$ROOT/packages/$name"
  echo "==> packing $name"
  (cd "$dir" && npm pack --dry-run)
  if [[ "$DRY" == "--dry-run" ]]; then
    continue
  fi
  if [[ "${NPM_PUBLISH:-}" != "1" ]]; then
    echo "Set NPM_PUBLISH=1 to actually publish (after setting private:false)."
    exit 0
  fi
  (cd "$dir" && npm publish --access public)
done
