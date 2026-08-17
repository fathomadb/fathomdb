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
import subprocess
import sys
import tempfile
import threading
from pathlib import Path
from typing import Protocol, cast

import fathomdb
import fathomdb._fathomdb as native
from fathomdb import Engine, graph, read
from fathomdb.errors import ErasureIncompleteError


TYPED_BASELINE_OBSERVATION_EXIT = 65
CLEAN_BASELINE_OBSERVATION_EXIT = 66
NATIVE_IDLE_ROLE_FACTS = (
    "writer:0(auto=1,txn=none,busy=0,received=1)",
    *(f"readers:{index}(auto=1,txn=none,busy=0,received=1)" for index in range(8)),
    "dispatcher:0(auto=1,txn=none,busy=0,received=1)",
    "workers:0(auto=1,txn=none,busy=0,received=1)",
    "workers:1(auto=1,txn=none,busy=0,received=1)",
)


class _WalSnapshotPauseForTest(Protocol):
    """Dev-only rendezvous exposed by the disposable ``test-hooks`` wheel."""

    def wait_snapshot_ready(self) -> None: ...

    def release(self) -> None: ...

    def reader_connection_autocommit_for_test(self) -> bool: ...

    def reader_native_state_for_test(self) -> str: ...


class _WalAttributionTestHooks(Protocol):
    """Private Slice 65 diagnostics, absent from the shipped SDK surface."""

    def _arm_next_reader_snapshot_pause_for_test(self) -> _WalSnapshotPauseForTest: ...

    def _arm_next_reader_completion_pause_for_test(self) -> _WalSnapshotPauseForTest: ...

    def _wal_attribution_checkpoint_records_for_test(self) -> list[tuple[int, bool, str, list[str]]]: ...

    def _wal_attribution_snapshot_for_test(self) -> dict[str, bool]: ...

    def _arm_actual_checkpoint_observation_for_test(self) -> None: ...

    def _drain_actual_checkpoint_observations_for_test(self) -> list[str]: ...

    def _wal_attribution_binding_inventory_for_test(self) -> str: ...

    def _wal_attribution_binding_native_state_inventory_for_test(self) -> str: ...

    def _arm_binding_native_state_observation_for_test(self) -> None: ...

    def _drain_binding_native_state_observations_for_test(self) -> list[str]: ...

    def _checkpoint_at_rest_for_test(self) -> list[tuple[bool, int, int]]: ...


class _NativeRawCheckpointTestHook(Protocol):
    """Private module-level raw probe in the disposable ``test-hooks`` wheel."""

    def _native_raw_wal_checkpoint_for_test(self, path: str) -> tuple[bool, int, int]: ...


def _wal_attribution_test_hooks(engine: Engine) -> _WalAttributionTestHooks:
    """Narrow the installed control to its disposable test-hook wheel boundary."""
    return cast(_WalAttributionTestHooks, engine._native)


def _native_raw_checkpoint_test_hook() -> _NativeRawCheckpointTestHook:
    """Narrow the private raw probe without advertising it in the public stub."""
    return cast(_NativeRawCheckpointTestHook, native)


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


def _raw_binding_checkpoint(path: str, case: str) -> tuple[int, int, int]:
    """Take one redacted independent native/Rusqlite checkpoint sample."""
    busy, log_frames, checkpointed_frames = _native_raw_checkpoint_test_hook()._native_raw_wal_checkpoint_for_test(path)
    print(
        "slice65_wal python_binding_raw "
        f"case={case} raw_busy={busy} raw_log_frames={log_frames} "
        f"raw_checkpointed_frames={checkpointed_frames}",
        flush=True,
    )
    return busy, log_frames, checkpointed_frames


