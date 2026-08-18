#!/usr/bin/env bash
# Select the checkout-owned Python virtualenv for shell-suite subprocesses.

# Usage: use_checkout_venv_python_path <checkout-root>
#
# `scripts/bootstrap.sh` installs Python developer dependencies into the
# checkout-local `.venv`. Shell suites deliberately invoke `python3` directly,
# so make that interpreter visible before the test harness registers any suite.
# A missing or incomplete venv leaves PATH untouched, preserving the existing
# system-Python fallback without mutating or rebinding any virtualenv.
use_checkout_venv_python_path() {
  local checkout_root="$1"
  local venv_bin="$checkout_root/.venv/bin"

  if [ -x "$venv_bin/python" ]; then
    case ":${PATH:-}:" in
      *":$venv_bin:"*) ;;
      *) PATH="$venv_bin${PATH:+:$PATH}" ;;
    esac
    export PATH
  fi
}

# Usage: select_python_for_venv
#
# 0.8.23 Slice 80.3: print the path to the newest available Python >=3.11
# interpreter, for `scripts/bootstrap.sh` to create `.venv` with. Bare
# `python3` is whatever the OS ships — 3.10 on Ubuntu 22.04 and L4T R36
# (every Jetson) — which lacks stdlib `tomllib`, required by several
# release/CI gates (scripts/check-license-consistency.sh,
# scripts/check-pinned-override-rot.py).
#
# Tries versions newest-first (3.13, 3.12, 3.11); for each, tries a bare PATH
# command before falling back to `uv python find` if `uv` is present, so a
# higher version wins regardless of which mechanism provides it. The `uv`
# fallback exists because a modern interpreter can be genuinely available on
# a host — via `uv python install` — without ever appearing as a bare command
# on PATH; a selection function that only probes bare commands silently fails
# on exactly that host. Fails closed (nonzero exit, actionable stderr, no
# stdout) when nothing qualifies anywhere.
select_python_for_venv() {
  local versions=(3.13 3.12 3.11) v candidate

  for v in "${versions[@]}"; do
    candidate="$(command -v "python$v" 2>/dev/null || true)"
    if [ -n "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
    if command -v uv >/dev/null 2>&1; then
      candidate="$(uv python find "$v" 2>/dev/null || true)"
      if [ -n "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    fi
  done

  {
    printf 'select_python_for_venv: no Python 3.11+ interpreter found (checked bare\n'
    printf '  python3.13/python3.12/python3.11 on PATH, and uv python find if uv is\n'
    printf '  installed). Install one: on Ubuntu, the deadsnakes PPA (e.g.\n'
    printf '  sudo add-apt-repository ppa:deadsnakes/ppa && sudo apt install python3.12),\n'
    printf '  or sudo-free via uv python install 3.12 (https://docs.astral.sh/uv/).\n'
  } >&2
  return 1
}

# Usage: create_venv_with_selected_python <target-dir>
#
# Creates a venv at <target-dir> using select_python_for_venv's choice. Fails
# closed with no partial <target-dir> left on disk — whether no interpreter
# qualifies, or a qualifying interpreter's own `-m venv` fails partway (disk
# full, ensurepip failure) — a half-created venv would look bootstrapped
# while silently carrying the wrong (or no) interpreter, which is worse than
# no venv.
create_venv_with_selected_python() {
  local target_dir="$1" py

  py="$(select_python_for_venv)" || return 1
  if ! "$py" -m venv "$target_dir"; then
    rm -rf "$target_dir"
    return 1
  fi
}
