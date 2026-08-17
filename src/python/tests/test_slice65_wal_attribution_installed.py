"""Installed-wheel controls for Slice 65 Windows WAL attribution.

This file is deliberately executable as a script against a wheel installed in
an isolated environment.  It never imports the checkout package: the Windows
job installs either the released 0.8.22 wheel or a current, disposable
``test-hooks`` wheel before invoking it.
"""

from __future__ import annotations

import argparse
import tempfile
from pathlib import Path

import fathomdb
from fathomdb import Engine, graph, read
from fathomdb.errors import ErasureIncompleteError


def _node(logical_id: str, source_id: str) -> dict[str, str]:
    return {
        "kind": "doc",
        "body": f"Slice 65 incident control {logical_id}",
        "logical_id": logical_id,
        "source_id": source_id,
    }


def _edge(from_id: str, to_id: str, source_id: str) -> dict[str, dict[str, str]]:
    return {
        "edge": {
            "kind": "relates_to",
            "from": from_id,
            "to": to_id,
            "logical_id": "slice65-incident-edge",
            "source_id": source_id,
        }
    }


def _assert_installed_version(expected: str) -> None:
    assert fathomdb.__version__ == expected, (
        f"expected installed wheel {expected}, got {fathomdb.__version__}; "
        "do not run this control against a checkout import"
    )
    package_file = Path(fathomdb.__file__).resolve()
    assert "site-packages" in package_file.parts, f"not an installed wheel: {package_file}"


def run_serial_incident(expected_version: str, require_attribution: bool) -> None:
    """Run the audited close/reopen/recovery-read/nested-erasure shape once."""
    _assert_installed_version(expected_version)
    with tempfile.TemporaryDirectory(prefix="slice65-serial-") as directory:
        path = str(Path(directory) / "incident.sqlite")
        old = Engine.open(path, use_default_embedder=False)
        old.write(
            [
                _node("slice65-root", "slice65-root-source"),
                _node("slice65-nested", "slice65-nested-source"),
                _edge("slice65-root", "slice65-nested", "slice65-nested-source"),
            ]
        )
        old.close()

        fresh = Engine.open(path, use_default_embedder=False)
        try:
            first = read.get(fresh, "slice65-root")
            assert first is not None and first.logical_id == "slice65-root"
            neighbors = graph.neighbors(fresh, "slice65-root", depth=1, direction="out")
            assert [node.logical_id for node in neighbors] == ["slice65-nested"]
            print("slice65_wal serial_recovery_reads=passed", flush=True)

            nested = fresh.erase_source("slice65-nested-source")
            assert nested.nodes_excised == 1
            fresh.transition("slice65-root", "deleted", "slice65 incident control")
            fresh.purge("slice65-root")
            if require_attribution:
                # The collector itself emits redacted lifecycle/checkpoint records.
                print("slice65_wal serial_current_attribution_expected=1", flush=True)
        finally:
            fresh.close()
    print(f"slice65_wal serial_result=passed wheel_version={expected_version}", flush=True)


def run_binding_reader_erase(expected_version: str) -> None:
    """Pause an actual reader-worker snapshot via the installed test wheel."""
    _assert_installed_version(expected_version)
    with tempfile.TemporaryDirectory(prefix="slice65-binding-") as directory:
        engine = Engine.open(str(Path(directory) / "binding.sqlite"), use_default_embedder=False)
        try:
            engine.write([_node("slice65-binding", "slice65-binding-source")])
            native = engine._native
            pause = native._pause_reader_after_wal_snapshot_for_test()
            pause.wait_snapshot_ready()
            print("slice65_wal python_binding_snapshot_ready", flush=True)
            try:
                engine.erase_source("slice65-binding-source")
            except ErasureIncompleteError:
                print("slice65_wal python_binding_owned_reader_busy", flush=True)
            else:
                raise AssertionError("paused reader snapshot must fail closed")
            pause.release()
            engine.erase_source("slice65-binding-source")
            print("slice65_wal python_binding_snapshot_released", flush=True)
        finally:
            engine.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wheel-version", required=True)
    parser.add_argument("--control", choices=("serial", "binding"), required=True)
    parser.add_argument("--require-attribution", action="store_true")
    args = parser.parse_args()
    if args.control == "serial":
        run_serial_incident(args.wheel_version, args.require_attribution)
    else:
        if not args.require_attribution:
            raise SystemExit("binding control requires the current test-hooks attribution wheel")
        run_binding_reader_erase(args.wheel_version)


if __name__ == "__main__":
    main()
