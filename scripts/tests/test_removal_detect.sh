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

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

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
    >/dev/null 2>"$WORK/negative.err"
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "undocumented fixture: linter must exit 1, got $rc"
fi
if ! grep -q "secret_unannounced" "$WORK/negative.err"; then
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
        >/dev/null 2>"$WORK/reexport_${case_name}.err"
    rc=$?
    set -e
    if [ "$rc" -ne 1 ]; then
        fail "$case_name re-export fixture: expected undocumented removal exit 1, got $rc"
    fi
    if ! grep -q "Foo" "$WORK/reexport_${case_name}.err"; then
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
    >/dev/null 2>"$WORK/pub_use_removed.err"
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "pub-use-removed fixture: linter must exit 1 (removed pub use is a removal), got $rc"
fi
if ! grep -q "EngineError" "$WORK/pub_use_removed.err"; then
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
        >/dev/null 2>"$WORK/cfg_pending_${case_name}.err"
    rc=$?
    set -e
    if [ "$rc" -ne 1 ]; then
        fail "$case_name cfg-pending fixture: expected undocumented removal exit 1, got $rc"
    fi
    if ! grep -q "Foo" "$WORK/cfg_pending_${case_name}.err"; then
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
    >/dev/null 2>"$WORK/pub_use_dropped_entry.err"
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "dropped-entry fixture: linter must exit 1 (Foo dropped from context-only pub use block), got $rc"
fi
if ! grep -q "Foo" "$WORK/pub_use_dropped_entry.err"; then
    fail "dropped-entry fixture: diagnostic must name the dropped re-export Foo"
fi
echo "OK pub-use-block-partial dropped-entry"

set +e
python3 "$LINT" \
    --diff-file "$FIX/pub-use-block-partial/reopened-with-context.patch" \
    --changelog "$FIX/pub-use-block-partial/CHANGELOG.md" \
    --repo-root "$REPO_ROOT" \
    >/dev/null 2>"$WORK/pub_use_reopened.err"
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "reopened-with-context fixture: linter must exit 1 (Foo removed, block reopened+closed around a context line), got $rc"
fi
if ! grep -q "Foo" "$WORK/pub_use_reopened.err"; then
    fail "reopened-with-context fixture: diagnostic must name the removed re-export Foo"
fi
echo "OK pub-use-block-partial reopened-with-context"

# Per-side reconstruction: the exported `pub use` set is parsed from the
# whole old and new file text, so a name that stops being UNCONDITIONALLY
# exported is a removal regardless of which lines of the diff changed.
#   W-1: moved into an existing (context) cfg-gated block / below a context
#        cfg attribute.
#   W-2: dropped from deep inside a long block (full-context patch; the
#        live-git path is covered by the temp-repo case below).
#   W-3: only a cfg attribute added above an unchanged `pub use`, item, or
#        inline module whose contents then leave the default build; likewise
#        an item removed and re-added behind a cfg attribute.
w_case() {
    local case_name="$1" expected_name="$2"
    set +e
    python3 "$LINT" \
        --diff-file "$FIX/pub-use-full-context/$case_name.patch" \
        --changelog "$FIX/pub-use-full-context/CHANGELOG.md" \
        --repo-root "$REPO_ROOT" \
        >/dev/null 2>"$WORK/full_context_${case_name}.err"
    rc=$?
    set -e
    if [ "$rc" -ne 1 ]; then
        fail "$case_name fixture: expected undocumented removal exit 1, got $rc"
    fi
    for name in $expected_name; do
        if ! grep -qw "$name" "$WORK/full_context_${case_name}.err"; then
            fail "$case_name fixture: diagnostic must name removed $name"
        fi
    done
    echo "OK $case_name"
}
w_case w1-into-existing-cfg-block Foo
w_case w1-context-cfg-above-added Foo
w_case w2-long-block-full-context Foo
w_case w3-cfg-added-single Foo
w_case w3-cfg-added-group "Bar Baz"
w_case w3-cfg-added-item Foo
w_case w3-cfg-added-mod-block Foo
w_case item-made-conditional Foo

# Same-file cancellation still holds for the per-side parse: a name moved
# from a cfg-gated block into an unconditional one, or from a `pub use` to a
# direct public item, is still exported by the default build; a cfg-gated
# item moved within the file stays exactly as conditional as it was.
for case_name in moved-into-unconditional-block use-to-item item-gated-move; do
    if ! python3 "$LINT" \
        --diff-file "$FIX/pub-use-full-context/$case_name.patch" \
        --changelog "$FIX/pub-use-full-context/CHANGELOG.md" \
        --repo-root "$REPO_ROOT" \
        >/dev/null; then
        fail "$case_name fixture: linter must exit 0 (name still unconditionally exported)"
    fi
    echo "OK $case_name"
done

