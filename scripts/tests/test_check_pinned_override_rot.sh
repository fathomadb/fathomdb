#!/usr/bin/env bash
# RED-first recurrence coverage for the offline pinned-override rot gate.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-pinned-override-rot.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

# The schema-5 Cargo shape (0.8.28 Slice 31, design § 3.4): the root patch
# table holds only candle-kernels by git at K9 (plus the SQLite path patch the
# seed adds); the embedder pins the Candle fork crates and cudarc-fathomdb
# exactly (R-CODE-4: Candle requires cudarc-fathomdb only as a caret
# requirement); Cargo.lock holds each fork crate once,
# from crates.io, with its published checksum.
write_cargo_shape() {
  local fixture="$1" mode="$2"
  python3 - "$fixture" "$mode" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
mode = sys.argv[2]
git = "https://github.com/coreyt/candle-fathomdb.git"
k9 = "182ceab0547348993b6dbec4b1aed610dcc8b23f"
other_rev = "89abcdef0123456789abcdef0123456789abcdef"
registry = "registry+https://github.com/rust-lang/crates.io-index"
cudarc_git = "git+https://github.com/coreyt/cudarc.git?rev=b777f8d94eedca8334e9331cf520d554b22544b7#b777f8d94eedca8334e9331cf520d554b22544b7"
checksums = {
    "cudarc-fathomdb": "8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2",
    "candle-core-fathomdb": "b5feacc063f44c77a54cb8b43d03c5e32fa5421f4163f0524194783e21563341",
    "candle-nn-fathomdb": "69c7432aa63cb543065f49453fa2e258e4dc3bd458abbcb7c150d17e90026200",
    "candle-transformers-fathomdb": "513008c79a9751ec5fdc08918dcdb1a8872c95e90f7697a9f8c5ef1951e3d222",
}
versions = {"cudarc-fathomdb": "0.19.10", "candle-core-fathomdb": "0.11.0",
            "candle-nn-fathomdb": "0.11.0", "candle-transformers-fathomdb": "0.11.0"}

patch_rev = other_rev if mode == "revision-drift" else k9
patches = "" if mode == "missing-kernels" else f'candle-kernels = {{ git = "{git}", rev = "{patch_rev}" }}\n'
if mode == "cudarc-patch":
    patches += 'cudarc = { path = "local/cudarc" }\n'
if mode == "candle-git-patch":
    patches += f'candle-core-fathomdb = {{ git = "{git}", rev = "{k9}" }}\n'
extra_manifest = ""
if mode == "replace":
    extra_manifest = "\n[replace]\n\"other:1.0.0\" = { git = \"https://example.invalid/other.git\", rev = \"0123456789abcdef0123456789abcdef01234567\" }\n"
elif mode == "direct-git":
    extra_manifest = "\n[workspace.dependencies]\nother = { git = \"https://example.invalid/other.git\", rev = \"0123456789abcdef0123456789abcdef01234567\" }\n"
(root / "Cargo.toml").write_text(
    "[workspace]\nresolver = \"2\"\n\n[patch.crates-io]\n" + patches + extra_manifest, encoding="utf-8"
)

core_requirement = "0.11.0" if mode == "caret" else "=0.11.0"
embedder = (
    "[package]\nname = \"fathomdb-embedder\"\nversion = \"0.0.0\"\n\n[dependencies]\n"
    f'candle-core = {{ package = "candle-core-fathomdb", version = "{core_requirement}", optional = true }}\n'
    'candle-nn = { package = "candle-nn-fathomdb", version = "=0.11.0", optional = true }\n'
    'candle-transformers = { package = "candle-transformers-fathomdb", version = "=0.11.0", optional = true, default-features = false }\n'
)
if mode == "cudarc-caret":
    embedder += 'cudarc-fathomdb = { version = "0.19.10", optional = true, default-features = false }\n'
elif mode != "cudarc-undeclared":
    embedder += 'cudarc-fathomdb = { version = "=0.19.10", optional = true, default-features = false }\n'
if mode == "direct-exact":
    embedder += 'cudarc = { package = "cudarc-fathomdb", version = "=0.19.10", optional = true }\n'
elif mode == "direct-target-caret":
    embedder += (
        "\n[target.'cfg(all(target_os = \"linux\", target_arch = \"aarch64\"))'.dependencies]\n"
        'cudarc = { package = "cudarc-fathomdb", version = "0.19.10", optional = true }\n'
    )
embedder_path = root / "src/rust/crates/fathomdb-embedder/Cargo.toml"
embedder_path.parent.mkdir(parents=True, exist_ok=True)
embedder_path.write_text(embedder, encoding="utf-8")

lock = []
for name, version in versions.items():
    source = registry
    checksum = checksums[name]
    if name == "cudarc-fathomdb" and mode == "lock-git-source":
        source, checksum = cudarc_git, None
    if name == "cudarc-fathomdb" and mode == "lock-checksum":
        checksum = "0" * 64
    entry = f'[[package]]\nname = "{name}"\nversion = "{version}"\nsource = "{source}"\n'
    if checksum:
        entry += f'checksum = "{checksum}"\n'
    lock.append(entry)
if mode == "lock-second-git":
    lock.append(f'[[package]]\nname = "cudarc-fathomdb"\nversion = "0.19.10"\nsource = "{cudarc_git}"\n')
if mode == "lock-second-version":
    lock.append(f'[[package]]\nname = "cudarc-fathomdb"\nversion = "0.19.11"\nsource = "{registry}"\nchecksum = "{"1" * 64}"\n')
kernels_rev = other_rev if mode == "lock-drift" else k9
kernels_version = "0.10.3" if mode == "version-drift" else "0.10.2"
lock.append(f'[[package]]\nname = "candle-kernels"\nversion = "{kernels_version}"\nsource = "git+{git}?rev={kernels_rev}#{kernels_rev}"\n')
(root / "Cargo.lock").write_text("version = 4\n\n" + "\n".join(lock), encoding="utf-8")

if mode == "metadata-missing-pin":
    metadata = root / "scripts/pinned-override-rot.json"
    data = json.loads(metadata.read_text())
    pins = data.get("cargo_fork_registry_pins", {}).get("pins", [])
    data.setdefault("cargo_fork_registry_pins", {})["pins"] = [
        pin for pin in pins if pin.get("package") != "candle-nn-fathomdb"
    ]
    metadata.write_text(json.dumps(data), encoding="utf-8")
PY
}

