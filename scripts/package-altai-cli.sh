#!/usr/bin/env bash
# Package a built altai-cli release binary into a platform archive + sha256.
#
# Usage:
#   scripts/package-altai-cli.sh \
#     --binary src-tauri/target/release/altai-cli \
#     --target aarch64-apple-darwin \
#     --version v0.6.4 \
#     --out-dir dist/cli
#
# Produces:
#   altai-cli_<version>_<target>.tar.gz   (Unix)
#   altai-cli_<version>_<target>.zip      (Windows targets)
#   matching .sha256 sidecar

set -euo pipefail

binary=""
target=""
version=""
out_dir="dist/cli"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary) binary="$2"; shift 2 ;;
    --target) target="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --out-dir) out_dir="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$binary" || -z "$target" || -z "$version" ]]; then
  echo "usage: $0 --binary PATH --target TRIPLE --version TAG [--out-dir DIR]" >&2
  exit 2
fi

if [[ ! -f "$binary" ]]; then
  echo "binary not found: $binary" >&2
  exit 1
fi

mkdir -p "$out_dir"
out_dir="$(cd "$out_dir" && pwd)"
stage="$(mktemp -d "${TMPDIR:-/tmp}/altai-cli-pkg.XXXXXX")"
cleanup() { rm -rf "$stage"; }
trap cleanup EXIT

bin_name="$(basename "$binary")"
if [[ "$bin_name" == *.exe ]]; then
  published_name="altai-cli.exe"
else
  published_name="altai-cli"
fi

cp "$binary" "$stage/$published_name"
chmod +x "$stage/$published_name" 2>/dev/null || true

cat >"$stage/README.txt" <<EOF
ALTAI CLI (${version})
Target: ${target}

Install:
  1. Place \`${published_name}\` somewhere on your PATH (for example ~/bin or /usr/local/bin).
  2. Run \`${published_name} doctor\` and \`${published_name} version --verbose\`.

Optional: symlink as \`altai\` if you prefer a shorter command name:
  ln -s ${published_name} altai

Docs: https://github.com/altaidevorg/altai-app/blob/main/INSTALL.md
EOF

stem="altai-cli_${version}_${target}"
if [[ "$target" == *windows* || "$published_name" == *.exe ]]; then
  archive="${out_dir}/${stem}.zip"
  if command -v zip >/dev/null 2>&1; then
    (cd "$stage" && zip -q "$archive" "$published_name" README.txt)
  else
    # Prefer Python zipfile: always present on GitHub runners and accepts
    # Unix-style paths from Git Bash / MSYS without cygpath conversion.
    py=python3
    if ! command -v "$py" >/dev/null 2>&1; then
      py=python
    fi
    "$py" - "$stage" "$published_name" "$archive" <<'PY'
import sys, zipfile
from pathlib import Path
stage, name, archive = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    zf.write(stage / name, arcname=name)
    zf.write(stage / "README.txt", arcname="README.txt")
print(f"wrote {archive}")
PY
  fi
else
  archive="${out_dir}/${stem}.tar.gz"
  tar -C "$stage" -czf "$archive" "$published_name" README.txt
fi

checksum="${archive}.sha256"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$out_dir" && sha256sum "$(basename "$archive")" >"$(basename "$checksum")")
elif command -v shasum >/dev/null 2>&1; then
  (cd "$out_dir" && shasum -a 256 "$(basename "$archive")" >"$(basename "$checksum")")
else
  echo "neither sha256sum nor shasum found" >&2
  exit 1
fi

echo "packed $archive"
echo "checksum $checksum"