# W-2 live-git path: the real gate diffs git refs itself, so the fixture must
# prove that `load_diff` produces whole-file context — dropping one name from
# deep inside a long block must still be seen as leaving that block.
tmp_repo="$WORK/repo"
mkdir -p "$tmp_repo"
git -C "$tmp_repo" init -q
git -C "$tmp_repo" config user.email removal-detect@example.invalid
git -C "$tmp_repo" config user.name removal-detect
mkdir -p "$tmp_repo/src/rust/crates/example/src"
lib="$tmp_repo/src/rust/crates/example/src/lib.rs"
{
    echo "mod m;"
    echo "pub use m::{"
    for i in $(seq 1 10); do printf '    Name%02d,\n' "$i"; done
    echo "    Foo,"
    for i in $(seq 11 20); do printf '    Name%02d,\n' "$i"; done
    echo "};"
    echo "pub fn keep() {}"
} >"$lib"
cp "$FIX/pub-use-full-context/CHANGELOG.md" "$tmp_repo/CHANGELOG.md"
git -C "$tmp_repo" add -A
git -C "$tmp_repo" commit -q -m base
git -C "$tmp_repo" tag base
grep -v '^    Foo,$' "$lib" >"$lib.new"
cat "$lib.new" >"$lib"
rm "$lib.new"
git -C "$tmp_repo" commit -q -am drop-foo
set +e
python3 "$LINT" --repo-root "$tmp_repo" --base base --head HEAD \
    >/dev/null 2>"$WORK/live_git_w2.err"
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "live-git W-2: expected undocumented removal exit 1, got $rc"
fi
if ! grep -qw "Foo" "$WORK/live_git_w2.err"; then
    fail "live-git W-2: diagnostic must name removed Foo"
fi
echo "OK live-git W-2 (whole-file context)"

# X-1/X-2: Rust items are keyed by owner — the enclosing `impl` self type
# for associated items, the inline-module path for free items. Only an
# unrestricted, unconditional `pub` item on the old side can be removed, and
# an item that reappears unconditionally in the same crate (associated items
# in an `impl` of the same type) is a move, not a removal.
x_case() {
    local case_name="$1" expected_rc="$2" expected_name="${3:-}"
    set +e
    python3 "$LINT" \
        --diff-file "$FIX/cross-file/$case_name.patch" \
        --changelog "$FIX/cross-file/CHANGELOG.md" \
        --repo-root "$REPO_ROOT" \
        >/dev/null 2>"$WORK/cross_file_${case_name}.err"
    rc=$?
    set -e
    if [ "$rc" -ne "$expected_rc" ]; then
        fail "$case_name fixture: expected exit $expected_rc, got $rc ($(cat "$WORK/cross_file_${case_name}.err"))"
    fi
    if [ -n "$expected_name" ] && ! grep -qF "$expected_name" "$WORK/cross_file_${case_name}.err"; then
        fail "$case_name fixture: diagnostic must name $expected_name"
    fi
    echo "OK $case_name"
}
x_case x1-free-fn-moved-reexported 0
x_case x1-method-moved 0
x_case x1-pub-crate-removed 0
x_case x1-cfg-test-mod-removed 0
x_case x2-method-gated-collision 1 "B::as_str"
x_case x2-impl-block-gated-collision 1 "B::new"
x_case guard-method-deleted 1 "Engine::close"
x_case guard-method-moved-to-other-type 1 "Engine::close"

# X-3 live-git: renames are diffed as delete + add, so a file renamed into
# another crate is compared (its items leave the old crate) while a rename
# within the crate is a move.
git -C "$tmp_repo" checkout -q -b renames base
cat >"$tmp_repo/src/rust/crates/example/src/moved.rs" <<'RS'
pub struct Widget;
impl Widget {
    pub fn render(&self) -> u32 { 1 }
    pub fn resize(&self, width: u32) -> u32 { width }
}
pub fn widget_helper() -> Widget { Widget }
RS
git -C "$tmp_repo" add -A
git -C "$tmp_repo" commit -q -m add-moved
git -C "$tmp_repo" tag rename-base
git -C "$tmp_repo" mv src/rust/crates/example/src/moved.rs src/rust/crates/example/src/renamed.rs
git -C "$tmp_repo" commit -q -m rename-in-crate
set +e
python3 "$LINT" --repo-root "$tmp_repo" --base rename-base --head HEAD \
    >/dev/null 2>"$WORK/live_git_rename_in_crate.err"
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
    fail "live-git X-3 in-crate rename: expected exit 0, got $rc ($(cat "$WORK/live_git_rename_in_crate.err"))"
fi
echo "OK live-git X-3 in-crate rename"
git -C "$tmp_repo" checkout -q -b cross-crate rename-base
mkdir -p "$tmp_repo/src/rust/crates/other/src"
git -C "$tmp_repo" mv src/rust/crates/example/src/moved.rs src/rust/crates/other/src/moved.rs
git -C "$tmp_repo" commit -q -m rename-cross-crate
set +e
python3 "$LINT" --repo-root "$tmp_repo" --base rename-base --head HEAD \
    >/dev/null 2>"$WORK/live_git_rename_cross_crate.err"
rc=$?
set -e
if [ "$rc" -ne 1 ]; then
    fail "live-git X-3 cross-crate rename: expected exit 1, got $rc"
fi
if ! grep -qF "Widget::render" "$WORK/live_git_rename_cross_crate.err"; then
    fail "live-git X-3 cross-crate rename: diagnostic must name Widget::render"
fi
echo "OK live-git X-3 cross-crate rename"

echo "test_removal_detect.sh: all cases pass"