make_fixture() {
  local name="$1"
  local mode="$2"
  local fixture="$WORK/$name"
  mkdir -p "$fixture/scripts"
  cp "$REPO_ROOT/scripts/pinned-override-rot.json" "$fixture/scripts/pinned-override-rot.json"
  cp "$REPO_ROOT/scripts/pinned-override-advisories.json" "$fixture/scripts/pinned-override-advisories.json"
  python3 - "$fixture" "$mode" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
mode = sys.argv[2]
metadata = root / "scripts/pinned-override-rot.json"
data = json.loads(metadata.read_text())
data["schema_version"] = 5
package = "js-yaml"
version = "4.2.0" if mode == "vulnerable" else "4.3.0"
if mode == "prerelease":
    version = "4.3.0-rc.1"
data["npm_overrides"] = [{
    "package": package,
    "version": version,
    "rationale": "fixture security override",
    "advisory_ids": ["GHSA-h67p-54hq-rp68", "GHSA-52cp-r559-cp3m"],
    "unpin_evidence": {
        "resolved_version": "4.3.0",
        "dependent_ranges": [">=4.0.0, <5.0.0"],
        "provenance": "fixture no-override resolution"
    }
}]
if mode == "obsolete":
    data["npm_overrides"][0].pop("unpin_evidence")
if mode == "falsified-r2":
    data["npm_overrides"][0]["unpin_evidence"]["resolved_version"] = "999.0.0"
if mode in {"snapshot-digest", "missing-advisory-mapping"}:
    data["npm_overrides"][0]["unpin_evidence"]["resolved_version"] = "1.0.0"
if mode == "missing-advisory-mapping":
    data["npm_overrides"][0].pop("advisory_ids", None)
if mode == "snapshot-digest":
    data["advisory_snapshot"]["sha256"] = "0" * 64
if mode == "missing-rationale":
    data["npm_overrides"][0].pop("rationale")
if mode == "malformed-advisories":
    source = root / "scripts/pinned-override-advisories.json"
    snapshot = json.loads(source.read_text())
    snapshot["advisories"] = []
    source.write_text(json.dumps(snapshot), encoding="utf-8")
if mode == "empty-valid-advisory-snapshot":
    source = root / "scripts/pinned-override-advisories.json"
    snapshot = json.loads(source.read_text())
    snapshot["advisories"] = []
    source.write_text(json.dumps(snapshot), encoding="utf-8")
    data["advisory_snapshot"]["sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
if mode in {"forged-metadata-source", "forged-source", "malformed-advisory-id", "mismatched-advisory-url", "canonical-looking-forgery"}:
    source = root / "scripts/pinned-override-advisories.json"
    snapshot = json.loads(source.read_text())
    if mode == "forged-metadata-source":
        data["advisory_snapshot"]["source"] = "Invented advisory source"
    elif mode == "forged-source":
        snapshot["source"]["name"] = "Invented advisory source"
    elif mode == "malformed-advisory-id":
        snapshot["advisories"][0]["id"] = "NOT-A-GHSA"
    elif mode == "canonical-looking-forgery":
        snapshot["advisories"][0]["id"] = "GHSA-2345-2345-2345"
        snapshot["advisories"][0]["url"] = "https://github.com/advisories/GHSA-2345-2345-2345"
        snapshot["source"]["provenance"] = "Forged but canonical-looking GitHub advisory source"
    else:
        snapshot["advisories"][0]["url"] = "https://github.com/advisories/GHSA-0000-0000-0000"
    source.write_text(json.dumps(snapshot), encoding="utf-8")
    data["advisory_snapshot"]["sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
metadata.write_text(json.dumps(data), encoding="utf-8")
(root / "package.json").write_text(json.dumps({"overrides": {package: version}}), encoding="utf-8")
(root / "package-lock.json").write_text(json.dumps({
    "lockfileVersion": 3,
    "packages": {"node_modules/parent": {"dependencies": {
        package: ">=4.0.0, <5.0.0"
    }}}
}), encoding="utf-8")
PY
  write_cargo_shape "$fixture" approved
  printf '%s' "$fixture"
}

seed_sqlite_fixture() {
  local fixture="$1"
  if [ "$fixture" = "$REPO_ROOT" ] || [ -e "$fixture/.omit-sqlite-patch" ]; then
    return
  fi
  mkdir -p "$fixture/third_party/libsqlite3-sys-0.38.1"
  if [ ! -e "$fixture/third_party/libsqlite3-sys-0.38.1/sqlite3/sqlite3.c" ]; then
    cp -a "$REPO_ROOT/third_party/libsqlite3-sys-0.38.1/." \
      "$fixture/third_party/libsqlite3-sys-0.38.1/"
  fi
  python3 - "$fixture" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
manifest = root / "Cargo.toml"
if manifest.exists():
    text = manifest.read_text(encoding="utf-8")
    patch = 'libsqlite3-sys = { path = "third_party/libsqlite3-sys-0.38.1" }\n'
    if 'libsqlite3-sys = ' not in text and "[patch.crates-io]\n" in text:
        text = text.replace("[patch.crates-io]\n", "[patch.crates-io]\n" + patch, 1)
        manifest.write_text(text, encoding="utf-8")
lock = root / "Cargo.lock"
if lock.exists():
    text = lock.read_text(encoding="utf-8")
    if 'name = "libsqlite3-sys"' not in text:
        lock.write_text(text + '\n[[package]]\nname = "libsqlite3-sys"\nversion = "0.38.1"\n', encoding="utf-8")
PY
}

run_fixture() {
  local fixture="$1"
  seed_sqlite_fixture "$fixture"
  set +e
  OUT="$(bash "$CHECKER" --root "$fixture" 2>&1)"
  RC=$?
  set -e
}

make_and_run_fixture() {
  local fixture
  fixture="$(make_fixture "$@")"
  run_fixture "$fixture"
}

# Structural checks must still apply after an intentionally reviewed snapshot
# refresh. This copies the checker and replaces only its source-owned anchor
# with the fixture's digest; ordinary fixtures always use the real anchor.
run_fixture_with_reanchored_snapshot() {
  local fixture="$1"
  seed_sqlite_fixture "$fixture"
  local fixture_checker="$fixture/check-pinned-override-rot.py"
  python3 - "$REPO_ROOT/scripts/check-pinned-override-rot.py" "$fixture" "$fixture_checker" <<'PY'
import hashlib
import re
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text(encoding="utf-8")
root = Path(sys.argv[2])
target = Path(sys.argv[3])
digest = hashlib.sha256((root / "scripts/pinned-override-advisories.json").read_bytes()).hexdigest()
updated, count = re.subn(
    r'^PINNED_ADVISORY_SNAPSHOT_SHA256 = "[0-9a-f]{64}"$',
    f'PINNED_ADVISORY_SNAPSHOT_SHA256 = "{digest}"',
    source,
    flags=re.MULTILINE,
)
if count != 1:
    raise SystemExit("fixture could not replace exactly one checker advisory anchor")
target.write_text(updated, encoding="utf-8")
PY
  set +e
  OUT="$(python3 "$fixture_checker" --root "$fixture" 2>&1)"
  RC=$?
  set -e
}

make_and_run_fixture_with_reanchored_snapshot() {
  local fixture
  fixture="$(make_fixture "$@")"
  run_fixture_with_reanchored_snapshot "$fixture"
}

expect_failure() {
  local expected="$1" description="$2"
  if [ "$RC" -ne 1 ] && [ "$RC" -ne 2 ]; then
    fail "$description — expected a hard failure, got rc=$RC output=$OUT"
  fi
  if ! grep -Fq "$expected" <<<"$OUT"; then
    fail "$description — output did not name $expected: $OUT"
  fi
  pass "$description"
}

# Ignored benchmark copies contain historical Cargo patches. They are not
# source inputs, while a new nonignored manifest must still be inspected.
python3 - "$REPO_ROOT/scripts/check-pinned-override-rot.py" "$WORK" <<'PY'
from pathlib import Path
import runpy
import subprocess
import sys

checker = runpy.run_path(sys.argv[1])
root = Path(sys.argv[2]) / "ignored-manifest-scope"
(root / "data/benchmark").mkdir(parents=True)
(root / "extra").mkdir()
(root / ".gitignore").write_text("/data/\n", encoding="utf-8")
(root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
(root / "data/benchmark/Cargo.toml").write_text(
    '[patch.crates-io]\nold = { git = "https://example.invalid/old" }\n', encoding="utf-8"
)
subprocess.run(["git", "init", "-q", str(root)], check=True)
assert checker["cargo_governed_pins"](root) == [], "ignored benchmark manifest reached the gate"
(root / "extra/Cargo.toml").write_text(
    '[dependencies]\nnew = { git = "https://example.invalid/new" }\n', encoding="utf-8"
)
assert checker["cargo_governed_pins"](root), "nonignored new manifest escaped the gate"
PY
pass "Cargo override scan excludes ignored benchmark copies but sees new source"

# R1 exact historical regression: 4.2.0 was the old js-yaml override and is
# inside GHSA-52cp-r559-cp3m's >=4.0.0,<4.3.0 range.
make_and_run_fixture vulnerable vulnerable
expect_failure 'R1 npm override js-yaml@4.2.0 is vulnerable to GHSA-52cp-r559-cp3m' \
  'R1 rejects the historical vulnerable js-yaml 4.2.0 override'

# R2 deliberately fails closed: a lockfile produced while an override is live
# cannot prove what npm would resolve without that override.
make_and_run_fixture obsolete obsolete
if [ "$RC" -ne 2 ]; then
  fail "R2 must be unverified without reproducible evidence, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: R2 cannot derive a no-override resolution' \
  'R2 fails closed when npm cannot reproduce a no-override resolution'

# R3: an override cannot rely on an unstructured package.json comment.
make_and_run_fixture missing-rationale missing-rationale
expect_failure 'R3 npm override js-yaml@4.3.0 has no recorded rationale' \
  'R3 rejects an override without a recorded rationale'

# Advisory input unavailable/malformed is unverified, not clean.
make_and_run_fixture malformed-advisories malformed-advisories
if [ "$RC" -ne 2 ]; then
  fail "malformed advisory input must exit 2 (unverified), got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot sha256' \
  'malformed advisory input loudly refuses a clean verdict'

# A missing checked-in advisory source is also an unverified failure, never a
# network fallback or an implicit pass.
fixture="$(make_fixture missing-source obsolete)"
set +e
OUT="$(bash "$CHECKER" --root "$fixture" --metadata "$fixture/scripts/does-not-exist.json" 2>&1)"
RC=$?
set -e
if [ "$RC" -ne 2 ]; then
  fail "missing advisory source must exit 2 (unverified), got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: cannot read pinned-override metadata' \
  'missing advisory input loudly refuses a clean verdict'

# SemVer prereleases cannot be compared by the intentionally small stable-only
# range grammar. They must refuse a verdict, rather than flattening rc.1 to
# 2.0.0 and potentially reporting an R1 false green.
make_and_run_fixture prerelease prerelease
if [ "$RC" -ne 2 ]; then
  fail "prerelease override must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: prerelease version' \
  'R1 refuses prerelease pins that the stable-only comparator cannot order'

# The old R2 decision trusted a JSON field that anyone could edit. A falsified
# self-attestation cannot convert this gate into a clean (or obsolete) result.
make_and_run_fixture falsified-r2 falsified-r2
if [ "$RC" -ne 2 ]; then
  fail "self-attested R2 evidence must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: R2 cannot derive a no-override resolution' \
  'R2 refuses falsified self-attested no-override evidence'

# Advisory content is a separate checked-in snapshot. Its digest is pinned by
# governed metadata, so a valid-but-edited JSON source cannot silently erase an
# advisory.
make_and_run_fixture snapshot-digest snapshot-digest
if [ "$RC" -ne 2 ]; then
  fail "advisory digest mismatch must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot sha256' \
  'advisory snapshot content digest is enforced'

# Updating the digest cannot make an intentionally empty but valid JSON
# snapshot trustworthy: completeness is checked separately from integrity.
make_and_run_fixture_with_reanchored_snapshot empty-valid-advisory-snapshot empty-valid-advisory-snapshot
if [ "$RC" -ne 2 ]; then
  fail "empty advisory snapshot must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot advisories must be a non-empty list' \
  'empty valid advisory snapshot is not a clean result'

# Every governed override must name the advisories considered for that package.
# An empty or omitted mapping is not evidence that there are no advisories.
make_and_run_fixture missing-advisory-mapping missing-advisory-mapping
if [ "$RC" -ne 2 ]; then
  fail "missing per-pin advisory mapping must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: npm override js-yaml.advisory_ids' \
  'per-pin advisory mapping cannot be omitted'

# The digest binds a checked-in snapshot, but it must not let a rehashed
# arbitrary source, identifier, or URL impersonate the GitHub Advisory DB.
# Each arm deliberately recomputes the digest after mutating that field.
make_and_run_fixture forged-metadata-source forged-metadata-source
if [ "$RC" -ne 2 ]; then
  fail "forged metadata advisory source must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory_snapshot.source' \
  'metadata source must be the canonical GitHub Advisory Database'

make_and_run_fixture_with_reanchored_snapshot forged-source forged-source
if [ "$RC" -ne 2 ]; then
  fail "forged advisory source must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot source.name' \
  'snapshot source must be the canonical GitHub Advisory Database'

make_and_run_fixture_with_reanchored_snapshot malformed-advisory-id malformed-advisory-id
if [ "$RC" -ne 2 ]; then
  fail "malformed GitHub advisory id must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot advisories[0].id' \
  'snapshot advisory id must use the canonical GHSA form'

make_and_run_fixture_with_reanchored_snapshot mismatched-advisory-url mismatched-advisory-url
if [ "$RC" -ne 2 ]; then
  fail "mismatched GitHub advisory URL must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot advisories[0].url' \
  'snapshot advisory URL must exactly name its GHSA identifier'

# A checksum recorded only in mutable metadata cannot authenticate a forged
# but canonical-looking GitHub source. Recomputing that metadata checksum must
# still fail against the checker-source anchor.
make_and_run_fixture canonical-looking-forgery canonical-looking-forgery
if [ "$RC" -ne 2 ]; then
  fail "canonical-looking forged advisory snapshot must be unverified, got rc=$RC output=$OUT"
fi
expect_failure 'UNVERIFIED pinned-override-rot: advisory snapshot sha256 does not match independently pinned checker digest' \
  'canonical-looking forged advisory snapshot cannot bypass the checker anchor'

# Cargo git sources can appear in workspace and target-specific dependency
# tables, not just root dependency sections. Both must block a clean result.
cargo_fixture="$WORK/cargo-nested-git"
mkdir -p "$cargo_fixture/scripts" "$cargo_fixture/member"
cp "$REPO_ROOT/scripts/pinned-override-rot.json" "$cargo_fixture/scripts/pinned-override-rot.json"
cp "$REPO_ROOT/scripts/pinned-override-advisories.json" "$cargo_fixture/scripts/pinned-override-advisories.json"
printf '%s\n' '{"overrides": {}}' >"$cargo_fixture/package.json"
printf '%s\n' '{"lockfileVersion": 3, "packages": {}}' >"$cargo_fixture/package-lock.json"
printf '%s\n' 'version = 4' 'package = []' >"$cargo_fixture/Cargo.lock"
cat >"$cargo_fixture/Cargo.toml" <<'EOF'
[workspace]
resolver = "2"

[workspace.dependencies]
workspace-git = { git = "https://example.invalid/workspace-git" }
EOF
cat >"$cargo_fixture/member/Cargo.toml" <<'EOF'
[package]
name = "member"
version = "0.1.0"

[target.'cfg(unix)'.dependencies]
target-git = { git = "https://example.invalid/target-git" }
EOF
run_fixture "$cargo_fixture"
if [ "$RC" -ne 1 ]; then
  fail "nested Cargo git dependencies must fail, got rc=$RC output=$OUT"
fi
expect_failure 'Cargo.toml:workspace.dependencies.workspace-git' \
  'Cargo workspace dependency git override is detected'
expect_failure 'member/Cargo.toml:target.cfg(unix).dependencies.target-git' \
  'Cargo target-specific dependency git override is detected'

# Cargo scope is deliberately a short list of FathomDB exceptions, not a
# general package-ID parser.
make_candle_fixture() {
  local name="$1"
  local mode="$2"
  local fixture="$WORK/$name"
  mkdir -p "$fixture/scripts"
  cp "$REPO_ROOT/scripts/pinned-override-rot.json" "$fixture/scripts/pinned-override-rot.json"
  cp "$REPO_ROOT/scripts/pinned-override-advisories.json" "$fixture/scripts/pinned-override-advisories.json"
  printf '%s\n' '{"overrides": {}}' >"$fixture/package.json"
  printf '%s\n' '{"lockfileVersion": 3, "packages": {}}' >"$fixture/package-lock.json"
  write_cargo_shape "$fixture" "$mode"
  case "$mode" in
    config-patch)
      mkdir -p "$fixture/.cargo"
      printf '%s\n' '[patch.crates-io]' >"$fixture/.cargo/config.toml"
      ;;
    config-unparseable)
      mkdir -p "$fixture/.cargo"
      printf '%s' 'not = [valid' >"$fixture/.cargo/config"
      ;;
    config-ordinary)
      mkdir -p "$fixture/.cargo"
      printf '%s\n' '[build]' 'target-dir = "target"' >"$fixture/.cargo/config.toml"
      ;;
  esac
  printf '%s' "$fixture"
}

