#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
installer="$root/install.sh"

fail() { printf '%s\n' "Assertion failed: $1" >&2; exit 1; }
contains() { case "$1" in *"$2"*) ;; *) fail "$3" ;; esac; }
not_contains() { case "$1" in *"$2"*) fail "$3" ;; *) ;; esac; }

[ -f "$installer" ] || fail "Missing installer: $installer"
sh -n "$installer"

plan() {
  LLM_WIKIS_PLAN_OS=$1 LLM_WIKIS_PLAN_ARCH=$2 LLM_WIKIS_VERSION=${3-} sh "$installer" --print-plan
}

linux=$(plan Linux x86_64 '')
contains "$linux" 'releases/latest/download/llm-wikis-linux-amd64' 'Linux latest URL'
contains "$linux" '.local/bin/llm-wikis' 'Linux install path'
contains "$linux" '.profile' 'Linux profile path'

linux_alias=$(plan Linux amd64 '')
contains "$linux_alias" 'llm-wikis-linux-amd64' 'Linux amd64 alias'

darwin=$(plan Darwin arm64 v0.1.0)
contains "$darwin" 'releases/download/v0.1.0/llm-wikis-darwin-arm64' 'Darwin pinned URL'
contains "$darwin" '.zprofile' 'Darwin profile path'

darwin_alias=$(plan Darwin aarch64 '')
contains "$darwin_alias" 'llm-wikis-darwin-arm64' 'Darwin aarch64 alias'

pinned=$(plan Linux x86_64 0.1.0)
contains "$pinned" 'releases/download/v0.1.0/llm-wikis-linux-amd64' 'normalized pinned URL'

fixture_directory=$(mktemp -d "${TMPDIR:-/tmp}/llm-wikis-installer-parser.XXXXXX")
cleanup_fixture() { rm -rf "$fixture_directory"; }
trap cleanup_fixture EXIT HUP INT TERM
asset='llm-wikis-linux-amd64'
hash='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
printf '%s  %s\n' "$hash" "$asset" > "$fixture_directory/valid.txt"
parsed=$(sh "$installer" --parse-checksum "$asset" "$fixture_directory/valid.txt")
contains "$parsed" "sha256=$hash" 'parser accepts exactly one valid entry'

printf '%s  %s\n' 'not-a-hash' "$asset" > "$fixture_directory/malformed.txt"
if sh "$installer" --parse-checksum "$asset" "$fixture_directory/malformed.txt" >/dev/null 2>&1; then fail 'parser accepted malformed matching entry'; fi

printf '%s  %s\n%s  %s\n' "$hash" "$asset" "$hash" "$asset" > "$fixture_directory/duplicate.txt"
if sh "$installer" --parse-checksum "$asset" "$fixture_directory/duplicate.txt" >/dev/null 2>&1; then fail 'parser accepted duplicate matching entries'; fi

for mapping in 'Darwin:x86_64' 'Linux:armv7l' 'FreeBSD:x86_64'; do
  os=${mapping%%:*}; arch=${mapping#*:}
  if plan "$os" "$arch" '' >/dev/null 2>&1; then fail "unsupported mapping $mapping was accepted"; fi
done

source=$(cat "$installer")
not_contains "$source" 'fake' 'fake origin'
not_contains "$source" 'test-origin' 'test origin'
not_contains "$source" "http"'.server' 'local HTTP server'
not_contains "$source" 'exec "$' 'downloaded executable invocation'
contains "$source" 'set -eu' 'strict shell mode'
contains "$source" 'sha256sum' 'checksum verification'
contains "$source" 'madao02515634/llm-wikis' 'fixed repository'
contains "$source" 'mktemp -d' 'same-directory temporary directory'
contains "$source" 'chmod 700 "$temporary_asset"' 'downloaded binary receives safe executable permissions before move'
contains "$source" 'rm -rf "$temporary_directory"' 'temporary directory cleanup'
contains "$source" 'case ":${PATH-}:"' 'current PATH is checked before profile mutation'
[ "$(printf '%s' "$source" | grep -o 'LLM-WIKIS-PATH' | wc -l | tr -d ' ')" = 1 ] || fail 'one managed profile marker'

printf '%s\n' 'POSIX installer contract checks passed.'
