#!/usr/bin/env bash
# Regression fixtures for the Linux CUDA release-contract checker.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="${CHECKER_UNDER_TEST:-$REPO_ROOT/scripts/check-cuda-release-contract.py}"

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

make_fixture() {
  local root="$1"
  mkdir -p "$root/.github/workflows" "$root/scripts/lib" "$root/scripts/release" "$root/src/rust/crates/fathomdb-napi" "$root/src/ts" "$root/dev/release"
  cp "$REPO_ROOT/Cargo.toml" "$REPO_ROOT/Cargo.lock" "$root/"
  cp "$REPO_ROOT/.github/workflows/release.yml" "$root/.github/workflows/"
  cp "$REPO_ROOT/scripts/verify-release-gates.sh" "$root/scripts/"
  cp "$REPO_ROOT/scripts/release/cuda-artifact-contract.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/build-napi-cuda.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/build-python-cuda-tegra.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-preflight.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/lib/cuda-gpu-selection.sh" "$root/scripts/lib/"
  cp "$REPO_ROOT/scripts/release/cuda-image-attestation.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-preflight-witness.schema.json" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/verify-cuda-preflight-witness.py" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/verify-cuda-unmerged-candidate.py" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/verify-cuda-unmerged-receipt.py" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-unmerged-route-receipt.schema.json" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-package-rehearsal.schema.json" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/verify-cuda-package-rehearsal.py" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-package-rehearsal.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/cuda-package-rehearsal-smoke.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/seal-cuda-cli-archive.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/Dockerfile.cuda-manylinux" "$root/scripts/release/"
  cp "$REPO_ROOT/scripts/release/provision-cuda-manylinux.sh" "$root/scripts/release/"
  cp "$REPO_ROOT/src/rust/crates/fathomdb-napi/Cargo.toml" "$root/src/rust/crates/fathomdb-napi/"
  cp "$REPO_ROOT/src/ts/package.json" "$root/src/ts/"
  cp "$REPO_ROOT/dev/release/cuda-unmerged-candidates.json" "$root/dev/release/"
  cp "$REPO_ROOT/dev/release/cuda-unmerged-candidates.schema.json" "$root/dev/release/"
  cp "$REPO_ROOT/dev/release/cuda-protection-baseline.json" "$root/dev/release/"
}

require_provisioning_assets() {
  local file
  for file in \
    "$REPO_ROOT/scripts/release/Dockerfile.cuda-manylinux" \
    "$REPO_ROOT/scripts/release/provision-cuda-manylinux.sh"; do
    if [ ! -f "$file" ]; then
      printf 'FAIL  missing reproducible CUDA provisioning asset: %s\n' "$file" >&2
      exit 1
    fi
  done
}

expect_pass() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null; then
    printf 'PASS  %s\n' "$description"
  else
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
}

expect_fail() {
  local root="$1" description="$2"
  if REPO_ROOT="$root" python3 "$CHECKER" >/dev/null 2>&1; then
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
  printf 'PASS  %s\n' "$description"
}

assert_unmerged_control_plane() {
  local root="$1"
  REPO_ROOT="$root" python3 "$CHECKER" >/dev/null
}

require_provisioning_assets

# The CUDA contract checker is part of the Python 3.10 release-tooling
# envelope. `datetime.UTC` was introduced in Python 3.11, so use the
# long-standing `timezone.utc` spelling instead.
if grep -Fq 'from datetime import UTC, datetime' "$CHECKER" \
  || ! grep -Fq 'from datetime import datetime, timezone' "$CHECKER" \
  || ! grep -Fq 'datetime.now(timezone.utc)' "$CHECKER"; then
  printf 'FAIL  CUDA release-contract checker must remain compatible with Python 3.10\n' >&2
  exit 1
fi
printf 'PASS  CUDA release-contract checker remains Python 3.10 compatible\n'

# The verifier's eligibility decision is inline in the trusted workflow. A
# candidate can alter its ordinary release-gate script without making the
# self-hosted job eligible.
assert_unmerged_control_plane "$REPO_ROOT"
printf 'PASS  hosted CUDA verifier is main-owned and candidate-independent\n'