make_and_run_candle_fixture() {
  local fixture
  fixture="$(make_candle_fixture "$@")"
  run_fixture "$fixture"
}

make_and_run_candle_fixture approved approved
if [ "$RC" -ne 0 ]; then
  fail "approved schema-5 Cargo shape must pass, got rc=$RC output=$OUT"
fi
pass 'approved candle-kernels patch and fork registry pins pass'

sqlite_missing="$(make_candle_fixture sqlite-missing approved)"
touch "$sqlite_missing/.omit-sqlite-patch"
run_fixture "$sqlite_missing"
expect_failure 'missing approved SQLite patch libsqlite3-sys' \
  'missing local SQLite backport is rejected'

sqlite_drift="$(make_candle_fixture sqlite-drift approved)"
run_fixture "$sqlite_drift"
if [ "$RC" -ne 0 ]; then
  fail "exact local SQLite backport must pass, got rc=$RC output=$OUT"
fi
printf '\n' >>"$sqlite_drift/third_party/libsqlite3-sys-0.38.1/sqlite3/sqlite3.c"
run_fixture "$sqlite_drift"
expect_failure 'SQLite vendored amalgamation SHA-256 drift' \
  'modified local SQLite amalgamation is rejected'

sqlite_build="$(make_candle_fixture sqlite-build approved)"
run_fixture "$sqlite_build"
if [ "$RC" -ne 0 ]; then
  fail "exact local SQLite vendor tree must pass, got rc=$RC output=$OUT"
