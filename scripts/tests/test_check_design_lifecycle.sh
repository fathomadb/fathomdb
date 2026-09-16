#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
CHECKER=${CHECKER_UNDER_TEST:-$REPO_ROOT/scripts/check-design-lifecycle.py}

if [[ ! -f "$CHECKER" ]]; then
  echo "FAIL: missing design-lifecycle checker: $CHECKER" >&2
  exit 1
fi

TMP_ROOT=$(mktemp -d)
trap 'rm -rf "$TMP_ROOT"' EXIT

write_fixture() {
  local root=$1
  local mode=$2
  mkdir -p "$root/dev/design" "$root/scripts" "$root/.github/workflows"
  printf '# Alpha\n' >"$root/dev/design/alpha.md"
  printf '# Legacy\n' >"$root/dev/design/legacy.md"
  printf '%s\n' \
    'run_capped check-design-lifecycle "$SCRIPT_DIR/check-design-lifecycle.py"' \
    >"$root/scripts/agent-lint-md.sh"
  printf '%s\n' \
    '      - name: design document lifecycle' \
    '        run: python3 scripts/check-design-lifecycle.py' \
    >"$root/.github/workflows/ci.yml"

  case "$mode" in
    valid)
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"maintained","topic":"alpha","role":"design","owner":"dev/design/alpha.md","release":"cross-release","successor":null},
  {"path":"dev/design/legacy.md","class":"superseded","topic":"legacy","role":"design","owner":"dev/design/alpha.md","release":"historical:0.8.0","successor":"dev/design/alpha.md"}
]}
JSON
      ;;
    missing)
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"maintained","topic":"alpha","role":"design","owner":"dev/design/alpha.md","release":"cross-release","successor":null}
]}
JSON
      ;;
    extra)
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"maintained","topic":"alpha","role":"design","owner":"dev/design/alpha.md","release":"cross-release","successor":null},
  {"path":"dev/design/legacy.md","class":"historical","topic":"legacy","role":"design","owner":"dev/design/legacy.md","release":"historical:0.8.0","successor":null},
  {"path":"dev/design/ghost.md","class":"reference","topic":"ghost","role":"analysis","owner":"dev/design/ghost.md","release":"cross-release","successor":null}
]}
JSON
      ;;
    duplicate_path)
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"maintained","topic":"alpha","role":"design","owner":"dev/design/alpha.md","release":"cross-release","successor":null},
  {"path":"dev/design/alpha.md","class":"reference","topic":"alpha-copy","role":"analysis","owner":"dev/design/alpha.md","release":"cross-release","successor":null},
  {"path":"dev/design/legacy.md","class":"historical","topic":"legacy","role":"design","owner":"dev/design/legacy.md","release":"historical:0.8.0","successor":null}
]}
JSON
      ;;
    invalid_class)
      write_fixture "$root" valid
      sed -i 's/"class":"maintained"/"class":"current"/' "$root/dev/design/document-lifecycle.json"
      ;;
    missing_field)
      write_fixture "$root" valid
      sed -i 's/,"release":"cross-release"//' "$root/dev/design/document-lifecycle.json"
      ;;
    invalid_release)
      write_fixture "$root" valid
      sed -i 's/"release":"cross-release"/"release":"soon"/' "$root/dev/design/document-lifecycle.json"
      ;;
    invalid_topic)
      write_fixture "$root" valid
      sed -i 's/"topic":"alpha"/"topic":"Alpha design"/' "$root/dev/design/document-lifecycle.json"
      ;;
    missing_owner)
      write_fixture "$root" valid
      sed -i 's#"owner":"dev/design/alpha.md"#"owner":"dev/design/missing.md"#' "$root/dev/design/document-lifecycle.json"
      ;;
    missing_successor)
      write_fixture "$root" valid
      sed -i 's#"successor":"dev/design/alpha.md"#"successor":"dev/design/missing.md"#' "$root/dev/design/document-lifecycle.json"
      ;;
    superseded_without_successor)
      write_fixture "$root" valid
      sed -i 's#"successor":"dev/design/alpha.md"#"successor":null#' "$root/dev/design/document-lifecycle.json"
      ;;
    duplicate_owner)
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"maintained","topic":"shared","role":"design","owner":"dev/design/alpha.md","release":"cross-release","successor":null},
  {"path":"dev/design/legacy.md","class":"maintained","topic":"shared","role":"design","owner":"dev/design/legacy.md","release":"cross-release","successor":null}
]}
JSON
      ;;
    missing_local_wiring)
      write_fixture "$root" valid
      : >"$root/scripts/agent-lint-md.sh"
      ;;
    missing_ci_wiring)
      write_fixture "$root" valid
      : >"$root/.github/workflows/ci.yml"
      ;;
    *)
      echo "unknown fixture mode: $mode" >&2
      exit 1
      ;;
  esac
}

run_ok() {
  local mode=$1
  local root="$TMP_ROOT/$mode"
  write_fixture "$root" "$mode"
  python3 "$CHECKER" --repo-root "$root" >/dev/null
}

run_fail() {
  local mode=$1
  local root="$TMP_ROOT/$mode"
  write_fixture "$root" "$mode"
  if python3 "$CHECKER" --repo-root "$root" >/dev/null 2>&1; then
    echo "FAIL: checker accepted $mode fixture" >&2
    exit 1
  fi
}

run_ok valid
for mode in \
  missing \
  extra \
  duplicate_path \
  invalid_class \
  missing_field \
  invalid_release \
  invalid_topic \
  missing_owner \
  missing_successor \
  superseded_without_successor \
  duplicate_owner \
  missing_local_wiring \
  missing_ci_wiring
do
  run_fail "$mode"
done

echo "All design-lifecycle tests passed"
