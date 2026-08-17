"""Installed-wheel controls for Slice 65 Windows WAL attribution.

This file is deliberately executable as a script against a wheel installed in
an isolated environment.  It never imports the checkout package: the Windows
job installs either the released 0.8.22 wheel or a current, disposable
``test-hooks`` wheel before invoking it.
"""

from __future__ import annotations

import argparse
import json
import re
import tempfile
import threading
from pathlib import Path

import fathomdb
from fathomdb import Engine, graph, read
from fathomdb.errors import ErasureIncompleteError


EXPECTED_BASELINE_OBSERVATION_EXIT = 65


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


def _expected_wal_baseline(error: ErasureIncompleteError) -> str:
    """Normalize only the released WAL-checkpoint refusal for the artifact."""
    if error.stage != "wal_checkpoint":
        raise AssertionError(f"expected WAL checkpoint stage, got {error.stage!r}") from error
    frame_match = re.search(r"\((\d+) frames still in the log\)", error.detail)
    if frame_match is None or "wal_checkpoint(TRUNCATE)" not in error.detail or "BUSY" not in error.detail:
        raise AssertionError("expected typed WAL checkpoint BUSY detail with retained frame count") from error
    return frame_match.group(1)


def run_serial_incident(
    expected_version: str,
    wheel_label: str,
    require_attribution: bool,
    expect_erasure_incomplete: bool,
) -> None:
    """Run the audited close/reopen/recovery-read/nested-erasure shape once."""
    _assert_installed_version(expected_version)
    print(f"slice65_wal serial_wheel_selector={wheel_label}", flush=True)
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
        expected_baseline_frames: str | None = None
        try:
            first = read.get(fresh, "slice65-root")
            assert first is not None and first.logical_id == "slice65-root"
            if require_attribution:
                assert fresh._native._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
                print("slice65_wal serial_idle_after_read_get=passed", flush=True)
            neighbors = graph.neighbors(fresh, "slice65-root", depth=1, direction="outgoing")
            assert [node.logical_id for node in neighbors] == ["slice65-nested"]
            if require_attribution:
                assert fresh._native._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
                print("slice65_wal serial_idle_after_neighbors=passed", flush=True)
            print("slice65_wal serial_recovery_reads=passed", flush=True)

            before = len(fresh._native._wal_attribution_checkpoint_records_for_test()) if require_attribution else 0
            try:
                nested = fresh.erase_source("slice65-nested-source")
            except ErasureIncompleteError as error:
                if not expect_erasure_incomplete:
                    raise
                expected_baseline_frames = _expected_wal_baseline(error)
            if expected_baseline_frames is None:
                assert nested.nodes_excised == 1
                fresh.transition("slice65-root", "deleted", "slice65 incident control")
                fresh.purge("slice65-root")
                if require_attribution:
                    records = fresh._native._wal_attribution_checkpoint_records_for_test()[before:]
                    assert records and all(r[1:] == (False, "no_owned_snapshot", []) for r in records)
                    print("slice65_wal serial_current_attribution_expected=1", flush=True)
        finally:
            fresh.close()
    if expect_erasure_incomplete:
        if expected_baseline_frames is None:
            raise AssertionError("released serial baseline unexpectedly completed instead of refusing WAL erasure")
        print(
            "slice65_wal serial_expected_erasure "
            f"type=ErasureIncompleteError stage=wal_checkpoint wal_frames={expected_baseline_frames}",
            flush=True,
        )
        raise SystemExit(EXPECTED_BASELINE_OBSERVATION_EXIT)
    print(
        f"slice65_wal serial_result=passed wheel_version={expected_version} wheel_selector={wheel_label}",
        flush=True,
    )


def run_binding_reader_erase(expected_version: str) -> None:
    """Pause an actual reader-worker snapshot via the installed test wheel."""
    _assert_installed_version(expected_version)
    with tempfile.TemporaryDirectory(prefix="slice65-binding-") as directory:
        engine = Engine.open(str(Path(directory) / "binding.sqlite"), use_default_embedder=False)
        try:
            engine.write([_node("slice65-binding", "slice65-binding-source")])
            native = engine._native
            pause = native._arm_next_reader_snapshot_pause_for_test()
            read_outcome: list[object] = []

            def governed_read() -> None:
                read_outcome.append(read.get(engine, "slice65-binding"))

            reader = threading.Thread(target=governed_read, name="slice65-python-read")
            reader.start()
            pause.wait_snapshot_ready()
            print("slice65_wal python_binding_snapshot_ready", flush=True)
            try:
                engine.erase_source("slice65-binding-source")
            except ErasureIncompleteError:
                print("slice65_wal python_binding_owned_reader_busy", flush=True)
            else:
                raise AssertionError("paused reader snapshot must fail closed")
            pause.release()
            reader.join(timeout=5)
            assert not reader.is_alive() and read_outcome and read_outcome[0] is not None
            engine.erase_source("slice65-binding-source")
            records = native._wal_attribution_checkpoint_records_for_test()
            busy = [r for r in records if r[1]]
            assert len(busy) == 5 and all(
                r[2] == "owned_reader_snapshot" and r[3] == ["reader_worker:0"] for r in busy
            )
            assert records[-1][1:] == (False, "no_owned_snapshot", [])
            print("slice65_wal python_binding_snapshot_released", flush=True)
        finally:
            engine.close()


def run_retained_materialized(expected_version: str) -> None:
    """Retain public read data and prove the live Engine is immediately idle."""
    _assert_installed_version(expected_version)
    with tempfile.TemporaryDirectory(prefix="slice65-retained-") as directory:
        engine = Engine.open(str(Path(directory) / "retained.sqlite"), use_default_embedder=False)
        try:
            engine.write(
                [
                    {**_node("slice65-retained", "slice65-retained-source"), "body": json.dumps({"nested": {"value": "kept"}})},
                    _node("slice65-retained-root", "slice65-retained-root-source"),
                ]
            )
            retained = read.get(engine, "slice65-retained")
            assert retained is not None and json.loads(retained.body)["nested"]["value"] == "kept"
            native = engine._native
            assert native._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
            before = len(native._wal_attribution_checkpoint_records_for_test())
            engine.erase_source("slice65-retained-source")
            engine.transition("slice65-retained-root", "deleted", "slice65 retained idle control")
            engine.purge("slice65-retained-root")
            records = native._wal_attribution_checkpoint_records_for_test()[before:]
            assert records and all(r[1:] == (False, "no_owned_snapshot", []) for r in records)
            print("slice65_wal python_retained_materialized_idle=passed", flush=True)
        finally:
            engine.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wheel-version", required=True)
    parser.add_argument("--wheel-label", required=True)
    parser.add_argument("--control", choices=("serial", "binding", "retained"), required=True)
    parser.add_argument("--require-attribution", action="store_true")
    parser.add_argument("--expect-erasure-incomplete", action="store_true")
    args = parser.parse_args()
    if args.control == "serial":
        run_serial_incident(
            args.wheel_version,
            args.wheel_label,
            args.require_attribution,
            args.expect_erasure_incomplete,
        )
    else:
        if not args.require_attribution:
            raise SystemExit("binding control requires the current test-hooks attribution wheel")
        if args.control == "binding":
            run_binding_reader_erase(args.wheel_version)
        else:
            run_retained_materialized(args.wheel_version)


if __name__ == "__main__":
    main()