fi
printf '\n' >>"$sqlite_build/third_party/libsqlite3-sys-0.38.1/build.rs"
run_fixture "$sqlite_build"
expect_failure 'SQLite vendor tree SHA-256 drift' \
  'modified SQLite build script is rejected'

sqlite_path="$(make_candle_fixture sqlite-path approved)"
run_fixture "$sqlite_path"
if [ "$RC" -ne 0 ]; then
  fail "exact local SQLite path must pass, got rc=$RC output=$OUT"
fi
python3 - "$sqlite_path/Cargo.toml" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
path.write_text(path.read_text().replace(
    'libsqlite3-sys = { path = "third_party/libsqlite3-sys-0.38.1" }',
    'libsqlite3-sys = { path = "third_party/unreviewed-sqlite" }',
))
PY
run_fixture "$sqlite_path"
expect_failure 'SQLite patch is not the approved local path source' \
  'SQLite patch path drift is rejected'

# The kernel patch is the driverless-linkage boundary. Its omission must fail
# even if the remaining public Candle crates retain their governed source.
make_and_run_candle_fixture missing-kernels missing-kernels
if [ "$RC" -ne 1 ]; then
  fail "missing Candle kernel patch must fail, got rc=$RC output=$OUT"
fi
expect_failure 'missing approved Candle patch candle-kernels' \
  'missing candle-kernels patch is rejected'