FIXTURE="$TMPROOT/fixture"
make_fixture "$FIXTURE"
expect_pass "$FIXTURE" 'baseline CUDA contract agrees'

if grep -Fq 'bash candidate/scripts/release/cuda-package-rehearsal' "$REPO_ROOT/.github/workflows/release.yml" \
  || grep -Fq 'bash scripts/release/cuda-package-rehearsal-smoke.sh' "$REPO_ROOT/.github/workflows/release.yml"; then
  printf 'FAIL  Slice 20 self-hosted rehearsal must execute only trusted control-plane helpers\n' >&2
  exit 1
fi
printf 'PASS  Slice 20 self-hosted rehearsal executes only trusted control-plane helpers\n'

# Slice 80.7: the Tegra wrapper stages only metadata, stamps +tegra, proves
# both wheel surfaces, and prints the concrete install command after success.
python3 - "$REPO_ROOT/scripts/release/build-python-cuda-tegra.sh" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text()
required = (
    'TEGRA_LOCAL_VERSION="${BASE_VERSION}+tegra"',
    'mktemp -d',
    'Version: ${TEGRA_LOCAL_VERSION}',
    'python -m pip install $WHEEL',
    "printf '%q' \"$WHEEL\"",
)
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"Slice 80.7 Tegra metadata/install contract missing: {missing}")
PY
printf 'PASS  Slice 80.7 Tegra wrapper stamps and proves local-version metadata\n'

SPACE_WHEEL='/tmp/fathomdb wheel;literal.whl'
QUOTED_WHEEL="$(printf '%q' "$SPACE_WHEEL")"
ROUND_TRIP="$(bash -c "set -- python -m pip install $QUOTED_WHEEL; printf '%s' \"\$5\"")"
if [ "$ROUND_TRIP" = "$SPACE_WHEEL" ]; then
  printf 'PASS  Slice 80.7 final wheel-install command preserves a space/metacharacter path\n'
else
  printf 'FAIL  Slice 80.7 final wheel-install command did not preserve a quoted wheel path\n' >&2
  exit 1
fi

if grep -Fq 'exec env -i PATH=/opt/python/cp311-cp311/bin:/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache FATHOMDB_EMBED_DEVICE=cuda:0' \
  "$REPO_ROOT/scripts/release/cuda-package-rehearsal-smoke.sh" \
  && grep -Fq 'exec env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp/unavailable HF_HOME=/fathomdb-hf XDG_CACHE_HOME=/fathomdb-product-cache FATHOMDB_EMBED_DEVICE=cuda:0 node' \
  "$REPO_ROOT/scripts/release/cuda-package-rehearsal-smoke.sh"; then
  printf 'PASS  Slice 20 GPU PID attestations exec the actual Python and Node runtimes\n'
else
  printf 'FAIL  Slice 20 GPU PID attestations must exec the actual Python and Node runtimes\n' >&2
  exit 1
fi

python_cpu_smoke="$(sed -n '/docker run --rm --network none/,/write_cpu python/p' "$REPO_ROOT/scripts/release/cuda-package-rehearsal-smoke.sh")"
if grep -Fq 'test ! -e /dev/nvidiactl' <<<"$python_cpu_smoke"; then
  printf 'PASS  Slice 20 Python CPU smoke proves driverless execution\n'
else
  printf 'FAIL  Slice 20 Python CPU smoke must prove /dev/nvidiactl is absent\n' >&2
  exit 1
fi

for package_rehearsal_mutation in missing-gate source-smoke host-network; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/.github/workflows/release.yml" "$FIXTURE/scripts/release/cuda-package-rehearsal-smoke.sh" "$package_rehearsal_mutation" <<'PY'
from pathlib import Path
import sys

workflow = Path(sys.argv[1])
smoke = Path(sys.argv[2])
mutation = sys.argv[3]
if mutation == "missing-gate":
    text = workflow.read_text()
    needle = "      - cuda-package-rehearsal\n"
    if text.count(needle) != 1:
        raise SystemExit("fixture lacks the aggregate CUDA package rehearsal gate")
    workflow.write_text(text.replace(needle, "", 1))
