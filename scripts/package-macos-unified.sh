#!/usr/bin/env bash
# Build an unsigned macOS installer containing ALTAI Desktop and altai-cli.

set -euo pipefail

app=""
cli=""
version=""
arch=""
out_dir="dist/release"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app) app="$2"; shift 2 ;;
    --cli) cli="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --arch) arch="$2"; shift 2 ;;
    --out-dir) out_dir="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "$app" || -z "$cli" || -z "$version" || -z "$arch" ]]; then
  echo "usage: $0 --app PATH --cli PATH --version VERSION --arch ARCH [--out-dir DIR]" >&2
  exit 2
fi
if [[ ! -d "$app" ]]; then
  echo "app bundle not found: $app" >&2
  exit 1
fi
if [[ ! -f "$cli" ]]; then
  echo "CLI binary not found: $cli" >&2
  exit 1
fi

stage="$(mktemp -d "${TMPDIR:-/tmp}/altai-pkg.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/root/Applications" "$stage/root/usr/local/bin" "$out_dir"
cp -R "$app" "$stage/root/Applications/ALTAI.app"
mkdir -p "$stage/root/Applications/ALTAI.app/Contents/Resources/bin"
cp "$cli" "$stage/root/Applications/ALTAI.app/Contents/Resources/bin/altai-cli"
chmod 755 "$stage/root/Applications/ALTAI.app/Contents/Resources/bin/altai-cli"
ln -s "/Applications/ALTAI.app/Contents/Resources/bin/altai-cli" \
  "$stage/root/usr/local/bin/altai-cli"

# Adding the CLI changes the app bundle after Tauri's ad-hoc signature.
chmod -R u+w "$stage/root/Applications/ALTAI.app"
codesign --force --sign - \
  "$stage/root/Applications/ALTAI.app/Contents/Resources/bin/altai-cli"
codesign --force --deep --sign - "$stage/root/Applications/ALTAI.app"

asset_version="${version#v}"
package_version="${asset_version%%-*}"
output="$out_dir/ALTAI_${asset_version}_${arch}.pkg"
pkgbuild \
  --root "$stage/root" \
  --identifier dev.altai.app \
  --version "$package_version" \
  --install-location / \
  "$output"

echo "packed $output"