make_and_run_candle_fixture revision-drift revision-drift
if [ "$RC" -ne 1 ]; then
  fail "Candle revision drift must fail, got rc=$RC output=$OUT"
fi
expect_failure 'Candle patch candle-kernels revision is not the approved immutable revision' \
  'candle_kernels_patch_at_another_rev_is_rejected'

make_and_run_candle_fixture lock-drift lock-drift
if [ "$RC" -ne 1 ]; then
  fail "Candle lock drift must fail, got rc=$RC output=$OUT"
fi
expect_failure 'Candle patch candle-kernels has no matching Cargo.lock source' \
  'Candle lock-source drift is rejected'

make_and_run_candle_fixture version-drift version-drift
if [ "$RC" -ne 1 ]; then
  fail "Candle per-package version drift must fail, got rc=$RC output=$OUT"
fi
expect_failure 'Candle patch candle-kernels has no matching Cargo.lock source' \
  'Candle per-package lock version drift is rejected'

make_and_run_candle_fixture replace replace
if [ "$RC" -ne 1 ]; then
  fail "non-Candle Cargo replace must fail, got rc=$RC output=$OUT"
fi
expect_failure 'unsupported Cargo override/git source Cargo.toml:replace.other:1.0.0' \
  'non-Candle Cargo replace is rejected'

