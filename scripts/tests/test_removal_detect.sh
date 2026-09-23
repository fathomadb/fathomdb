#!/usr/bin/env bash
# Self-test for AC-050c removal-detect linter
# (`scripts/security/check_removal_changelog.py`). Drives positive,
# negative, and same-name-move fixtures.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LINT="$REPO_ROOT/scripts/security/check_removal_changelog.py"
FIX="$SCRIPT_DIR/fixtures/removal-detect"

fail() { echo "FAIL: $*" >&2; exit 1; }

# Positive: every removal documented → exit 0.
if ! python3 "$LINT" \
    --diff-file "$FIX/clean/diff.patch" \
    --changelog "$FIX/clean/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "clean fixture: linter must exit 0 (every removal documented)"
fi
echo "OK clean"

# Negative: undocumented removal → exit 1.
set +e
python3 "$LINT" \
    --diff-file "$FIX/undocumented/diff.patch" \
    --changelog "$FIX/undocumented/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null 2>/tmp/removal_detect_negative.err
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "undocumented fixture: linter must exit 1, got $rc"
fi
if ! grep -q "secret_unannounced" /tmp/removal_detect_negative.err; then
    fail "undocumented fixture: diagnostic must name the undocumented symbol"
fi
echo "OK undocumented"

# Move-within-file: same symbol name re-emerges → not a removal.
if ! python3 "$LINT" \
    --diff-file "$FIX/moved-in-file/diff.patch" \
    --changelog "$FIX/moved-in-file/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "moved-in-file fixture: linter must exit 0 (alpha re-appears in same file)"
fi
echo "OK moved-in-file"

# Move-to-private-module: public re-exports preserve the original crate-root
# names, so the implementation move is not a consumer-visible removal.
if ! python3 "$LINT" \
    --diff-file "$FIX/public-reexport/diff.patch" \
    --changelog "$FIX/public-reexport/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "public-reexport fixture: linter must exit 0 (crate-root paths are preserved)"
fi
echo "OK public-reexport"

# Explicit re-export names cancel removals only when the same public name is
# retained. Globs, crate-private imports, renamed exports, interrupted
# statement state, and cfg-gated re-exports (C-1: a `#[cfg(...)]`-gated
# `pub use` does not unconditionally restore the removed public name) all
# stay fail-closed.
for case_name in glob private renamed state-poison cfg-gated; do
    set +e
    python3 "$LINT" \
        --diff-file "$FIX/reexport-edges/$case_name.patch" \
        --changelog "$FIX/reexport-edges/CHANGELOG.md" \
        --repo-root "$REPO_ROOT" \
        >/dev/null 2>"/tmp/removal_detect_reexport_${case_name}.err"
    rc=$?
    set -e
    if [ "$rc" -ne 1 ]; then
        fail "$case_name re-export fixture: expected undocumented removal exit 1, got $rc"
    fi
    if ! grep -q "Foo" "/tmp/removal_detect_reexport_${case_name}.err"; then
        fail "$case_name re-export fixture: diagnostic must retain removed Foo"
    fi
done
echo "OK fail-closed reexports"

# An explicit alias that retains the old crate-root name is equivalent to a
# direct public re-export and therefore cancels the removal.
if ! python3 "$LINT" \
    --diff-file "$FIX/reexport-edges/alias-preserved.patch" \
    --changelog "$FIX/reexport-edges/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "alias-preserved fixture: explicit exported name Foo must cancel removal"
fi
echo "OK alias-preserved"

# C-2: a bare removed `pub use` (no replacement) must itself be recorded as a
# removal — probe regression for `-pub use errors::EngineError;` -> `[]`.
set +e
python3 "$LINT" \
    --diff-file "$FIX/pub-use-removed/diff.patch" \
    --changelog "$FIX/pub-use-removed/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null 2>/tmp/removal_detect_pub_use_removed.err
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "pub-use-removed fixture: linter must exit 1 (removed pub use is a removal), got $rc"
fi
if ! grep -q "EngineError" /tmp/removal_detect_pub_use_removed.err; then
    fail "pub-use-removed fixture: diagnostic must name the removed re-export EngineError"
fi
echo "OK pub-use-removed"

# C-2: a re-export moved from one `pub use` list to another unconditional one
# in the same file still cancels (same-name-move semantics extend to pub use
# removals, not just additions).
if ! python3 "$LINT" \
    --diff-file "$FIX/pub-use-moved/diff.patch" \
    --changelog "$FIX/pub-use-moved/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "pub-use-moved fixture: linter must exit 0 (EngineError re-export moved, name preserved)"
fi
echo "OK pub-use-moved"