elif mutation == "source-smoke":
    text = smoke.read_text()
    needle = "# never mounted; env -i"
    if needle not in text:
        raise SystemExit("fixture lacks source-isolation marker")
    smoke.write_text(text.replace(needle, "--mount type=bind,src=$PWD,dst=/source ", 1))
elif mutation == "host-network":
    text = smoke.read_text()
    needle = "docker run --rm --network none"
    if text.count(needle) < 2:
        raise SystemExit("fixture lacks isolated container smoke")
    smoke.write_text(text.replace(needle, "docker run --rm --network host", 1))
else:
    raise SystemExit("unknown mutation")
PY
  expect_fail "$FIXTURE" "rejects CUDA package rehearsal mutation: $package_rehearsal_mutation"
done

make_fixture "$FIXTURE"
python3 - "$FIXTURE/dev/release/cuda-unmerged-candidates.json" <<'PY'
import json
import sys

path = sys.argv[1]
record = {
    "schema_version": "fathomdb.cuda-unmerged-candidate/v1",
    "candidate_sha": "0123456789abcdef0123456789abcdef01234567",
    "candidate_pr": 228,
    "candidate_pr_head_sha": "0123456789abcdef0123456789abcdef01234567",
    "required_reviewers": ["independent-reviewer"],
    "expires_at": "2999-01-01T00:00:00Z",
    "purpose": "0.8.23 non-publishing CUDA preflight",
    "provenance_pr": 229,
    "provenance_head_sha": "2222222222222222222222222222222222222222",
    "provenance_commit": "1111111111111111111111111111111111111111",
    "provenance_required_reviewers": ["independent-provenance-reviewer"],
}
value = {"schema_version": "fathomdb.cuda-unmerged-candidates/v1", "candidates": [record]}
open(path, "w", encoding="utf-8").write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
PY
expect_pass "$FIXTURE" 'accepts one canonical future unmerged candidate record without authorizing it'

for manifest_mutation in multiple expired noncanonical; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/dev/release/cuda-unmerged-candidates.json" "$manifest_mutation" <<'PY'
import json
import sys

path, mutation = sys.argv[1:]
record = {
    "schema_version": "fathomdb.cuda-unmerged-candidate/v1",
    "candidate_sha": "0123456789abcdef0123456789abcdef01234567",
    "candidate_pr": 228,
    "candidate_pr_head_sha": "0123456789abcdef0123456789abcdef01234567",
    "required_reviewers": ["independent-reviewer"],
    "expires_at": "2999-01-01T00:00:00Z",
    "purpose": "0.8.23 non-publishing CUDA preflight",
    "provenance_pr": 229,
    "provenance_head_sha": "2222222222222222222222222222222222222222",
    "provenance_commit": "1111111111111111111111111111111111111111",
    "provenance_required_reviewers": ["independent-provenance-reviewer"],
}
if mutation == "multiple":
    candidates = [record, dict(record, candidate_sha="89abcdef0123456789abcdef0123456789abcdef", candidate_pr_head_sha="89abcdef0123456789abcdef0123456789abcdef")]
elif mutation == "expired":
    candidates = [dict(record, expires_at="2000-01-01T00:00:00Z")]
elif mutation == "noncanonical":
    candidates = [record]
else:
    raise SystemExit("unknown mutation")
value = {"schema_version": "fathomdb.cuda-unmerged-candidates/v1", "candidates": candidates}
if mutation == "noncanonical":
    rendered = json.dumps(value, indent=2, sort_keys=True) + "\n"
else:
    rendered = json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
open(path, "w", encoding="utf-8").write(rendered)
PY
  expect_fail "$FIXTURE" "rejects a $manifest_mutation unmerged candidate manifest"
done

make_fixture "$FIXTURE"
printf 'exit 0\n' > "$FIXTURE/scripts/verify-release-gates.sh"
assert_unmerged_control_plane "$FIXTURE"
printf 'PASS  candidate release-gate mutation cannot supply CUDA eligibility\n'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '-e "LIBRARY_PATH=$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one CUDA runtime linker search path")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA build without image-owned runtime link search paths'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = " -e FATHOMDB_GPU_ALLOCATION_WITNESS=1"
if text.count(needle) != 2:
    raise SystemExit("fixture no longer contains both CUDA allocation-witness opt-ins")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects CUDA preflight without both in-process allocation witnesses'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" "$FIXTURE/scripts/release/cuda-package-rehearsal-smoke.sh" <<'PY'