make_and_run_candle_fixture direct-git direct-git
if [ "$RC" -ne 1 ]; then
  fail "direct Cargo Git dependency must fail, got rc=$RC output=$OUT"
fi
expect_failure 'unsupported Cargo override/git source Cargo.toml:workspace.dependencies.other' \
  'direct Cargo Git dependency is rejected'

make_and_run_candle_fixture config-ordinary config-ordinary
if [ "$RC" -ne 0 ]; then
  fail "ordinary Cargo config must remain allowed, got rc=$RC output=$OUT"
fi
pass 'ordinary Cargo config remains allowed'

make_and_run_candle_fixture config-patch config-patch
if [ "$RC" -ne 1 ]; then
  fail "Cargo config patch must fail, got rc=$RC output=$OUT"
fi
expect_failure 'ungoverned Cargo config patch .cargo/config.toml' \
  'Cargo config patch is rejected'

make_and_run_candle_fixture config-unparseable config-unparseable
if [ "$RC" -ne 1 ]; then
  fail "unparseable Cargo config must fail, got rc=$RC output=$OUT"
fi
expect_failure 'unparseable Cargo config .cargo/config' \
  'unparseable Cargo config is rejected'

# 0.8.28 Slice 31 (schema 5, design § 3.4): the fork crates are exact
# registry pins. Each arm asserts the checker's message.
make_and_run_candle_fixture caret caret
expect_failure 'fork registry pin candle-core-fathomdb: src/rust/crates/fathomdb-embedder/Cargo.toml dependencies.candle-core must require exactly "=0.11.0"' \
  'fork_registry_pin_with_a_caret_requirement_is_rejected'