# tests/-excluded: removals under any `tests/` directory are NOT public API and
# must NOT require a CHANGELOG entry → exit 0 even with an empty Removed section.
# (Slice 27 fix-1: the scanner scopes `tests/` out so test-function churn — e.g.
# the Slice-25 test_surface.py rewrite — never trips the gate.)
if ! python3 "$LINT" \
    --diff-file "$FIX/tests-excluded/diff.patch" \
    --changelog "$FIX/tests-excluded/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "tests-excluded fixture: linter must exit 0 (tests/ removals are not public API)"
fi
echo "OK tests-excluded"

# default-base-ref live-git path — exercises the default --base argument
# against live git history (no --diff-file). Catches the B-001 regression
# where the default base-ref was "0.6.0-rewrite" (a closed branch removed
# at 0.6.0 GA), causing `fatal: bad revision '0.6.0-rewrite..HEAD'`.
# See: dev/plans/runs/0.6.1-planning-output.json § blockers_encountered B-001.
set +e
stderr_out="$(bash "$REPO_ROOT/scripts/security/check-removal-changelog.sh" 2>&1 >/dev/null)"
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
    fail "default-base-ref live-git path: expected exit 0, got $rc (stderr: $stderr_out)"
fi
if echo "$stderr_out" | grep -q "fatal: bad revision"; then
    fail "default-base-ref live-git path: got 'fatal: bad revision' in stderr — default base-ref is broken"
fi
echo "OK default-base-ref live-git path"

# V-1 (fix-1): the added-side cfg-pending tracker must survive stacked
# attribute lines, not just a single-line `#[cfg(...)]` immediately above
# the `pub use` it guards — a multi-line (rustfmt-wrapped) attribute, and
# doc-comment / line-comment / blank lines interposed between the cfg
# attribute and the `pub use`, must all keep the gate pending so the
# `pub use` stays fail-closed (does not cancel the removal it's re-exporting
# under a cfg predicate).
for case_name in multiline-attr doc-comment line-comment blank-line; do
    set +e
    python3 "$LINT" \
        --diff-file "$FIX/cfg-pending/$case_name.patch" \
        --changelog "$FIX/cfg-pending/CHANGELOG.md" \
        --repo-root "$REPO_ROOT" \
        >/dev/null 2>"/tmp/removal_detect_cfg_pending_${case_name}.err"
    rc=$?
    set -e
    if [ "$rc" -ne 1 ]; then
        fail "$case_name cfg-pending fixture: expected undocumented removal exit 1, got $rc"
    fi
    if ! grep -q "Foo" "/tmp/removal_detect_cfg_pending_${case_name}.err"; then
        fail "$case_name cfg-pending fixture: diagnostic must retain removed Foo"
    fi
done
echo "OK cfg-pending (fail-closed through stacked/multi-line attributes)"

# V-1 companion: a cfg attribute on an UNRELATED item must still reset the
# pending gate (existing behavior) so it doesn't leak onto a later
# unconditional `pub use` of the actually-removed symbol.
if ! python3 "$LINT" \
    --diff-file "$FIX/cfg-pending/unrelated-item-resets.patch" \
    --changelog "$FIX/cfg-pending/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null; then
    fail "unrelated-item-resets fixture: linter must exit 0 (cfg on Bar must not gate the later unconditional pub use of Foo)"
fi
echo "OK cfg-pending unrelated-item resets"

# V-2 (fix-2): the removed-side `pub use { ... }` accumulator only started
# on a REMOVED opening line and was wiped by any context line, so it missed
# the common rustfmt diff shape for dropping ONE name from an otherwise
# untouched multi-line re-export list. The old-side tracker reconstructs the
# pre-image from BOTH context and removed lines within a hunk.
if [ ! -f "$FIX/pub-use-block-partial/dropped-entry.patch" ]; then
    fail "pub-use-block-partial fixtures missing"
fi
set +e
python3 "$LINT" \
    --diff-file "$FIX/pub-use-block-partial/dropped-entry.patch" \
    --changelog "$FIX/pub-use-block-partial/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null 2>/tmp/removal_detect_pub_use_dropped_entry.err
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "dropped-entry fixture: linter must exit 1 (Foo dropped from context-only pub use block), got $rc"
fi
if ! grep -q "Foo" /tmp/removal_detect_pub_use_dropped_entry.err; then
    fail "dropped-entry fixture: diagnostic must name the dropped re-export Foo"
fi
echo "OK pub-use-block-partial dropped-entry"

set +e
python3 "$LINT" \
    --diff-file "$FIX/pub-use-block-partial/reopened-with-context.patch" \
    --changelog "$FIX/pub-use-block-partial/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null 2>/tmp/removal_detect_pub_use_reopened.err
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "reopened-with-context fixture: linter must exit 1 (Foo removed, block reopened+closed around a context line), got $rc"
fi
if ! grep -q "Foo" /tmp/removal_detect_pub_use_reopened.err; then
    fail "reopened-with-context fixture: diagnostic must name the removed re-export Foo"
fi
echo "OK pub-use-block-partial reopened-with-context"

echo "test_removal_detect.sh: all cases pass"
