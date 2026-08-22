#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ]; then
  printf 'usage: %s <predicate-quantifier> <filters-yaml> <output-file> <changed-path>...\n' "$0" >&2
  exit 64
fi

quantifier="$1"
filters_path="$2"
output_path="$3"
shift 3

case "$quantifier" in
  some|every) ;;
  *)
    printf 'run-paths-filter-action: invalid predicate quantifier: %s\n' "$quantifier" >&2
    exit 64
    ;;
esac

if [ ! -f "$filters_path" ]; then
  printf 'run-paths-filter-action: filters file does not exist: %s\n' "$filters_path" >&2
  exit 66
fi

case "$output_path" in
  /*) ;;
  *)
    printf 'run-paths-filter-action: output path must be absolute: %s\n' "$output_path" >&2
    exit 64
    ;;
esac
mkdir -p "$(dirname "$output_path")"
: >"$output_path"

repo_root="$(git rev-parse --show-toplevel)"
action_entrypoint="$repo_root/scripts/tests/fixtures/dorny-paths-filter-v4/dist/index.js"
fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-paths-filter.XXXXXX")"
trap 'rm -rf "$fixture_root"' EXIT

fixture_repo="$fixture_root/repository"
mkdir -p "$fixture_repo"
git -C "$fixture_repo" init --quiet --initial-branch=main
git -C "$fixture_repo" config user.email paths-filter-fixture@example.invalid
git -C "$fixture_repo" config user.name 'Paths Filter Fixture'
printf 'base\n' >"$fixture_repo/.fixture-base"
git -C "$fixture_repo" add .fixture-base
git -C "$fixture_repo" commit --quiet -m base
base_sha="$(git -C "$fixture_repo" rev-parse HEAD)"

for changed_path in "$@"; do
  case "/$changed_path/" in
    //*|*/../*|*/./*)
      printf 'run-paths-filter-action: changed path must be repository-relative: %s\n' "$changed_path" >&2
      exit 64
      ;;
  esac
  mkdir -p "$fixture_repo/$(dirname "$changed_path")"
  printf 'changed: %s\n' "$changed_path" >"$fixture_repo/$changed_path"
done

git -C "$fixture_repo" add --all
git -C "$fixture_repo" commit --quiet -m changed
head_sha="$(git -C "$fixture_repo" rev-parse HEAD)"

event_path="$fixture_root/event.json"
printf '{"ref":"refs/heads/main","before":"%s","after":"%s","repository":{"default_branch":"main"}}\n' \
  "$base_sha" "$head_sha" >"$event_path"

filters="$(<"$filters_path")"
(
  cd "$fixture_repo"
  env \
    CI=true \
    GITHUB_ACTIONS=true \
    GITHUB_EVENT_NAME=push \
    GITHUB_EVENT_PATH="$event_path" \
    GITHUB_OUTPUT="$output_path" \
    GITHUB_REF=refs/heads/main \
    GITHUB_REPOSITORY=fathomadb/fathomdb \
    GITHUB_SHA="$head_sha" \
    GITHUB_WORKSPACE="$fixture_repo" \
    RUNNER_TEMP="$fixture_root" \
    "INPUT_FILTERS=$filters" \
    'INPUT_INITIAL-FETCH-DEPTH=10' \
    'INPUT_LIST-FILES=json' \
    "INPUT_PREDICATE-QUANTIFIER=$quantifier" \
    node "$action_entrypoint"
)