# R-CODE-4: the embedder declares cudarc-fathomdb once, by its own name, as an
# exact pin; Candle's requirement on it is only a caret requirement.
make_and_run_candle_fixture cudarc-undeclared cudarc-undeclared
expect_failure 'fork registry pin cudarc-fathomdb: src/rust/crates/fathomdb-embedder/Cargo.toml dependencies.cudarc-fathomdb is not declared' \
  'an_undeclared_cudarc_fathomdb_pin_is_rejected'

make_and_run_candle_fixture cudarc-caret cudarc-caret
expect_failure 'fork registry pin cudarc-fathomdb: src/rust/crates/fathomdb-embedder/Cargo.toml dependencies.cudarc-fathomdb must require exactly "=0.19.10"' \
  'a_caret_cudarc_fathomdb_pin_is_rejected'

for direct in direct-exact direct-target-caret; do
  make_and_run_candle_fixture "$direct" "$direct"
  expect_failure 'fork registry pin cudarc-fathomdb: src/rust/crates/fathomdb-embedder/Cargo.toml must declare it once, as dependencies.cudarc-fathomdb' \
    "a_second_cudarc_fathomdb_declaration_is_rejected ($direct)"
done

make_and_run_candle_fixture lock-second-git lock-second-git
expect_failure 'Cargo.lock cudarc-fathomdb 0.19.10 has source git+https://github.com/coreyt/cudarc.git' \
  'fork_registry_pin_with_a_non_registry_source_in_the_lock_is_rejected'

make_and_run_candle_fixture lock-git-source lock-git-source
expect_failure 'Cargo.lock cudarc-fathomdb 0.19.10 has source git+https://github.com/coreyt/cudarc.git' \
  'fork_registry_pin_with_a_git_source_in_the_lock_is_rejected'

make_and_run_candle_fixture lock-checksum lock-checksum
expect_failure 'Cargo.lock cudarc-fathomdb 0.19.10 checksum is not the published 8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2' \
  'fork_registry_pin_with_a_different_checksum_is_rejected'

