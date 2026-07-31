#!/bin/sh
set -eu

repository='madao02515634/llm-wikis'
asset=''
install_dir="$HOME/.local/bin"
install_path="$install_dir/llm-wikis"
path_marker='# LLM-WIKIS-PATH'

select_manifest_checksum() {
  selected_asset=$1
  manifest=$2
  case "$selected_asset" in
    llm-wikis-linux-amd64|llm-wikis-darwin-arm64) ;;
    *) printf '%s\n' "Unsupported release asset: $selected_asset" >&2; return 1 ;;
  esac
  matches=$(grep -E "^[[:xdigit:]]{64}[ *]+$selected_asset$" "$manifest" || true)
  [ "$(printf '%s\n' "$matches" | sed '/^$/d' | wc -l | tr -d ' ')" = 1 ] || {
    printf '%s\n' "SHA256SUMS must contain exactly one checksum for $selected_asset." >&2
    return 1
  }
  printf '%s\n' "$matches" | awk '{print tolower($1)}'
}

mode=install
case "${1-}" in
  --print-plan) [ "$#" = 1 ] || { printf '%s\n' 'Usage: install.sh [--print-plan|--parse-checksum ASSET MANIFEST]' >&2; exit 2; }; mode=plan ;;
  --parse-checksum)
    [ "$#" = 3 ] || { printf '%s\n' 'Usage: install.sh [--print-plan|--parse-checksum ASSET MANIFEST]' >&2; exit 2; }
    mode=parse
    parser_asset=$2
    parser_manifest=$3
    ;;
  '') ;;
  *) printf '%s\n' 'Usage: install.sh [--print-plan|--parse-checksum ASSET MANIFEST]' >&2; exit 2 ;;
esac

if [ "$mode" = parse ]; then
  parser_checksum=$(select_manifest_checksum "$parser_asset" "$parser_manifest") || exit 1
  printf '%s\n' "sha256=$parser_checksum"
  exit 0
fi

if [ "$mode" = plan ]; then
  os=${LLM_WIKIS_PLAN_OS-$(uname -s)}
  arch=${LLM_WIKIS_PLAN_ARCH-$(uname -m)}
else
  os=$(uname -s)
  arch=$(uname -m)
fi

case "$os:$arch" in
  Linux:x86_64|Linux:amd64) asset='llm-wikis-linux-amd64'; profile="$HOME/.profile" ;;
  Darwin:arm64|Darwin:aarch64) asset='llm-wikis-darwin-arm64'; profile="$HOME/.zprofile" ;;
  *) printf '%s\n' "Unsupported platform: $os $arch. Supported: Linux x64 and Darwin ARM64." >&2; exit 1 ;;
esac

version=${LLM_WIKIS_VERSION-}
if [ -n "$version" ]; then
  version=${version#v}
  release_base="https://github.com/$repository/releases/download/v$version"
else
  release_base="https://github.com/$repository/releases/latest/download"
fi
asset_url="$release_base/$asset"
manifest_url="$release_base/SHA256SUMS"

if [ "$mode" = plan ]; then
  printf '%s\n' \
    "asset=$asset" \
    "asset_url=$asset_url" \
    "manifest_url=$manifest_url" \
    "install_path=$install_path" \
    "profile=$profile"
  exit 0
fi

mkdir -p "$install_dir"
temporary_directory=$(mktemp -d "$install_dir/.llm-wikis.XXXXXX")
cleanup() { rm -rf "$temporary_directory"; }
trap cleanup EXIT HUP INT TERM
temporary_asset="$temporary_directory/$asset"
temporary_manifest="$temporary_directory/SHA256SUMS"

curl -fsSL "$asset_url" -o "$temporary_asset"
curl -fsSL "$manifest_url" -o "$temporary_manifest"
expected=$(select_manifest_checksum "$asset" "$temporary_manifest")
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$temporary_asset" | awk '{print tolower($1)}')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$temporary_asset" | awk '{print tolower($1)}')
else
  printf '%s\n' 'No SHA-256 tool is available.' >&2
  exit 1
fi
[ "$expected" = "$actual" ] || { printf '%s\n' "Checksum verification failed for $asset." >&2; exit 1; }

chmod 700 "$temporary_asset"
mv -f "$temporary_asset" "$install_path"
case ":${PATH-}:" in
  *":$install_dir:"*) ;;
  *)
    if ! grep -F "$path_marker" "$profile" >/dev/null 2>&1; then
      printf '%s\n' "$path_marker" 'export PATH="$HOME/.local/bin:$PATH"' >> "$profile"
    fi
    ;;
esac

printf '%s\n' "Installed $install_path"
printf '%s\n' 'Run llm-wikis --version manually to verify the installation.'
