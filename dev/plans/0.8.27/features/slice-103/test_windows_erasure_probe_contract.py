"""Contract checks for the installed-wheel erasure trial recorder."""

from pathlib import Path
import runpy


PROBE = Path(__file__).with_name("windows_erasure_probe.py")


def _passes(record: dict) -> bool:
    return runpy.run_path(str(PROBE))["record_passes"](record)


def _complete_record() -> dict:
    return {
        "arm": "retry",
        "first": {"ok": True, "report": {"nodes_excised": 50}},
        "zero_count_completion": {
            "nodes_excised": 0,
            "edges_excised": 0,
            "projections_invalidated": 0,
        },
        "post_completion_write": {"ok": True},
        "final_rows": [{"source_id": "src-b", "body": '{"text":"keep"}'}],
    }


def test_complete_erase_with_zero_count_and_survivor_passes() -> None:
    assert _passes(_complete_record())


def test_unresolved_reopen_refusal_cannot_pass() -> None:
    record = _complete_record()
    record["first"] = {"ok": False}
    record["same_engine_retries"] = [{"ok": False}]
    record["reopen_retry"] = {"ok": False}
    assert not _passes(record)


def test_missing_or_nonzero_second_completion_cannot_pass() -> None:
    record = _complete_record()
    del record["zero_count_completion"]
    assert not _passes(record)

    record = _complete_record()
    record["zero_count_completion"]["nodes_excised"] = 1
    assert not _passes(record)


def test_pending_dependency_closure_requires_write_refusal() -> None:
    record = _complete_record()
    record["arm"] = "diagnostic"
    record["first"] = {"ok": False}
    record["reopen_retry"] = {"ok": True}
    record["pending_after_first"] = [{"phase": "owed", "cause": "erase_source"}]
    record["write_fence"] = {"ok": True}
    assert not _passes(record)

    record["write_fence"] = {"ok": False, "error": {"stage": "dependency_closure"}}
    assert _passes(record)


def test_empty_closure_does_not_require_write_refusal() -> None:
    record = _complete_record()
    record["arm"] = "diagnostic"
    record["first"] = {"ok": False}
    record["reopen_retry"] = {"ok": True}
    record["pending_after_first"] = []
    record["write_fence"] = {"ok": True}
    assert _passes(record)
