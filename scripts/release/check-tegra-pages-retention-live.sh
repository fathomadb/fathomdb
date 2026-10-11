#!/usr/bin/env bash
# Refuse a Pages deployment that would silently withdraw a published Tegra
# artifact.
#
# Every Pages deployment replaces the whole site, and the site is rebuilt from
# the retained list on the deploying branch. An artifact that is live but not
# yet in that list would vanish. Before deploying, this reads the live Tegra
# index pages and fails if any artifact they link is missing from the retained
# list plus the run's own additions, or is linked there with another digest
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 6). Removing a
# live artifact needs a deliberate decision: name it with --withdraw, and the
# withdrawal is recorded in the run summary.
#
# An index page that answers 404 links nothing. That is the first deployment
# of its kind only while the retained list holds no artifact of that kind; the
# list records what has been published, so a 404 for a kind it holds means the
# live site is already damaged and the deployment stops for a person to look.
#
# Usage: check-tegra-pages-retention-live.sh --manifest RETAINED_JSON
#          [--added FILENAME=SHA256]... [--withdraw "NAME[,NAME ...]"]...
#          [--max-attempts N] [--retry-interval-seconds N]
# Writes a summary to $GITHUB_STEP_SUMMARY when that is set.
# Exit 0 when nothing live would be withdrawn unannounced, 1 otherwise, 2 on a
# usage error.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPER="$SCRIPT_DIR/tegra-pages-retained.py"
SITE_URL='https://fathomadb.github.io/fathomdb'
WHEEL_INDEX_URL="$SITE_URL/tegra/simple/fathomdb/"
NODE_INDEX_URL="$SITE_URL/tegra/node/"
MAX_ATTEMPTS_CEILING=20
NAME_PATTERN='^fathomdb-[A-Za-z0-9_.+-]+\.(whl|tgz)$'

usage_error() {
  printf 'check-tegra-pages-retention-live: %s\n' "$1" >&2
  printf 'usage: %s --manifest RETAINED_JSON [--added FILENAME=SHA256]... [--withdraw "NAME[,NAME ...]"]... [--max-attempts N] [--retry-interval-seconds N]\n' \
    "$(basename "$0")" >&2
  exit 2
}

manifest=''
max_attempts=5
interval=10
added=()
withdraw_args=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest | --added | --withdraw | --max-attempts | --retry-interval-seconds)
      [ "$#" -ge 2 ] || usage_error "$1 needs a value"
      case "$1" in
        --manifest) manifest="$2" ;;
        --added) added+=("$2") ;;
        --withdraw) withdraw_args+=("$2") ;;
        --max-attempts) max_attempts="$2" ;;
        --retry-interval-seconds) interval="$2" ;;
        *) usage_error "unhandled argument $1" ;;
      esac
      shift 2
      ;;
    *) usage_error "unknown argument: $1" ;;
  esac
done
[ -n "$manifest" ] || usage_error '--manifest is required'
[[ "$max_attempts" =~ ^[0-9]+$ ]] && [ "$max_attempts" -ge 1 ] && [ "$max_attempts" -le "$MAX_ATTEMPTS_CEILING" ] ||
  usage_error "--max-attempts must be an integer from 1 to $MAX_ATTEMPTS_CEILING"
[[ "$interval" =~ ^[0-9]+$ ]] && [ "$interval" -le 300 ] ||
  usage_error '--retry-interval-seconds must be an integer from 0 to 300'

# filename -> SHA-256 the deployment will serve.
declare -A kept=()
retained_wheels=0
retained_tarballs=0
if ! listing="$(python3 "$HELPER" list --manifest "$manifest")"; then
  printf 'check-tegra-pages-retention-live: the retained list is invalid\n' >&2
  exit 1
fi
while IFS=$'\t' read -r kind _version filename sha256 _url; do
  [ -n "$filename" ] || continue
  kept["$filename"]="$sha256"
  case "$kind" in
    wheel) retained_wheels=$((retained_wheels + 1)) ;;
    node-tarball) retained_tarballs=$((retained_tarballs + 1)) ;;
    *) printf 'check-tegra-pages-retention-live: unknown retained kind %s\n' "$kind" >&2; exit 1 ;;
  esac
