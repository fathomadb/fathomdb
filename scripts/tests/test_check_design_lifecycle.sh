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
  mkdir -p "$root/dev/design" "$root/dev/adr" "$root/dev/interfaces" \
    "$root/src" "$root/scripts" "$root/.github/workflows"
  printf '# Alpha\n' >"$root/dev/design/alpha.md"
  printf '# Legacy\n' >"$root/dev/design/legacy.md"
  printf '%s\n' '---' 'status: accepted' '---' '# Current authority' \
    >"$root/dev/adr/current.md"
  printf '%s\n' '---' 'status: locked' '---' '# Locked interface' \
    >"$root/dev/interfaces/current.md"
  printf 'fn current_witness() {}\n' >"$root/src/current.rs"
  printf '# Agent invariants\n' >"$root/AGENTS.md"
  printf '%s\n' \
    'run_capped check-design-lifecycle "$SCRIPT_DIR/check-design-lifecycle.py"' \
    >"$root/scripts/agent-lint-md.sh"
  printf '%s\n' \
    'jobs:' \
    '  markdownlint:' \
    "    if: needs.changes.outputs.docs_only == 'true'" \
    '    steps:' \
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
      cat >"$root/dev/design/current-owner-authority.json" <<'JSON'
{"schema_version":1,"profiles":[
  {"path":"dev/design/alpha.md","profile":"current","semantic_authority":["dev/adr/current.md"],"implementation_witness":["src/current.rs"],"evidence_only":[]}
]}
JSON
      ;;
    valid_external_successor)
      write_fixture "$root" valid
      mkdir -p "$root/dev/adr"
      printf '%s\n' '---' 'status: accepted' '---' '# Current authority' \
        >"$root/dev/adr/current.md"
      sed -i \
        's#"owner":"dev/design/alpha.md","release":"historical:0.8.0","successor":"dev/design/alpha.md"#"owner":"dev/adr/current.md","release":"historical:0.8.0","successor":"dev/adr/current.md"#' \
        "$root/dev/design/document-lifecycle.json"
      ;;
    valid_locked_interface)
      write_fixture "$root" valid
      sed -i 's#dev/adr/current.md#dev/interfaces/current.md#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    valid_agents_method)
      write_fixture "$root" valid
      sed -i 's/"role":"design"/"role":"method"/' \
        "$root/dev/design/document-lifecycle.json"
      sed -i 's#dev/adr/current.md#AGENTS.md#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    valid_historical_evidence)
      write_fixture "$root" valid
      mkdir -p "$root/dev/plans"
      printf '# Historical evidence\n' >"$root/dev/plans/historical.md"
      sed -i 's#"evidence_only":\[\]#"evidence_only":["dev/plans/historical.md"]#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    untracked_draft)
      write_fixture "$root" valid
      printf '# Local draft\n' >"$root/dev/design/local-draft.md"
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
    self_successor)
      write_fixture "$root" valid
      sed -i \
        's#"owner":"dev/design/alpha.md","release":"historical:0.8.0","successor":"dev/design/alpha.md"#"owner":"dev/design/legacy.md","release":"historical:0.8.0","successor":"dev/design/legacy.md"#' \
        "$root/dev/design/document-lifecycle.json"
      ;;
    successor_cycle)
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"superseded","topic":"alpha","role":"design","owner":"dev/design/legacy.md","release":"historical:0.8.0","successor":"dev/design/legacy.md"},
  {"path":"dev/design/legacy.md","class":"superseded","topic":"legacy","role":"design","owner":"dev/design/alpha.md","release":"historical:0.8.0","successor":"dev/design/alpha.md"}
]}
JSON
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
    commented_local_wiring)
      write_fixture "$root" valid
      printf '%s\n' \
        '# run_capped check-design-lifecycle "$SCRIPT_DIR/check-design-lifecycle.py"' \
        >"$root/scripts/agent-lint-md.sh"
      ;;
    commented_ci_wiring)
      write_fixture "$root" valid
      printf '%s\n' \
        'jobs:' \
        '  markdownlint:' \
        "    if: needs.changes.outputs.docs_only == 'true'" \
        '    steps:' \
        '      # run: python3 scripts/check-design-lifecycle.py' \
        >"$root/.github/workflows/ci.yml"
      ;;
    wrong_ci_job)
      write_fixture "$root" valid
      printf '%s\n' \
        'jobs:' \
        '  markdownlint:' \
        "    if: needs.changes.outputs.docs_only == 'true'" \
        '    steps:' \
        '      - run: true' \
        '  unrelated:' \
        '    steps:' \
        '      - run: python3 scripts/check-design-lifecycle.py' \
        >"$root/.github/workflows/ci.yml"
      ;;
    missing_current_profile)
      write_fixture "$root" valid
      printf '{"schema_version":1,"profiles":[]}\n' \
        >"$root/dev/design/current-owner-authority.json"
      ;;
    extra_current_profile)
      write_fixture "$root" valid
      python3 - "$root/dev/design/current-owner-authority.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text())
