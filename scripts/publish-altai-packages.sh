#!/usr/bin/env bash
# Publish shared @altai packages in dependency order (requires NPM_TOKEN / npm login).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY="${1:-}"
order=(host-contract agent-protocol agent-ui)
pack_dir="$(mktemp -d "${TMPDIR:-/tmp}/altai-packages.XXXXXX")"
trap 'rm -rf "$pack_dir"' EXIT
for name in "${order[@]}"; do
  echo "==> packing $name"
  (cd "$ROOT" && pnpm --filter "@altai/$name" pack --pack-destination "$pack_dir")
  tarball="$(find "$pack_dir" -maxdepth 1 -name "altai-$name-*.tgz" -print -quit)"
  if [[ -z "$tarball" ]]; then
    echo "Could not find packed tarball for @altai/$name" >&2
    exit 1
  fi
  if [[ "$DRY" == "--dry-run" ]]; then
    continue
  fi
  if [[ "${NPM_PUBLISH:-}" != "1" ]]; then
    echo "Set NPM_PUBLISH=1 to actually publish (after setting private:false)."
    exit 0
  fi
  publish_args=(--access public)
  if [[ -n "${NPM_TAG:-}" && "${NPM_TAG}" != "latest" ]]; then
    publish_args+=(--tag "$NPM_TAG")
  fi
  npm publish "$tarball" "${publish_args[@]}"
done
