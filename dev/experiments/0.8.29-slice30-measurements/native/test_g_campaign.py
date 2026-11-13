"""Manual pinned-CUDA G manifest/controller integration, not a natural campaign."""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import sqlite3
from contextlib import closing
import sys
import tempfile
import time
import unittest

from test_g_text import GPU, _accounting

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from gd_campaign import run_manifest  # noqa: E402

MODES = ["engine_mean", "sdk_mean", "engine_hybrid", "sdk_hybrid", "engine_text", "sdk_text"]


class ActualGCampaignTests(unittest.TestCase):
    def test_actual_g_manifest_completes_all_six_existing_modes(self):
        if os.environ.get("CUDA_VISIBLE_DEVICES") != GPU or os.environ.get("FATHOMDB_EMBED_DEVICE") != "cuda:0":
            raise RuntimeError("approved forced CUDA reservation missing NOT_RED")
        task = Path(os.environ["S30_GD_TASK"])
        features = json.loads(Path(os.environ["S30_G_CONTROLLER_FEATURES"]).read_text())
        binary = Path(features["binary"])
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        if features["binary_sha256"] != digest or features["normal_gn"] is not True:
            raise RuntimeError("actual normal GN artifact unavailable NOT_RED")
        for artifact in features["artifacts"]:
            if any(feature == "test-hooks" or feature.startswith("tc5-benchmark") or feature == "slice30-measurement" for feature in artifact["features"]):
                raise RuntimeError("normal G feature contamination NOT_RED")
        for row in json.loads((task/"receipts/gd-pinned-inputs.json").read_text())["copied"]:
            if hashlib.sha256(Path(row["destination"]).read_bytes()).hexdigest() != row["sha256"]:
                raise RuntimeError("existing pinned model bytes unavailable NOT_RED")
        root = Path(tempfile.mkdtemp(prefix="s30-g-controller-"))
        print(f"actual G controller {root} binary={digest}", flush=True)
        records = [{"id":"alpha", "kind":"doc", "body":"alphaonly épreuve"},
                   {"id":"beta", "kind":"doc", "body":"betaonly βήτα"}]
        records += [{"id":f"doc-{i}", "kind":"doc", "body":f"commonword natural record {i}"} for i in range(2,65)]
        corpus, questions = root/"records.jsonl", root/"queries.jsonl"
        corpus.write_text("".join(json.dumps(row)+"\n" for row in records))
        queries = [{"id":0, "query":"betaonly"}, {"id":1, "query":"alphaonly"}, {"id":2, "query":"missingnomatch"}]
        questions.write_text("".join(json.dumps(row)+"\n" for row in queries))
        cell = {"id":"g-six-modes", "artifact":"gn", "family":"G1_PLAN_FIXTURE", "gpu_uuid":GPU,
                "templates":[{"records":str(corpus)}], "queries":str(questions),
                "mode":MODES[0], "passes":MODES, "warmups":2, "repeat":0}
        manifest = {"schema":"slice30-gd/source-fixture-v1", "cell_seconds":150, "campaign_seconds":14400,
                    "artifacts":{"gn":{"path":str(binary),"sha256":digest,"features":["slice30-g"],"profile":"release"}}, "cells":[cell]}
        (root/"manifest.json").write_text(json.dumps(manifest))
        snapshots, errors = [], []
        deadline = time.monotonic()+150
        def monitor(pid):
            try:
                snapshots.append(_accounting(pid, deadline))
            except Exception as error:
                errors.append(str(error))
                return str(error)
            return None
        with (task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
            before = _accounting(None, deadline)
            if before["util"] != 0 or before["total_MiB"]-before["used_MiB"] < 8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            outcomes = run_manifest(manifest, root/"results", monitor=monitor)
        print(json.dumps({"root":str(root),"before":before,"gpu_samples":snapshots,"scope":"whole real preparation plus six API-mode series; no per-query GPU attribution"}), flush=True)
        if errors or outcomes[0]["exit"] != 0:
            raise RuntimeError(f"native/GPU accounting setup NOT_RED {errors} {outcomes}")
        if not any(row["pid"] == outcomes[0]["pid"] and row["own_MiB"] > 0 and row["util"] > 0 for row in snapshots):
            raise RuntimeError("actual own PID/UUID CUDA engagement absent NOT_RED")
        attempt = Path(outcomes[0]["attempt"])
        prepared = json.loads((attempt/"case-0/template.json").read_text())["preparation"]
        assert (prepared["api_complete"], prepared["write_rows"], prepared["write_calls"], prepared["pending"]) == (True,65,2,0)
        assert len(prepared["coverage"]["rows"]) == 65 and prepared["coverage"]["profile"]["dimension"] == 384
        with closing(sqlite3.connect(f"file:{attempt/'cell/store.sqlite'}?mode=ro", uri=True)) as db:
            actual = db.execute("SELECT logical_id,body FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY write_cursor").fetchall()
            assert actual == [(row["id"],row["body"]) for row in records]
        for ordinal, mode in enumerate(MODES):
            directory = attempt/"cell"/f"pass-{ordinal}"
            proof = json.loads((directory/"api-proof.json").read_text())
            assert proof["mode"] == mode and proof["selected_cuda_uuid"] == GPU and proof["cuda_compiled"] is True
            assert (proof["warmup_api_calls"],proof["timed_api_calls"],proof["api_calls"]) == (2,3,5)
            timed = [json.loads(line) for line in (directory/"samples.jsonl").read_text().splitlines()]
            warm = [json.loads(line) for line in (directory/"warmups.jsonl").read_text().splitlines()]
            assert len(warm) == 2 and len(timed) == 3
            assert [(row["query_id"],row["query_fingerprint"]) for row in timed] == [(row["id"],hashlib.sha256(row["query"].encode()).hexdigest()) for row in queries]
            assert all(row["ok"] and row["elapsed_ns"] > 0 for row in [*warm,*timed])
            if mode.endswith("_mean"):
                assert all(row["dimension"] == 384 and row["finite"] and row["norm_squared"] > 0 for row in [*warm,*timed])
            elif mode.endswith("_text"):
                assert [row["ids"] for row in timed] == [["l:beta"],["l:alpha"],[]]
            else:
                assert all((row["returned_hits"],row["soft_fallback"],row["degradations"]) == (10,"None","[]") for row in [*warm,*timed])
        print("actual normal G preparation/CUDA/SQL/all-six-mode API controls passed", flush=True)
        self.assertEqual(outcomes[0]["status"], "complete", "missing actual G manifest completion integration")


if __name__ == "__main__":
    unittest.main()