extra = dict(payload["profiles"][0])
extra["path"] = "dev/design/legacy.md"
payload["profiles"].append(extra)
path.write_text(json.dumps(payload))
PY
      ;;
    proposed_adr)
      write_fixture "$root" valid
      sed -i 's/status: accepted/status: proposed/' "$root/dev/adr/current.md"
      ;;
    superseded_adr)
      write_fixture "$root" valid
      sed -i 's/status: accepted/status: superseded/' "$root/dev/adr/current.md"
      ;;
    malformed_adr_status)
      write_fixture "$root" valid
      printf '%s\n' '---' 'status:' '  nested: accepted' '---' '# Bad' \
        >"$root/dev/adr/current.md"
      ;;
    duplicate_spaced_adr_status)
      write_fixture "$root" valid
      sed -i '/status: accepted/a status : superseded' "$root/dev/adr/current.md"
      ;;
    draft_interface)
      write_fixture "$root" valid_locked_interface
      sed -i 's/status: locked/status: draft/' "$root/dev/interfaces/current.md"
      ;;
    malformed_interface_status)
      write_fixture "$root" valid_locked_interface
      sed -i '/status: locked/a status: locked' "$root/dev/interfaces/current.md"
      ;;
    agents_wrong_role)
      write_fixture "$root" valid
      sed -i 's#dev/adr/current.md#AGENTS.md#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    authority_cycle)
      write_fixture "$root" valid
      cat >"$root/dev/design/document-lifecycle.json" <<'JSON'
{"schema_version":1,"documents":[
  {"path":"dev/design/alpha.md","class":"maintained","topic":"alpha","role":"design","owner":"dev/design/alpha.md","release":"cross-release","successor":null},
  {"path":"dev/design/legacy.md","class":"maintained","topic":"legacy","role":"design","owner":"dev/design/legacy.md","release":"cross-release","successor":null}
]}
JSON
      cat >"$root/dev/design/current-owner-authority.json" <<'JSON'
{"schema_version":1,"profiles":[
  {"path":"dev/design/alpha.md","profile":"current","semantic_authority":["dev/design/legacy.md"],"implementation_witness":["src/current.rs"],"evidence_only":[]},
  {"path":"dev/design/legacy.md","profile":"current","semantic_authority":["dev/design/alpha.md"],"implementation_witness":["src/current.rs"],"evidence_only":[]}
]}
JSON
      ;;
    historical_authority)
      write_fixture "$root" valid
      mkdir -p "$root/dev/plans"
      printf '# Historical plan\n' >"$root/dev/plans/historical.md"
      sed -i 's#dev/adr/current.md#dev/plans/historical.md#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    invalid_witness_root)
      write_fixture "$root" valid
      printf 'witness\n' >"$root/dev/witness.txt"
      sed -i 's#src/current.rs#dev/witness.txt#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    overlapping_relationships)
      write_fixture "$root" valid
      sed -i 's#"evidence_only":\[\]#"evidence_only":["dev/adr/current.md"]#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    unsorted_relationships)
      write_fixture "$root" valid
      printf 'fn alternate() {}\n' >"$root/src/alternate.rs"
      sed -i 's#"implementation_witness":\["src/current.rs"\]#"implementation_witness":["src/current.rs","src/alternate.rs"]#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    duplicate_current_profile)
      write_fixture "$root" valid
      python3 - "$root/dev/design/current-owner-authority.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text())
payload["profiles"].append(dict(payload["profiles"][0]))
path.write_text(json.dumps(payload))
PY
      ;;
    empty_witness)
      write_fixture "$root" valid
      sed -i 's#"implementation_witness":\["src/current.rs"\]#"implementation_witness":[]#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    nonexistent_witness)
      write_fixture "$root" valid
      sed -i 's#src/current.rs#src/missing.rs#' \
        "$root/dev/design/current-owner-authority.json"
      ;;
    non_current_profile)
      write_fixture "$root" valid
      sed -i 's/"profile":"current"/"profile":"historical"/' \
        "$root/dev/design/current-owner-authority.json"
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
  git -C "$root" init -q
  git -C "$root" add \
    dev/design/alpha.md \
    dev/design/legacy.md \
    dev/design/document-lifecycle.json
  python3 "$CHECKER" --repo-root "$root" >/dev/null
}

run_fail() {
  local mode=$1
  local root="$TMP_ROOT/$mode"
  write_fixture "$root" "$mode"
  git -C "$root" init -q
  git -C "$root" add \
    dev/design/alpha.md \
    dev/design/legacy.md \
    dev/design/document-lifecycle.json
  if python3 "$CHECKER" --repo-root "$root" >/dev/null 2>&1; then
    echo "FAIL: checker accepted $mode fixture" >&2
    exit 1
  fi
}

run_ok valid
run_ok valid_external_successor
run_ok valid_locked_interface
run_ok valid_agents_method
run_ok valid_historical_evidence
run_ok untracked_draft
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
  self_successor \
  successor_cycle \
  duplicate_owner \
  missing_local_wiring \
  missing_ci_wiring \
  commented_local_wiring \
  commented_ci_wiring \
  wrong_ci_job \
  missing_current_profile \
  extra_current_profile \
  proposed_adr \
  superseded_adr \
  malformed_adr_status \
  duplicate_spaced_adr_status \
  draft_interface \
  malformed_interface_status \
  agents_wrong_role \
  authority_cycle \
  historical_authority \
  invalid_witness_root \
  overlapping_relationships \
  unsorted_relationships \
  duplicate_current_profile \
  empty_witness \
  nonexistent_witness \
  non_current_profile
do
  run_fail "$mode"
done

echo "All design-lifecycle tests passed"
