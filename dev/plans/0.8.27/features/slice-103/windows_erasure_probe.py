"""Installed-wheel, real-database Slice 103 erasure trial recorder.

Run outside the source tree with an isolated wheel environment. One JSON line
is emitted per trial; a checkpoint is never issued by this probe.
"""

from __future__ import annotations

import argparse
from contextlib import closing
import json
from pathlib import Path
import re
import shutil
import sqlite3
import sys
import tempfile
import time
from typing import Any

import fathomdb
from fathomdb import read
from fathomdb.types import PageRequestV1, ReadContextV1


FRAME_PATTERN = re.compile(r"\((\d+) frames still in the log\)")


def error_fields(exc: BaseException) -> dict[str, Any]:
    detail = getattr(exc, "detail", str(exc))
    match = FRAME_PATTERN.search(str(detail))
    return {
        "type": type(exc).__name__,
        "stage": getattr(exc, "stage", None),
        "detail": str(detail),
        "frames_in_detail": int(match.group(1)) if match else None,
    }


def wal_bytes(path: Path) -> int:
    wal = Path(f"{path}-wal")
    return wal.stat().st_size if wal.exists() else 0


def report_fields(report: Any) -> dict[str, Any]:
    return {
        "source_ref": report.source_ref,
        "nodes_excised": report.nodes_excised,
        "edges_excised": report.edges_excised,
        "projections_invalidated": report.projections_invalidated,
    }


def stored_rows(path: Path) -> list[dict[str, str]]:
    # This post-operation read uses a short-lived separate connection. It can
    # change timing, so the unchanged historical script remains the rate control.
    with closing(sqlite3.connect(f"file:{path}?mode=ro", uri=True)) as connection:
        return [
            {"source_id": source, "body": body}
            for source, body in connection.execute(
                "SELECT source_id, body FROM canonical_nodes ORDER BY write_cursor"
            )
        ]


def pending_physical(path: Path) -> list[dict[str, str]]:
    with closing(sqlite3.connect(f"file:{path}?mode=ro", uri=True)) as connection:
        return [
            {"phase": phase, "cause": cause}
            for phase, cause in connection.execute(
                "SELECT phase, cause FROM _fathomdb_dependency_closures "
                "WHERE phase!='complete' ORDER BY closure_operation_id"
            )
        ]


def one_trial(mode: str, batch: int, trial: int) -> dict[str, Any]:
    root = Path(tempfile.mkdtemp(prefix="fdb-s103w-"))
    db = root / "memory.db"
    record: dict[str, Any] = {
        "platform": sys.platform,
        "mode": mode,
        "batch": batch,
        "trial": trial,
        "verb": "erase_source",
        "binding": "python-installed-wheel",
    }
    engine = fathomdb.Engine.open(str(db), use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "Note",
                    "body": json.dumps({"text": f"row {index} " * 20}),
                    "source_id": "src-a",
                }
                for index in range(50)
            ]
        )
        engine.write([{"kind": "Note", "body": json.dumps({"text": "keep"}), "source_id": "src-b"}])
        if mode == "read":
            rows = [*read.list(engine, "Note")]
            record["materialized_read_count"] = len(rows)
        elif mode == "page":
            frozen = engine.freeze_read_context(ReadContextV1())
            page = read.canonical_page(engine, "Note", frozen, PageRequestV1(50, None))
            rows = [*page.items]
            record["materialized_read_count"] = len(rows)
        record["wal_before_erase_bytes"] = wal_bytes(db)
        try:
            first = engine.erase_source("src-a")
            record["first"] = {"ok": True, "report": report_fields(first)}
        except Exception as exc:  # Keep the exact typed failure in evidence.
            record["first"] = {"ok": False, "error": error_fields(exc)}
        record["wal_after_erase_bytes"] = wal_bytes(db)
        record["rows_after_first"] = stored_rows(db)
        record["pending_after_first"] = pending_physical(db)

        if not record["first"]["ok"]:
            try:
                engine.write([{"kind": "Note", "body": "fenced", "source_id": "src-c"}])
                record["write_fence"] = {"ok": True}
            except Exception as exc:
                record["write_fence"] = {"ok": False, "error": error_fields(exc)}
            record["rows_after_fence_probe"] = stored_rows(db)
            record["same_engine_retries"] = []
            for delay in (0.05, 0.25, 1.0, 2.0):
                time.sleep(delay)
                try:
                    result = engine.erase_source("src-a")
                    record["same_engine_retries"].append(
                        {"delay_s": delay, "ok": True, "report": report_fields(result)}
                    )
                    break
                except Exception as exc:
                    record["same_engine_retries"].append(
                        {"delay_s": delay, "ok": False, "error": error_fields(exc)}
                    )
        engine.close()

        engine = fathomdb.Engine.open(str(db), use_default_embedder=False)
        record["rows_after_reopen"] = stored_rows(db)
        if not record["first"]["ok"] and not any(
            item["ok"] for item in record.get("same_engine_retries", [])
        ):
            try:
                result = engine.erase_source("src-a")
                record["reopen_retry"] = {"ok": True, "report": report_fields(result)}
            except Exception as exc:
                record["reopen_retry"] = {"ok": False, "error": error_fields(exc)}
        else:
            result = engine.erase_source("src-a")
            record["zero_count_completion"] = report_fields(result)
        if record.get("reopen_retry", {}).get("ok"):
            result = engine.erase_source("src-a")
            record["zero_count_completion"] = report_fields(result)
        record["wal_after_reopen_bytes"] = wal_bytes(db)
        try:
            engine.write([{"kind": "Note", "body": "after-completion", "source_id": "src-c"}])
            record["post_completion_write"] = {"ok": True}
        except Exception as exc:
            record["post_completion_write"] = {"ok": False, "error": error_fields(exc)}
        record["final_rows"] = stored_rows(db)
        engine.close()
        record["assertions"] = {
            "erased_rows_absent": all(
                row["source_id"] != "src-a" for row in record["final_rows"]
            ),
            "survivor_present": any(
                row["source_id"] == "src-b" and "keep" in row["body"]
                for row in record["final_rows"]
            ),
            "post_completion_write": record["post_completion_write"]["ok"],
        }
        return record
    finally:
        engine.close()
        shutil.rmtree(root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("trials", type=int)
    parser.add_argument("mode", choices=("noread", "read", "page"))
    parser.add_argument("--batch", type=int, default=1)
    args = parser.parse_args()
    failures = 0
    for trial in range(args.trials):
        try:
            record = one_trial(args.mode, args.batch, trial)
            if not all(record["assertions"].values()):
                failures += 1
        except Exception as exc:
            failures += 1
            record = {
                "platform": sys.platform,
                "mode": args.mode,
                "batch": args.batch,
                "trial": trial,
                "probe_failure": error_fields(exc),
            }
        print(json.dumps(record, sort_keys=True), flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
