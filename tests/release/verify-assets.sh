#!/bin/sh
set -eu

expected_files='llm-wikis-windows-amd64.exe llm-wikis-linux-amd64 llm-wikis-darwin-arm64 SHA256SUMS'
checked_files='llm-wikis-windows-amd64.exe llm-wikis-linux-amd64 llm-wikis-darwin-arm64'

fail() { printf '%s\n' "Verification failed: $1" >&2; return 1; }
hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print tolower($1)}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print tolower($1)}'
  else fail 'no SHA-256 tool is available'; fi
}
has_name() { case " $checked_files " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }

verify_bundle() {
  directory=$1
  [ -d "$directory" ] || { fail 'staging path is a directory'; return 1; }
  actual=$(for file in "$directory"/*; do [ -f "$file" ] && basename "$file"; done | LC_ALL=C sort)
  expected=$(printf '%s\n' $expected_files | LC_ALL=C sort)
  [ "$(printf '%s\n' "$actual" | sed '/^$/d' | wc -l | tr -d ' ')" = 4 ] || { fail 'bundle contains exactly four files'; return 1; }
  [ "$actual" = "$expected" ] || { fail 'bundle file set is exact'; return 1; }

  manifest="$directory/SHA256SUMS"
  entries=$(sed '/^$/d' "$manifest")
  [ "$(printf '%s\n' "$entries" | wc -l | tr -d ' ')" = 3 ] || { fail 'SHA256SUMS has exactly three entries'; return 1; }
  names=''
  while IFS= read -r line || [ -n "$line" ]; do
    hash=$(printf '%s\n' "$line" | awk '{print $1}')
    name=$(printf '%s\n' "$line" | awk '{print $2}')
    [ "$(printf '%s\n' "$line" | awk 'NF == 2 { print $1 "  " $2 }')" = "$line" ] || { fail "manifest entry is valid: $line"; return 1; }
    case "$hash" in *[!0123456789abcdefABCDEF]*|'') fail "manifest entry is valid: $line"; return 1 ;; esac
    [ "${#hash}" = 64 ] || { fail "manifest entry is valid: $line"; return 1; }
    has_name "$name" || { fail "manifest entry names a release asset: $name"; return 1; }
    case " $names " in *" $name "*) fail "manifest entry is unique: $name"; return 1 ;; esac
    names="$names $name"
    [ "$(hash_file "$directory/$name")" = "$(printf '%s' "$hash" | tr '[:upper:]' '[:lower:]')" ] || { fail "checksum matches: $name"; return 1; }
  done <<EOF
$entries
EOF
  for name in $checked_files; do case " $names " in *" $name "*) ;; *) fail "every non-manifest asset is checksummed"; return 1 ;; esac; done
}

new_text_bundle() {
  directory=$1
  mkdir -p "$directory"
  for name in $checked_files; do printf '%s\n' "inert $name" > "$directory/$name"; done
  : > "$directory/SHA256SUMS"
  for name in $checked_files; do printf '%s  %s\n' "$(hash_file "$directory/$name")" "$name" >> "$directory/SHA256SUMS"; done
}

self_test() {
  root_dir=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
  workflow="$root_dir/.github/workflows/release.yml"
  [ -f "$workflow" ] || fail 'release workflow exists'
  workflow_source=$(cat "$workflow")
  case "$workflow_source" in *'gh release view'*) fail 'publish does not query release metadata after bundle download' ;; esac
  case "$workflow_source" in *'gh release create "$GITHUB_REF_NAME" --verify-tag'*) ;; *) fail 'release creation verifies the tag' ;; esac
  case "$workflow_source" in *'GH_TOKEN: ${{ github.token }}'*) ;; *) fail 'release creation receives the GitHub token' ;; esac

  root=$(mktemp -d "${TMPDIR:-/tmp}/llm-wikis-verifier.XXXXXX")
  trap 'rm -rf "$root"' EXIT HUP INT TERM
  new_text_bundle "$root"; verify_bundle "$root"
  rm "$root/llm-wikis-darwin-arm64"; if verify_bundle "$root" >/dev/null 2>&1; then fail 'missing file was accepted'; fi
  rm -rf "$root"; mkdir -p "$root"; new_text_bundle "$root"
  printf '%s\n' inert > "$root/unexpected.txt"; if verify_bundle "$root" >/dev/null 2>&1; then fail 'unexpected file was accepted'; fi
  rm "$root/unexpected.txt"
  head -n 1 "$root/SHA256SUMS" >> "$root/SHA256SUMS"; if verify_bundle "$root" >/dev/null 2>&1; then fail 'duplicate entry was accepted'; fi
  rm -rf "$root"; mkdir -p "$root"; new_text_bundle "$root"
  printf '%s\n' 'not a checksum' > "$root/SHA256SUMS"; if verify_bundle "$root" >/dev/null 2>&1; then fail 'malformed entry was accepted'; fi
  rm -rf "$root"; mkdir -p "$root"; new_text_bundle "$root"
  printf '%s\n' changed >> "$root/llm-wikis-linux-amd64"; if verify_bundle "$root" >/dev/null 2>&1; then fail 'mismatch was accepted'; fi
  printf '%s\n' 'POSIX release bundle verifier self-test passed.'
}

case "${1-}" in
  --self-test) [ "$#" = 1 ] || { printf '%s\n' 'Usage: verify-assets.sh [--self-test|STAGING_DIRECTORY]' >&2; exit 2; }; self_test ;;
  '') printf '%s\n' 'Usage: verify-assets.sh [--self-test|STAGING_DIRECTORY]' >&2; exit 2 ;;
  *) [ "$#" = 1 ] || { printf '%s\n' 'Usage: verify-assets.sh [--self-test|STAGING_DIRECTORY]' >&2; exit 2; }; verify_bundle "$1"; printf '%s\n' 'Release bundle verified.' ;;
esac
