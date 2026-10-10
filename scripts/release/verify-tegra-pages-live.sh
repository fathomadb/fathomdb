#!/usr/bin/env bash
# Verify a completed Tegra Pages deployment from its public URLs.
#
# Every Pages deployment replaces the whole site, so a deployment is correct
# only if every artifact in the retained list, plus any artifact the run added,
# is served at its first-party https URL with its pinned SHA-256, and the
# generated index pages link each one with that digest
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 6). Pages
# propagation lags a deployment, so failed checks are retried until a bounded
# total wait runs out.
#
# Usage: verify-tegra-pages-live.sh --manifest RETAINED_JSON
#          [--added FILENAME=SHA256]... [--max-wait-seconds N] [--retry-interval-seconds N]
# Exit 0 when everything verifies, 1 when it does not, 2 on a usage error.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPER="$SCRIPT_DIR/tegra-pages-retained.py"
SITE_URL='https://fathomadb.github.io/fathomdb'
PACKAGES_URL="$SITE_URL/tegra/packages"
WHEEL_INDEX_URL="$SITE_URL/tegra/simple/fathomdb/"
NODE_INDEX_URL="$SITE_URL/tegra/node/"
MAX_WAIT_CEILING=1800

usage_error() {
  printf 'verify-tegra-pages-live: %s\n' "$1" >&2
  printf 'usage: %s --manifest RETAINED_JSON [--added FILENAME=SHA256]... [--max-wait-seconds N] [--retry-interval-seconds N]\n' \
    "$(basename "$0")" >&2
  exit 2
}

manifest=''
max_wait=600
interval=15
added=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest | --added | --max-wait-seconds | --retry-interval-seconds)
      [ "$#" -ge 2 ] || usage_error "$1 needs a value"
      case "$1" in
        --manifest) manifest="$2" ;;
        --added) added+=("$2") ;;
        --max-wait-seconds) max_wait="$2" ;;
        --retry-interval-seconds) interval="$2" ;;
        *) usage_error "unhandled argument $1" ;;
      esac
      shift 2
      ;;
    *) usage_error "unknown argument: $1" ;;
  esac
done
[ -n "$manifest" ] || usage_error '--manifest is required'
[[ "$max_wait" =~ ^[0-9]+$ ]] && [ "$max_wait" -le "$MAX_WAIT_CEILING" ] ||
  usage_error "--max-wait-seconds must be an integer from 0 to $MAX_WAIT_CEILING"
[[ "$interval" =~ ^[0-9]+$ ]] && [ "$interval" -ge 0 ] && [ "$interval" -le 300 ] ||
  usage_error '--retry-interval-seconds must be an integer from 0 to 300'

# Each expected artifact: filename, SHA-256 and https URL, from the validated
# retained list and from the run's own additions.
if ! listing="$(python3 "$HELPER" list --manifest "$manifest")"; then
  printf 'verify-tegra-pages-live: the retained list is invalid; every URL must be the https Pages location\n' >&2
  exit 1
fi
names=() digests=() urls=()
while IFS=$'\t' read -r _kind _version filename sha256 url; do
  [ -n "$filename" ] || continue
  names+=("$filename") digests+=("$sha256") urls+=("$url")
done <<<"$listing"
for entry in "${added[@]}"; do
  filename="${entry%%=*}"
  sha256="${entry#*=}"
  [ "$filename" != "$entry" ] || usage_error "--added must be FILENAME=SHA256: $entry"
  [[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || usage_error "--added $filename needs a 64-hex SHA-256"
  [[ "$filename" =~ ^fathomdb-[A-Za-z0-9_.+-]+\.(whl|tgz)$ ]] ||
    usage_error "--added filename must be a bare Tegra wheel or tarball name: $filename"
  names+=("$filename") digests+=("$sha256") urls+=("$PACKAGES_URL/$filename")
done
[ "${#names[@]}" -gt 0 ] || usage_error 'nothing to verify: the retained list is empty and nothing was added'
for url in "${urls[@]}"; do
  case "$url" in
    "$PACKAGES_URL"/*) ;;
    *) printf 'verify-tegra-pages-live: refusing a URL that is not the https Pages location: %s\n' "$url" >&2; exit 1 ;;
  esac
done

WORK="$(mktemp -d)"
trap 'rm -rf -- "$WORK"' EXIT

# Prints the HTTP status; the body lands in $2. Never follows to a non-https
# location.
fetch() {
  local url="$1" out="$2" code
  if ! code="$(curl --silent --show-error --location --proto '=https' --tlsv1.2 \
    --connect-timeout 20 --max-time 300 --output "$out" --write-out '%{http_code}' "$url")"; then
    code="${code:-000}"
  fi
  printf '%s\n' "$code"
}

attempt() {
  problems=()
  local index name code actual
  for index in "${!names[@]}"; do
    name="${names[$index]}"
    code="$(fetch "${urls[$index]}" "$WORK/artifact")"
    if [ "$code" != 200 ]; then
      problems+=("$name: HTTP $code from ${urls[$index]}")
      continue
    fi
    actual="$(sha256sum "$WORK/artifact" | awk '{print $1}')"
    if [ "$actual" != "${digests[$index]}" ]; then
      problems+=("$name: served SHA-256 $actual, expected ${digests[$index]}")
    fi
  done
  check_index "$WHEEL_INDEX_URL" '../../packages' whl
  check_index "$NODE_INDEX_URL" '../packages' tgz
}

check_index() {
  local index_url="$1" prefix="$2" suffix="$3" index code needed=0
  for index in "${!names[@]}"; do
    [[ "${names[$index]}" == *".$suffix" ]] && needed=1
  done
  [ "$needed" -eq 1 ] || return 0
  code="$(fetch "$index_url" "$WORK/index.html")"
  if [ "$code" != 200 ]; then
    problems+=("index $index_url: HTTP $code")
    return 0
  fi
  for index in "${!names[@]}"; do
    [[ "${names[$index]}" == *".$suffix" ]] || continue
    if ! grep -Fq -- "href=\"$prefix/${names[$index]}#sha256=${digests[$index]}\"" "$WORK/index.html"; then
      problems+=("index $index_url does not link ${names[$index]} with its SHA-256")
    fi
  done
}

deadline=$((SECONDS + max_wait))
while true; do
  attempt
  if [ "${#problems[@]}" -eq 0 ]; then
    printf 'verify-tegra-pages-live: %s artifact(s) served with their SHA-256 and linked from the index pages\n' \
      "${#names[@]}"
    exit 0
  fi
  if [ "$SECONDS" -ge "$deadline" ]; then
    break
  fi
  printf 'verify-tegra-pages-live: %s problem(s); retrying for Pages propagation\n' "${#problems[@]}" >&2
  sleep "$interval"
done
for problem in "${problems[@]}"; do
  printf 'verify-tegra-pages-live: %s\n' "$problem" >&2
done
printf 'verify-tegra-pages-live: FAIL after waiting up to %ss\n' "$max_wait" >&2
exit 1