done <<<"$listing"
for entry in "${added[@]}"; do
  filename="${entry%%=*}"
  sha256="${entry#*=}"
  [ "$filename" != "$entry" ] || usage_error "--added must be FILENAME=SHA256: $entry"
  [[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || usage_error "--added $filename needs a 64-hex SHA-256"
  [[ "$filename" =~ $NAME_PATTERN ]] || usage_error "--added filename must be a bare Tegra wheel or tarball name: $filename"
  kept["$filename"]="$sha256"
done

declare -A withdrawn=()
for value in "${withdraw_args[@]}"; do
  read -r -a names <<<"${value//,/ }"
  for name in "${names[@]}"; do
    [[ "$name" =~ $NAME_PATTERN ]] ||
      usage_error "--withdraw names must be bare Tegra wheel or tarball filenames: $name"
    if [ -n "${kept[$name]:-}" ]; then
      printf 'check-tegra-pages-retention-live: %s is retained or added by this run; it cannot also be withdrawn\n' "$name" >&2
      exit 1
    fi
    withdrawn["$name"]=1
  done
done

WORK="$(mktemp -d)"
trap 'rm -rf -- "$WORK"' EXIT

summary() {
  [ -z "${GITHUB_STEP_SUMMARY:-}" ] || printf '%s\n' "$@" >>"$GITHUB_STEP_SUMMARY"
}

# Prints the final HTTP status; the body lands in $2. Connection failures
# (000), throttling and server errors are retried up to the attempt bound;
# any other answer, including 404, is final.
fetch() {
  local url="$1" out="$2" code attempt
  for ((attempt = 1; attempt <= max_attempts; attempt++)); do
    if ! code="$(curl --silent --show-error --location --proto '=https' --tlsv1.2 \
      --connect-timeout 20 --max-time 120 --output "$out" --write-out '%{http_code}' "$url")"; then
      code="${code:-000}"
    fi
    case "$code" in
      000 | 429 | 5??) ;;
      *) break ;;
    esac
    if [ "$attempt" -lt "$max_attempts" ]; then
      printf 'check-tegra-pages-retention-live: HTTP %s from %s; retrying\n' "$code" "$url" >&2
      sleep "$interval"
    fi
  done
  printf '%s\n' "$code"
}

problems=()
declare -A live=()
# check_index URL HREF_PREFIX SUFFIX RETAINED_COUNT LABEL
check_index() {
  local url="$1" prefix="$2" suffix="$3" retained_count="$4" label="$5" code href rest name digest
  code="$(fetch "$url" "$WORK/index.html")"
  if [ "$code" = 404 ] && [ "$retained_count" -eq 0 ]; then
    printf 'check-tegra-pages-retention-live: %s index %s is absent (HTTP 404); %s artifacts are not published yet\n' \
      "$label" "$url" "$label"
    summary "- The live $label index answered HTTP 404 and the retained list holds no $label: $label artifacts are not published yet, so this deployment withdraws none."
    return 0
  fi
  if [ "$code" != 200 ]; then
    problems+=("cannot read the live $label index $url: HTTP $code")
    return 0
  fi
  while IFS= read -r href; do
    href="${href#href=\"}"
    href="${href%\"}"
    rest="${href#"$prefix"/}"
    name="${rest%%#sha256=*}"
    digest="${rest#*#sha256=}"
    if [ "$rest" = "$href" ] || [ "$name" = "$rest" ] || ! [[ "$name" =~ $NAME_PATTERN ]] ||
      [[ "$name" != *".$suffix" ]] || ! [[ "$digest" =~ ^[0-9a-f]{64}$ ]]; then
      problems+=("unrecognised link in the live $label index $url: $href")
      continue
    fi
    live["$name"]="$digest"
  done < <(grep -o 'href="[^"]*"' "$WORK/index.html" || true)
}
check_index "$WHEEL_INDEX_URL" '../../packages' whl "$retained_wheels" wheel
check_index "$NODE_INDEX_URL" '../packages' tgz "$retained_tarballs" 'Node tarball'

withdrawing=()
for name in "${!live[@]}"; do
  if [ -n "${kept[$name]:-}" ]; then
    if [ "${kept[$name]}" != "${live[$name]}" ]; then
      problems+=("$name is live with SHA-256 ${live[$name]}; this deployment would replace it with ${kept[$name]}")
    fi
  elif [ -n "${withdrawn[$name]:-}" ]; then
    withdrawing+=("$name")
  else
    problems+=("$name is live but missing from the retained list; this deployment would withdraw it")
  fi
done
for name in "${!withdrawn[@]}"; do
  [ -n "${live[$name]:-}" ] || problems+=("--withdraw $name: not linked from the live Tegra index pages")
done

if [ "${#problems[@]}" -gt 0 ]; then
  for problem in "${problems[@]}"; do
    printf 'check-tegra-pages-retention-live: %s\n' "$problem" >&2
  done
  printf 'check-tegra-pages-retention-live: FAIL; add every live artifact to scripts/release/tegra-pages-retained.json, or name a deliberate withdrawal with withdraw_tegra_artifacts\n' >&2
  summary "- Deployment refused: ${#problems[@]} live Tegra artifact problem(s); see the job log."
  exit 1
fi
for name in "${withdrawing[@]}"; do
  printf 'check-tegra-pages-retention-live: deliberately withdrawing %s\n' "$name"
  summary "- Deliberately withdrawn by this deployment: \`$name\`"
done
printf 'check-tegra-pages-retention-live: %s live artifact(s); none withdrawn unannounced\n' "${#live[@]}"
summary "- Live Tegra artifacts checked: ${#live[@]}; deliberately withdrawn: ${#withdrawing[@]}."
