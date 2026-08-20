#!/usr/bin/env bash
# 0.8.23 Slice 80.3: one legible diagnostic for a bootstrapped developer
# environment, instead of the pinned-tool gap surfacing as N scattered,
# differently-worded failures across unrelated suites (shellcheck, gitleaks,
# actionlint, ruff, pyright, js-yaml each fail their OWN gate with their OWN
# message when missing). This suite names every tool that is missing or at
# the wrong pinned version in one place, before the rest of the fast tier
# runs into them piecemeal.
#
# Missing required developer tooling remains a failed gate. Gitleaks is a
# report-only security input until historical findings are remediated offline.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/shellcheck-version.sh
. "$REPO_ROOT/scripts/lib/shellcheck-version.sh"
# shellcheck source=../lib/gitleaks-version.sh
. "$REPO_ROOT/scripts/lib/gitleaks-version.sh"
# shellcheck source=../lib/actionlint-version.sh
. "$REPO_ROOT/scripts/lib/actionlint-version.sh"

cd "$REPO_ROOT" || exit 1

# Mirrors scripts/bootstrap.sh's own pin (kept local there too — not yet
# centralized into a lib/ constant the way shellcheck/gitleaks are).
ACTIONLINT_VERSION="1.7.12"

missing=()
warnings=()

check() {
  local label="$1" found="$2" want="$3"
  if [ -z "$found" ]; then
    missing+=("$label: not found (want $want)")
  elif [ "$found" != "$want" ]; then
    missing+=("$label: found $found, want $want")
  else
    printf 'ok  %s %s\n' "$label" "$found"
  fi
}

check_advisory() {
  local label="$1" found="$2" want="$3"
  if [ -z "$found" ]; then
    warnings+=("$label: not found (want $want)")
  elif [ "$found" != "$want" ]; then
    warnings+=("$label: found $found, want $want")
  else
    printf 'ok  %s %s\n' "$label" "$found"
  fi
}

# --- shellcheck ---
sc_bin="$(find_shellcheck_bin || true)"
sc_version=""
[ -n "$sc_bin" ] && sc_version="$(read_shellcheck_version "$sc_bin" || true)"
check shellcheck "$sc_version" "$SHELLCHECK_VERSION"

# --- gitleaks ---
gl_bin="$(find_gitleaks_bin || true)"
gl_version=""
[ -n "$gl_bin" ] && gl_version="$(read_gitleaks_version "$gl_bin" || true)"
check_advisory gitleaks "$gl_version" "$GITLEAKS_VERSION"

# --- actionlint ---
al_bin="$(find_actionlint_bin || true)"
al_version=""
[ -n "$al_bin" ] && al_version="$(read_actionlint_version "$al_bin" || true)"
check actionlint "$al_version" "$ACTIONLINT_VERSION"

# ruff / pyright — from the checkout-owned .venv, matching src/python/pyproject.toml's
# [project.optional-dependencies] pins (the single source of truth for these two).
RUFF_VERSION="$(grep -m1 -E '"ruff==' src/python/pyproject.toml | sed -n 's/.*"ruff==\([0-9.]*\)".*/\1/p')"
PYRIGHT_VERSION="$(grep -m1 -E '"pyright==' src/python/pyproject.toml | sed -n 's/.*"pyright==\([0-9.]*\)".*/\1/p')"
ruff_version=""
[ -x .venv/bin/ruff ] && ruff_version="$(.venv/bin/ruff --version 2>/dev/null | sed -n 's/^ruff //p')"
check ruff "$ruff_version" "$RUFF_VERSION"
pyright_version=""
[ -x .venv/bin/pyright ] && pyright_version="$(.venv/bin/pyright --version 2>/dev/null | sed -n 's/^pyright //p')"
check pyright "$pyright_version" "$PYRIGHT_VERSION"

# js-yaml — a transitive dep (via markdownlint-cli2) resolved at repo-root
# node_modules, not a direct dependency; presence is what several CI-shape
# checkers (test-ci-verify-observability, test-ci-rust-workspace-gate) need.
if [ -d node_modules/js-yaml ]; then
  printf 'ok  js-yaml resolvable at repo root\n'
else
  missing+=("js-yaml: not found at repo-root node_modules/ (run npm ci at repo root)")
fi

if [ "${#missing[@]}" -gt 0 ]; then
  printf 'FAIL  dev-environment-tools: %d pinned tool(s) missing or wrong version:\n' "${#missing[@]}" >&2
  for m in "${missing[@]}"; do
    printf '  - %s\n' "$m" >&2
  done
  printf '  Run scripts/bootstrap.sh (shellcheck/gitleaks/actionlint auto-install; ruff/pyright\n' >&2
  printf '  via pip install -e src/python[dev] from a non-worktree checkout; js-yaml via\n' >&2
  printf '  npm ci at repo root).\n' >&2
  exit 1
fi

for warning in "${warnings[@]}"; do
  printf 'WARN  %s (security scan remains report-only)\n' "$warning" >&2
done

echo "dev-environment-tools: required pinned tools present"
