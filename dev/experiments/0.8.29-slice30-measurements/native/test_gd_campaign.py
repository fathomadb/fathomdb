"""Tiny real manifest/controller/native D integration; not campaign qualification."""

import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

from hypothesis import Phase, given, settings, strategies as st

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from gd_campaign import run_manifest  # noqa: E402


def verify(rows):
    root = Path(tempfile.mkdtemp(prefix="s30-gd-controller-"))
    binary = Path(os.environ["CARGO_TARGET_DIR"])/"release/examples/slice30_gd_diagnostic"
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    print(f"source controller {root} binary={digest}", flush=True)
    records = [{"id":f"manifest-{i}", "kind":"doc" if i%2==0 else "note",
                "body":f"campaignwitness évidence {i}"} for i in range(rows)]
    base = {"family":"D_QUERY_FIXTURE", "records":[], "empty_start":True,
            "write_records":records, "cohort":2, "warmups":2, "samples":3,
            "queries":[{"id":"actual-query", "query":"campaignwitness"}],
            "readiness":True, "artifact":"do"}
    cells = [dict(base, id="d-off", observer=False), dict(base, id="d-on", observer=True)]
    artifact = {"path":str(binary), "sha256":digest,
                "features":["test-hooks","slice30-measurement"], "profile":"release"}
    manifest = {"schema":"slice30-gd/source-fixture-v1", "cell_seconds":180,
                "campaign_seconds":14400, "artifacts":{"do":artifact}, "cells":cells}
    path = root/"manifest.json"
    path.write_text(json.dumps(manifest))
    control = root/"direct-control"
    config = root/"direct.json"
    config.write_text(json.dumps(cells[0]))
    direct = subprocess.run([str(binary), str(config), str(control)], capture_output=True, text=True, timeout=150)
    assert direct.returncode == 0, f"native fixture setup NOT_RED {direct.stderr}"
    assert json.loads((control/"complete.json").read_text())["complete"] is True
    outcomes = run_manifest(json.loads(path.read_text()), root/"results")
    assert [outcome["id"] for outcome in outcomes] == ["d-off", "d-on"]
    attempts = []
    for outcome, cell in zip(outcomes, cells, strict=True):
        assert outcome["exit"] == 0, f"native controller setup NOT_RED {outcome}"
        attempt = Path(outcome["attempt"])
        attempts.append(str(attempt))
        assert json.loads((attempt/"complete.json").read_text())["complete"] is True
        with sqlite3.connect(f"file:{attempt/'query.sqlite'}?mode=ro", uri=True) as db:
            actual = db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall()
            assert actual == [(record["id"],record["kind"],record["body"]) for record in records]
            for table in ["_fathomdb_vector_rows","_fathomdb_vector_spans"]:
                assert db.execute(f"SELECT count(*) FROM {table}").fetchone()[0] == rows
        calls = [json.loads(line) for line in (attempt/"queries.jsonl").read_text().splitlines()]
        assert [call["phase"] for call in calls] == ["warmup"]*2+["timed"]*3
        assert all(call["ok"] and call["query_id"] == "actual-query" for call in calls)
        writes = json.loads((attempt/"writes.json").read_text())
        assert len(writes) == (rows+1)//2 and sum(write["rows"] for write in writes) == rows
        observer = json.loads((attempt/"worker-observer.json").read_text())
        assert observer["available"] is cell["observer"]
        if cell["observer"]:
            assert sum(worker["job_count"] for worker in observer["records"]) == rows
        else:
            assert observer["records"] is None
    print("actual controller child calls/counts/SQL/feature controls passed", flush=True)
    assert [outcome["status"] for outcome in outcomes] == ["complete", "complete"], "missing D completion protocol integration in real controller"
    state = json.loads((root/"results/campaign-state.json").read_text())
    assert [row["id"] for row in state] == ["d-off", "d-on"]
    resumed = run_manifest(manifest, root/"results")
    assert [outcome["attempt"] for outcome in resumed] == attempts
    assert len(list((root/"results").glob("*/attempt-*"))) == 2


class ActualCampaignTests(unittest.TestCase):
    def test_real_two_cell_manifest_uses_actual_native_completion_and_resume(self):
        verify(3)

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=1, max_value=5))
    def test_real_manifest_preserves_population_tail_and_cell_order(self, rows):
        verify(rows)


if __name__ == "__main__":
    unittest.main()