def run_serial_incident(
    expected_version: str,
    wheel_label: str,
    require_attribution: bool,
    observe_baseline_first_erase: bool,
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
        test_hooks = _wal_attribution_test_hooks(fresh)
        baseline_observation: tuple[str, str | None] | None = None
        try:
            first = read.get(fresh, "slice65-root")
            assert first is not None and first.logical_id == "slice65-root"
            if require_attribution:
                assert test_hooks._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
                print("slice65_wal serial_idle_after_read_get=passed", flush=True)
            neighbors = graph.neighbors(fresh, "slice65-root", depth=1, direction="outgoing")
            assert [node.logical_id for node in neighbors] == ["slice65-nested"]
            if require_attribution:
                assert test_hooks._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
                print("slice65_wal serial_idle_after_neighbors=passed", flush=True)
            print("slice65_wal serial_recovery_reads=passed", flush=True)

            if require_attribution:
                test_hooks._arm_actual_checkpoint_observation_for_test()
            try:
                nested = fresh.erase_source("slice65-nested-source")
            except ErasureIncompleteError as error:
                if observe_baseline_first_erase:
                    baseline_observation = ("typed_erasure_incomplete", _expected_wal_baseline(error))
                elif require_attribution:
                    assert error.stage == "wal_checkpoint"
                    print(
                        "slice65_wal serial_current_erase_observation=typed_erasure_incomplete",
                        flush=True,
                    )
                else:
                    raise
            else:
                assert nested.nodes_excised == 1
                if observe_baseline_first_erase:
                    baseline_observation = ("clean_completion", None)
                elif require_attribution:
                    print("slice65_wal serial_current_erase_observation=clean_completion", flush=True)
            if not observe_baseline_first_erase:
                if require_attribution:
                    records = test_hooks._drain_actual_checkpoint_observations_for_test()
                    assert records and len(records) % 2 == 0
                    for before, after in zip(records[::2], records[1::2], strict=True):
                        assert "control=python_serial phase=before" in before
                        assert "control=python_serial phase=after" in after
                        assert "writer_autocommit=1" in before
                        assert "writer_autocommit=1" in after
                        assert "direct_inventory=roles=writer:0,readers:0-7,dispatcher:0,workers:0-1;writer=autocommit;readers=autocommit;dispatcher=autocommit;workers=2-autocommit;registry=complete;creation=writer:1,readers:8,dispatcher:1,workers:2,probes:2;complete=1" in before
                        assert "collector_roles=idle" in after
                        assert "elapsed_ms=" in after and "busy=" in after
                        print(f"slice65_wal actual_checkpoint {before}", flush=True)
                        print(f"slice65_wal actual_checkpoint {after}", flush=True)
                    print("slice65_wal serial_current_attribution_expected=1", flush=True)
        finally:
            fresh.close()
    if observe_baseline_first_erase:
        assert baseline_observation is not None, "first erase must produce one baseline observation"
        outcome, frames = baseline_observation
        if outcome == "typed_erasure_incomplete":
            assert frames is not None
            print(
                "slice65_wal BASELINE_FIRST_ERASE outcome=typed_erasure_incomplete "
                f"type=ErasureIncompleteError stage=wal_checkpoint wal_frames={frames}",
                flush=True,
            )
            raise SystemExit(TYPED_BASELINE_OBSERVATION_EXIT)
        assert outcome == "clean_completion"
        print(
            "slice65_wal BASELINE_FIRST_ERASE outcome=clean_completion "
            "type=none stage=none wal_frames=0",
            flush=True,
        )
        raise SystemExit(CLEAN_BASELINE_OBSERVATION_EXIT)
    print(
        f"slice65_wal serial_result=passed wheel_version={expected_version} wheel_selector={wheel_label}",
        flush=True,
    )


def run_binding_reader_erase(expected_version: str) -> None:
    """Pause an actual reader-worker snapshot via the installed test wheel."""
    _assert_installed_version(expected_version)
    with tempfile.TemporaryDirectory(prefix="slice65-binding-") as directory:
        path = str(Path(directory) / "binding.sqlite")
        engine = Engine.open(path, use_default_embedder=False)
        try:
            engine.write([_node("slice65-binding", "slice65-binding-source")])
            test_hooks = _wal_attribution_test_hooks(engine)
            snapshot_pause = test_hooks._arm_next_reader_snapshot_pause_for_test()
            completion_pause = test_hooks._arm_next_reader_completion_pause_for_test()
            read_outcome: list[object] = []

            def governed_read() -> None:
                read_outcome.append(read.get(engine, "slice65-binding"))

            reader = threading.Thread(target=governed_read, name="slice65-python-read")
            reader.start()
            snapshot_pause.wait_snapshot_ready()
            snapshot_state = snapshot_pause.reader_native_state_for_test()
            assert "auto=0" in snapshot_state and "txn=read" in snapshot_state
            print(f"slice65_wal python_binding_held_reader_native_state={snapshot_state}", flush=True)
            print("slice65_wal python_binding_snapshot_ready", flush=True)
            try:
                engine.erase_source("slice65-binding-source")
            except ErasureIncompleteError as error:
                assert error.stage == "wal_checkpoint"
                print("slice65_wal python_binding_owned_reader_busy", flush=True)
            else:
                raise AssertionError("paused reader snapshot must fail closed")
            snapshot_pause.release()
            completion_pause.wait_snapshot_ready()
            assert completion_pause.reader_connection_autocommit_for_test() is True
            assert test_hooks._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
            print(
                "slice65_wal python_binding_completion_ack reader_autocommit=1 collector=idle",
                flush=True,
            )
            completion_pause.release()
            reader.join(timeout=5)
            assert not reader.is_alive() and read_outcome and read_outcome[0] is not None
            records = test_hooks._wal_attribution_checkpoint_records_for_test()
            busy = [r for r in records if r[1]]
            assert len(busy) == 5 and all(
                r[2] == "owned_reader_snapshot" and r[3] == ["reader_worker:0"] for r in busy
            )
            legacy_inventory = test_hooks._wal_attribution_binding_inventory_for_test()
            print(f"slice65_wal python_binding_direct_inventory={legacy_inventory}", flush=True)
            inventory = test_hooks._wal_attribution_binding_native_state_inventory_for_test()
            assert "state_inventory=complete" in inventory
            print(f"slice65_wal python_binding_native_state_inventory={inventory}", flush=True)
            first_raw = _raw_binding_checkpoint(path, "before_engine_sampler")
            test_hooks._arm_binding_native_state_observation_for_test()
            samples = test_hooks._checkpoint_at_rest_for_test()
            assert 1 <= len(samples) <= 5
            state_records = test_hooks._drain_binding_native_state_observations_for_test()
            assert len(state_records) == len(samples) * 2
            for before, after in zip(state_records[::2], state_records[1::2], strict=True):
                assert "state_inventory=complete" in before and "state_inventory=complete" in after
                assert all(role_fact in before for role_fact in NATIVE_IDLE_ROLE_FACTS)
                assert all(role_fact in after for role_fact in NATIVE_IDLE_ROLE_FACTS)
                print(f"slice65_wal python_binding_native_state {before}", flush=True)
                print(f"slice65_wal python_binding_native_state {after}", flush=True)
            print(
                f"slice65_wal python_binding_engine_sampler samples={len(samples)} "
                f"final_busy={int(samples[-1][0])}",
                flush=True,
            )
            second_raw = _raw_binding_checkpoint(path, "after_engine_sampler")
            if first_raw[0] != 0 or second_raw[0] != 0:
                engine.close()
                result = subprocess.run(
                    [
                        sys.executable,
                        __file__,
                        "--control",
                        "binding-child",
                        "--wheel-version",
                        expected_version,
                        "--wheel-label",
                        "current-source-test-hooks",
                        "--child-path",
                        path,
                    ],
                    check=False,
                    capture_output=True,
                    text=True,
                )
                assert result.returncode == 0, result.stderr
                assert "slice65_wal python_binding_child_raw case=after_close" in result.stdout
                print(result.stdout, end="", flush=True)
            else:
                print(
                    "slice65_wal python_binding_child_raw case=after_close outcome=not_required",
                    flush=True,
                )
        finally:
            engine.close()


def run_binding_child(expected_version: str, path: str) -> None:
    """Run the one conditional post-close raw probe in a fresh test-hook process."""
    _assert_installed_version(expected_version)
    busy, log_frames, checkpointed_frames = _native_raw_checkpoint_test_hook()._native_raw_wal_checkpoint_for_test(path)
    print(
        "slice65_wal python_binding_child_raw case=after_close outcome=recorded "
        f"raw_busy={busy} raw_log_frames={log_frames} raw_checkpointed_frames={checkpointed_frames}",
        flush=True,
    )


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
            test_hooks = _wal_attribution_test_hooks(engine)
            assert test_hooks._wal_attribution_snapshot_for_test()["no_owned_snapshot"] is True
            before = len(test_hooks._wal_attribution_checkpoint_records_for_test())
            engine.erase_source("slice65-retained-source")
            engine.transition("slice65-retained-root", "deleted", "slice65 retained idle control")
            engine.purge("slice65-retained-root")
            records = test_hooks._wal_attribution_checkpoint_records_for_test()[before:]
            assert records and all(r[1:] == (False, "no_owned_snapshot", []) for r in records)
            print("slice65_wal python_retained_materialized_idle=passed", flush=True)
        finally:
            engine.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wheel-version", required=True)
    parser.add_argument("--wheel-label", required=True)
    parser.add_argument("--control", choices=("serial", "binding", "binding-child", "retained"), required=True)
    parser.add_argument("--require-attribution", action="store_true")
    parser.add_argument("--observe-baseline-first-erase", action="store_true")
    parser.add_argument("--child-path")
    args = parser.parse_args()
    if args.control == "serial":
        run_serial_incident(
            args.wheel_version,
            args.wheel_label,
            args.require_attribution,
            args.observe_baseline_first_erase,
        )
    elif args.control == "binding-child":
        assert args.child_path is not None
        run_binding_child(args.wheel_version, args.child_path)
    else:
        if not args.require_attribution:
            raise SystemExit("binding control requires the current test-hooks attribution wheel")
        if args.control == "binding":
            run_binding_reader_erase(args.wheel_version)
        else:
            run_retained_materialized(args.wheel_version)


if __name__ == "__main__":
    main()