make_and_run_candle_fixture lock-second-version lock-second-version
expect_failure 'Cargo.lock must hold exactly one cudarc-fathomdb package (0.19.10); found 0.19.10, 0.19.11' \
  'fork_registry_pin_with_a_second_version_in_the_lock_is_rejected'

make_and_run_candle_fixture metadata-missing-pin metadata-missing-pin
expect_failure 'cargo_fork_registry_pins.pins must equal the checker-owned fork registry pins' \
  'missing_fork_registry_pin_is_rejected'

make_and_run_candle_fixture cudarc-patch cudarc-patch
expect_failure 'unsupported Cargo override/git source Cargo.toml:patch.crates-io.cudarc' \
  'a_cudarc_patch_entry_is_unsupported'

make_and_run_candle_fixture candle-git-patch candle-git-patch
expect_failure 'unsupported Cargo override/git source Cargo.toml:patch.crates-io.candle-core-fathomdb' \
  'a Candle fork crate reintroduced as a git patch is unsupported'

# The standalone Rust SDK driver (experiments/sdk_drivers/rust) is its own
# Cargo workspace, so it must repeat the root patch set. It may repeat only the
# approved root sources: libsqlite3-sys at the vendored path and candle-kernels
# at the approved revision. A stale copy of an older root set is unsupported.
write_sdk_driver() {
  local fixture="$1" mode="$2"
  python3 - "$fixture" "$mode" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
mode = sys.argv[2]
git = "https://github.com/coreyt/candle-fathomdb.git"
k9 = "182ceab0547348993b6dbec4b1aed610dcc8b23f"
old_rev = "1aefdd008ad1c994635b688b8e6f2ae5a5a920ae"
patches = []
if mode != "missing-sqlite":
    patches.append('libsqlite3-sys = { path = "../../../third_party/libsqlite3-sys-0.38.1" }')
if mode != "missing-kernels":
    patches.append(f'candle-kernels = {{ git = "{git}", rev = "{k9}" }}')
if mode == "stale-cudarc":
    patches.append('cudarc = { path = "../../../third_party/cudarc-0.19.7" }')
if mode == "stale-candle":
    patches.append(f'candle-core-fathomdb = {{ git = "{git}", rev = "{old_rev}" }}')
driver = root / "experiments/sdk_drivers/rust/Cargo.toml"
driver.parent.mkdir(parents=True, exist_ok=True)
driver.write_text(
    "[package]\nname = \"fathomdb-sdk-benchmark-driver\"\nversion = \"0.1.0\"\n\n[workspace]\n\n"
    "[patch.crates-io]\n" + "\n".join(patches) + "\n",
    encoding="utf-8",
)
PY
}

make_and_run_driver_fixture() {
  local name="$1" mode="$2" fixture
  fixture="$(make_candle_fixture "$name" approved)"
  write_sdk_driver "$fixture" "$mode"
  run_fixture "$fixture"
}

make_and_run_driver_fixture driver-approved approved
if [ "$RC" -ne 0 ]; then
  fail "SDK driver repeating only the approved root patches must pass, got rc=$RC output=$OUT"
fi
pass 'SDK driver duplicating the approved sqlite and candle-kernels patches passes'

make_and_run_driver_fixture driver-stale-cudarc stale-cudarc
expect_failure 'unsupported Cargo override/git source experiments/sdk_drivers/rust/Cargo.toml:patch.crates-io.cudarc' \
  'SDK driver with the stale vendored cudarc patch is rejected'

make_and_run_driver_fixture driver-stale-candle stale-candle
expect_failure 'unsupported Cargo override/git source experiments/sdk_drivers/rust/Cargo.toml:patch.crates-io.candle-core-fathomdb' \
  'SDK driver with a stale Candle fork git patch is rejected'

make_and_run_driver_fixture driver-missing-sqlite missing-sqlite
expect_failure 'missing SDK driver duplicate Cargo patch libsqlite3-sys' \
  'SDK driver missing the sqlite duplicate is rejected'

make_and_run_driver_fixture driver-missing-kernels missing-kernels
expect_failure 'missing SDK driver duplicate Cargo patch candle-kernels' \
  'SDK driver missing the candle-kernels duplicate is rejected'

# The real tree is the regression half: after removing obsolete root overrides,
# the snapshot remains parseable and the gate stays clean without a network.
run_fixture "$REPO_ROOT"
if [ "$RC" -ne 0 ]; then
  fail "real repository must pass the offline pin-rot gate: $OUT"
fi
pass 'real repository has exact governed npm and Cargo pin provenance'