from pathlib import Path
import sys

preflight, rehearsal = map(Path, sys.argv[1:])
for path in (preflight, rehearsal):
    text = path.read_text()
    if "FATHOMDB_CUDA_GPU_UUID" not in text:
        raise SystemExit(f"{path.name} does not require an environment-owned GPU UUID")
    if 'device=$CUDA_GPU_UUID' not in text:
        raise SystemExit(f"{path.name} does not pass the exact GPU UUID to Docker")
    if "device=0" in text:
        raise SystemExit(f"{path.name} still pins CUDA evidence to a mutable host index")
preflight.write_text(preflight.read_text().replace('device=$CUDA_GPU_UUID', 'device=0', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight whose Docker GPU selector is downgraded to host index zero'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '    env:\n      FATHOMDB_CUDA_GPU_UUID: ${{ vars.FATHOMDB_CUDA_GPU_UUID }}\n'
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains the three environment-owned CUDA GPU UUID bindings")
path.write_text(text.replace(needle, '', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA job without its environment-owned GPU UUID binding'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/build-napi-cuda.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'export LIBRARY_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64${LIBRARY_PATH:+:$LIBRARY_PATH}"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one N-API CUDA runtime linker search path")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects an N-API CUDA build without a toolkit runtime link search path'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/Cargo.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'candle-nn-fathomdb = { git = "https://github.com/coreyt/candle-fathomdb.git", rev = "5719d90e60edd14c4c1a3bf87952648131b2153a" }\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one Candle NN source pin")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a split Candle source selection'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/Cargo.lock" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'source = "git+https://github.com/coreyt/candle-fathomdb.git?rev=5719d90e60edd14c4c1a3bf87952648131b2153a#5719d90e60edd14c4c1a3bf87952648131b2153a"'
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains all three immutable Candle lock sources")
path.write_text(text.replace(needle, 'source = "registry+https://github.com/rust-lang/crates.io-index"', 1))
PY
expect_fail "$FIXTURE" 'rejects a Candle lockfile that falls back to crates.io'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/src/rust/crates/fathomdb-napi/Cargo.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'embed-cuda = ["default-embedder", "fathomdb-engine/embed-cuda"]\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one N-API CUDA forwarding feature")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a N-API binary without CUDA feature forwarding'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '    runs-on: [self-hosted, Linux, X64, gpu, cuda-12]\n'
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains all restricted CUDA runner selections")
path.write_text(text.replace(needle, '    runs-on: ubuntu-latest\n', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight moved onto ordinary CI'

for least_privilege_mutation in candidate-write candidate-credentials publishing-reach; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/.github/workflows/release.yml" "$least_privilege_mutation" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
mutation = sys.argv[2]
text = path.read_text()
if mutation == "candidate-write":
    needle = "  verify-release:\n    needs: verify-cuda-trusted-route\n"
    replacement = needle + "    permissions:\n      contents: write\n      id-token: write\n"
elif mutation == "candidate-credentials":
    start = text.index("  build-python:\n")
    end = text.index("  build-napi:\n", start)
    job = text[start:end]
    needle = "          ref: ${{ env.RELEASE_CHECKOUT_REF }}\n"
    replacement = needle + "          persist-credentials: true\n"
    if job.count(needle) != 1:
        raise SystemExit("fixture lacks the build-python candidate checkout")
    path.write_text(text[:start] + job.replace(needle, replacement, 1) + text[end:])
    raise SystemExit(0)
elif mutation == "publishing-reach":
    needle = "  publish-rust-t1-embedder-api:\n"
    replacement = needle + "    if: ${{ true }}\n"
else:
    raise SystemExit("unknown least-privilege mutation")
if text.count(needle) != 1:
    raise SystemExit(f"fixture lacks {mutation} mutation target: {needle!r}")
path.write_text(text.replace(needle, replacement, 1))
PY
  expect_fail "$FIXTURE" "rejects unmerged candidate least-privilege mutation: $least_privilege_mutation"
done

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '    environment: cuda-unmerged-preflight\n'
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains all exact protected CUDA environments")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects removal of the protected unmerged-candidate environment'

for environment_mutation in comment-only substituted mapping; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/.github/workflows/release.yml" "$environment_mutation" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
mutation = sys.argv[2]
text = path.read_text()
needle = "    environment: cuda-unmerged-preflight\n"
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains all exact protected CUDA environments")
replacements = {
    "comment-only": "    # environment: cuda-unmerged-preflight\n",
    "substituted": "    environment: ${{ inputs.cuda_environment }} # environment: cuda-unmerged-preflight\n",
    "mapping": "    environment:\n      name: cuda-unmerged-preflight # environment: cuda-unmerged-preflight\n",
}
path.write_text(text.replace(needle, replacements[mutation], 1))
PY
  expect_fail "$FIXTURE" "rejects a $environment_mutation protected environment lookalike"
done

for control_plane_mutation in remove reorder; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/.github/workflows/release.yml" "$control_plane_mutation" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
mutation = sys.argv[2]
text = path.read_text()
checkout_marker = (
    "      - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 "
    "# v7.0.0, main-owned receipt verifier only\n"
)
receipt_marker = "      - name: Verify same-run unmerged route receipt before candidate checkout\n"
candidate_marker = (
    "      - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 "
    "# v7.0.0, hosted route and receipt verified\n"
)
checkout_start = text.index(checkout_marker)
receipt_start = text.index(receipt_marker, checkout_start)
candidate_start = text.index(candidate_marker, receipt_start)
checkout = text[checkout_start:receipt_start]
receipt = text[receipt_start:candidate_start]
if mutation == "remove":
    replacement = receipt
elif mutation == "reorder":
    replacement = receipt + checkout
else:
    raise SystemExit("unknown mutation")
path.write_text(text[:checkout_start] + replacement + text[candidate_start:])
PY
  expect_fail "$FIXTURE" "rejects a $control_plane_mutation control-plane checkout before receipt verification"
done

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import re
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = (
    "  cuda-contract-preflight:\n"
    "    needs: [verify-release, verify-cuda-trusted-route]\n"
    "    if: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run == true }}\n"
)
replacement = (
    "  cuda-contract-preflight:\n"
    "    needs: [verify-release, verify-cuda-trusted-route]\n"
    "    if: ${{ github.event_name == 'workflow_dispatch' }}\n"
)
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains the CUDA job-level dry-run-only guard")
path.write_text(text.replace(needle, replacement, 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight that could run on a publishing dispatch'

for route_mutation in hosted-dependency unsafe-checkout; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/.github/workflows/release.yml" "$route_mutation" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
mutation = sys.argv[2]
text = path.read_text()
if mutation == "hosted-dependency":
    needle = "  cuda-contract-preflight:\n    needs: [verify-release, verify-cuda-trusted-route]\n"
    replacement = "  cuda-contract-preflight:\n    needs: verify-release\n"
elif mutation == "unsafe-checkout":
    needle = "          ref: ${{ github.workflow_sha }}\n          fetch-depth: 1\n          persist-credentials: false"
    replacement = "          ref: ${{ github.workflow_sha }}\n          fetch-depth: 1\n          persist-credentials: true"
else:
    raise SystemExit(f"unknown mutation {mutation}")
if text.count(needle) != 1:
    raise SystemExit(f"fixture lacks {mutation} mutation target: {needle!r}")
path.write_text(text.replace(needle, replacement, 1))
PY
  if assert_unmerged_control_plane "$FIXTURE"; then
    printf 'FAIL  rejects a CUDA preflight route with %s\n' "$route_mutation" >&2
    exit 1
  fi
  printf 'PASS  rejects a CUDA preflight route with %s\n' "$route_mutation"
done

for unsafe_step in \
  'run: ./candidate-gate' \
  'run: bash candidate-gate' \
  'uses: owner/action@ref'; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/.github/workflows/release.yml" "$unsafe_step" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
unsafe_step = sys.argv[2]
text = path.read_text()
needle = "\n  # Slice 0: a dry-run release dispatch from the trusted main workflow is the\n"
if text.count(needle) != 1:
    raise SystemExit("fixture lacks CUDA preflight boundary for hosted-verifier mutation")
step = "\n      - name: Candidate-controlled mutation\n        " + unsafe_step + "\n"
path.write_text(text.replace(needle, step + needle, 1))
PY
  expect_fail "$FIXTURE" "rejects an unsafe hosted-verifier step: $unsafe_step"
done

for device_variable in \
  FATHOMDB_EMBED_DEVICE \
  FATHOMDB_RERANK_DEVICE \
  CUDA_VISIBLE_DEVICES \
  NVIDIA_VISIBLE_DEVICES \
  HIP_VISIBLE_DEVICES \
  ROCR_VISIBLE_DEVICES; do
  make_fixture "$FIXTURE"
  python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" "$device_variable" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
variable = sys.argv[2]
text = path.read_text()
needle = (
    "env -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE -u CUDA_VISIBLE_DEVICES "
    "-u NVIDIA_VISIBLE_DEVICES -u HIP_VISIBLE_DEVICES -u ROCR_VISIBLE_DEVICES "
    "-u HUGGINGFACE_HUB_CACHE -u TRANSFORMERS_CACHE -u FATHOMDB_EMBEDDER_CACHE_DIR sh -ceu '"
)
if text.count(needle) != 2:
    raise SystemExit("fixture no longer contains exactly two driverless device scrubs")
path.write_text(text.replace(needle, f"{needle[:-8]} {variable}=injected sh -ceu '", 1))
PY
  expect_fail "$FIXTURE" "rejects $device_variable injected after the driverless scrub"
done

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '--network none'
if text.count(needle) < 2:
    raise SystemExit("fixture no longer contains both network-isolated driverless smokes")
path.write_text(text.replace(needle, '--network host', 1))
PY
expect_fail "$FIXTURE" 'rejects either driverless smoke without network isolation'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'Engine.open(str(pathlib.Path(directory) / "driverless.fdb"), use_default_embedder=True)'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one driverless Python open")
path.write_text(text.replace(needle, 'Engine.open(str(pathlib.Path(directory) / "driverless.fdb"), use_default_embedder=False)', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless Python smoke that skips the default embedder'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'Engine.open("/fathomdb-tmp/driverless-node.fdb", { useDefaultEmbedder: true })'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one driverless N-API default-embedder open")
path.write_text(text.replace(needle, 'Engine.open("/fathomdb-tmp/driverless-node.fdb", { useDefaultEmbedder: false })', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless installed N-API smoke that skips the default embedder'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '--mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly"'
if text.count(needle) < 2:
    raise SystemExit("fixture no longer mounts the local default-embedder mirror for both smokes")
path.write_text(text.replace(needle, '--mount "type=bind,src=/missing-model-cache,dst=/fathomdb-hf,readonly"', 1))
PY
expect_fail "$FIXTURE" 'rejects a driverless artifact smoke without the pinned local model mirror'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = '    runs-on: [self-hosted, Linux, X64, gpu, cuda-12]\n'
if text.count(needle) != 3:
    raise SystemExit("fixture no longer contains all CUDA runner selections")
path.write_text(text.replace(needle, needle + '    permissions:\n      contents: write\n', 1))
PY
expect_fail "$FIXTURE" 'rejects CUDA preflight permissions broader than read-only'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/.github/workflows/release.yml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a'
if text.count(needle) != 8:
    raise SystemExit("fixture CUDA artifact uploads must use the reviewed full action SHA")
path.write_text(text.replace(needle, needle[:-1], 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA witness uploader with a shortened action SHA'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/Dockerfile.cuda-manylinux" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "@sha256:"
if needle not in text:
    raise SystemExit("fixture Dockerfile no longer pins its base image by digest")
path.write_text(text.replace(needle, ":", 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA manylinux Dockerfile with a mutable base image'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/provision-cuda-manylinux.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'sha256sum --check --status'
if text.count(needle) != 1:
    raise SystemExit("fixture provisioner no longer verifies exactly one pinned cache manifest")
path.write_text(text.replace(needle, 'true', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA provisioner that accepts an unchecked model cache'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-preflight.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'sha256sum --check --status'
if text.count(needle) != 2:
    raise SystemExit("fixture preflight no longer verifies both pinned cache manifests")
path.write_text(text.replace(needle, 'true', 1))
PY
expect_fail "$FIXTURE" 'rejects a CUDA preflight that accepts an unchecked model cache'

# --- 0.8.23 Slice 80.6 (D-80.6-1/2/4, AC80-8): Tegra toolchain axis ---------
# The x86_64 arms above are unchanged; these add the Tegra target's own.

# The Tegra wrapper is covered by the SC2312 ratchet.  Keep the wheel
# discovery path free of process-substitution return masking: a failed
# discovery must not be converted into an empty wheel list.
if ! shellcheck --severity=style --include=SC2312 \
  "$REPO_ROOT/scripts/release/build-python-cuda-tegra.sh" >/dev/null; then
  printf '%s\n' 'FAIL  rejects a Tegra CUDA build wrapper with an SC2312 masked-return finding' >&2
  exit 1
fi
printf '%s\n' 'PASS  accepts a Tegra CUDA build wrapper without SC2312 masked-return findings'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/build-python-cuda-tegra.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'export LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LIBRARY_PATH:+:$LIBRARY_PATH}"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer contains exactly one Tegra cudart link search path")
path.write_text(text.replace(needle, "", 1))
PY
expect_fail "$FIXTURE" 'rejects a Tegra CUDA build without the measured cudart link search path'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/build-python-cuda-tegra.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_TEGRA_ORIN"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer selects the Tegra compute capability exactly once")
path.write_text(text.replace(needle, 'export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_X86_64"\n', 1))
PY
expect_fail "$FIXTURE" 'rejects a Tegra CUDA build that selects the x86_64 compute capability'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/build-python-cuda-tegra.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
path.write_text(text + 'twine upload "$OUT_DIR"/*.whl\n')
PY
expect_fail "$FIXTURE" 'rejects a Tegra CUDA build wrapper that publishes (D-80.6-1)'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/build-python-cuda-tegra.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'TEGRA_FLOOR="$(glibc_floor_for_family tegra)"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture no longer resolves the Tegra glibc floor exactly once")
path.write_text(text.replace(needle, 'TEGRA_FLOOR="2.28"\n', 1))
PY
expect_fail "$FIXTURE" 'rejects a Tegra CUDA build that hard-codes its glibc floor instead of declaring it (D-80.6-5)'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = "export CUDA_HOST_GCC_VERSION_TEGRA_ORIN='11.4.0'\n"
if text.count(needle) != 1:
    raise SystemExit("fixture contract no longer pins the Tegra host gcc exactly once")
path.write_text(text.replace(needle, "export CUDA_HOST_GCC_VERSION_TEGRA_ORIN='13.3.0'\n", 1))
PY
expect_fail "$FIXTURE" 'rejects a contract that re-points the Tegra host gcc at the x86_64 pin'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'export CUDA_TEGRA_HOST_NVCC_VERSION="$CUDA_NAPI_HOST_NVCC_VERSION"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture contract no longer shares the nvcc pin by reference")
replacement = (
    "export CUDA_TEGRA_HOST_NVCC_VERSION="
    "'Cuda compilation tools, release 12.6, V12.6.68'\n"
)
path.write_text(text.replace(needle, replacement, 1))
PY
expect_fail "$FIXTURE" 'rejects a second nvcc literal split off from the shared x86_64 pin'

make_fixture "$FIXTURE"
python3 - "$FIXTURE/scripts/release/cuda-artifact-contract.sh" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
needle = 'export CUDA_NAPI_HOST_CC="$CUDA_HOST_CC_X86_64"\n'
if text.count(needle) != 1:
    raise SystemExit("fixture contract no longer selects the N-API host CC from the x86_64 axis")
path.write_text(text.replace(needle, 'export CUDA_NAPI_HOST_CC="$CUDA_HOST_CC_TEGRA_ORIN"\n', 1))
PY
expect_fail "$FIXTURE" 'rejects an N-API host CC silently re-pointed at the Tegra axis'

printf '\nCUDA release-contract tests passed\n'
