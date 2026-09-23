#!/usr/bin/env bash
# Regenerate the committed hidden-surface oracle fixtures.
#
# Builds rustdoc JSON (pinned nightly, doc-hidden items included) for the
# fixture crates and records `cargo check --release` diagnostics for the
# committed release-probe sources. Output is canonical and host-independent:
# `external_crates[*].path` and `target.target_features` are removed, `paths`
# is pruned to the ids the index references, and the scratch directory is
# replaced by `/FIXTURE` in recorded cargo messages. Rerunning on the recorded
# host triple reproduces every file byte-for-byte. The self-tests never run
# this script and never need the nightly toolchain.
set -euo pipefail

readonly TOOLCHAIN="nightly-2026-04-24"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly here
out="${here}/json"

if ! cargo "+${TOOLCHAIN}" --version >/dev/null 2>&1; then
  printf 'regenerate.sh: pinned toolchain %s is not installed\n' "${TOOLCHAIN}" >&2
  exit 2
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/hs-fixture-regen.XXXXXX")"
trap 'rm -rf "${scratch}"' EXIT

for dir in crate crate-moved crate-changed facade probe; do
  cp -R "${here}/${dir}" "${scratch}/${dir}"
done
cp -R "${here}/facade" "${scratch}/facade-moved"
sed -i 's|path = "../crate"|path = "../crate-moved"|' "${scratch}/facade-moved/Cargo.toml"
grep -q 'path = "../crate-moved"' "${scratch}/facade-moved/Cargo.toml"
mkdir -p "${out}"

canonicalize() {
  python3 - "$1" "$2" <<'PY'
import json
import sys

source, destination = sys.argv[1], sys.argv[2]
with open(source, encoding="utf-8") as handle:
    doc = json.load(handle)
for crate in doc["external_crates"].values():
    crate.pop("path", None)
doc["target"].pop("target_features", None)
referenced = set()


def collect(value):
    if isinstance(value, dict):
        if isinstance(value.get("id"), int):
            referenced.add(str(value["id"]))
        for child in value.values():
            collect(child)
    elif isinstance(value, list):
        for child in value:
            collect(child)


collect(doc["index"])
doc["paths"] = {key: value for key, value in doc["paths"].items() if key in referenced}
with open(destination, "w", encoding="utf-8") as handle:
    handle.write(json.dumps(doc, sort_keys=True, separators=(",", ":"), ensure_ascii=False))
    handle.write("\n")
PY
}

rustdoc_row() {
  local dir="$1" crate="$2" name="$3"
  shift 3
  (
    cd "${scratch}/${dir}"
    CARGO_TARGET_DIR="${scratch}/target-${name}" cargo "+${TOOLCHAIN}" rustdoc \
      --offline --lib --no-default-features "$@" \
      -- -Z unstable-options --output-format json --document-hidden-items
  )
  canonicalize "${scratch}/target-${name}/doc/${crate}.json" "${out}/${name}.json"
}

rustdoc_row crate hs_fixture base-default
rustdoc_row crate hs_fixture base-hooks --features hooks
rustdoc_row crate-moved hs_fixture moved-default
rustdoc_row crate-changed hs_fixture changed-default
rustdoc_row facade hs_facade facade-default
rustdoc_row facade-moved hs_facade facade-moved-default

probe_row() {
  local dir="$1" crate="$2"
  local raw="${scratch}/probe-${crate}.raw"
  local status=0
  mkdir -p "${scratch}/${dir}/examples"
  cp "${scratch}/probe/${crate}.rs" "${scratch}/${dir}/examples/hs_probe.rs"
  (
    cd "${scratch}/${dir}"
    CARGO_TARGET_DIR="${scratch}/target-probe-${crate}" cargo "+${TOOLCHAIN}" check \
      --release --offline --example hs_probe --message-format json
  ) >"${raw}" 2>/dev/null || status=$?
  if [[ "${status}" -ne 101 ]]; then
    printf 'regenerate.sh: probe check for %s exited %s, expected 101\n' "${crate}" "${status}" >&2
    exit 2
  fi
  python3 - "${raw}" "${out}/probe-${crate}.jsonl" "${scratch}" <<'PY'
import json
import sys

raw, destination, scratch = sys.argv[1], sys.argv[2], sys.argv[3]
lines = []
with open(raw, encoding="utf-8") as handle:
    for line in handle:
        message = json.loads(line)
        if message.get("reason") != "compiler-message":
            continue
        if message.get("target", {}).get("name") != "hs_probe":
            continue
        text = json.dumps(message, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        lines.append(text.replace(scratch, "/FIXTURE"))
with open(destination, "w", encoding="utf-8") as handle:
    handle.write("".join(line + "\n" for line in lines))
PY
}

probe_row crate hs_fixture
probe_row facade hs_facade

printf 'regenerated fixtures in %s\n' "${out}"
