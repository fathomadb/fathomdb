"""Manual G1 six-pass execution on one cell copy, including reversed repeat order."""

import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest

from hypothesis import Phase, given, settings, strategies as st

from g_template_fixture import run
from test_g_mean import GPU, read_jsonl_records
from test_g_query_feed import MODES, QUESTIONS, TEXT_IDS


class PassPlanTests(unittest.TestCase):
    def require_plan(self, repeat):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-pass-plan-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, 65)]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text("".join(json.dumps({"id": identity, "query": query}) + "\n"
                                   for identity, query in QUESTIONS))
        order = MODES if repeat == 0 else list(reversed(MODES))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G1_PLAN_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "queries": str(queries),
                                     "repeat": repeat, "mode": order[0], "passes": order, "warmups": 2}))
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
        identity = {"root": str(root), "oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                    "queries_sha256": hashlib.sha256(queries.read_bytes()).hexdigest(),
                    "records_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
        (root / "oracle-identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        print(json.dumps(identity), flush=True)
        run(config, root, binary=binary, feature_path=task / "proofs/gd-g-host-feature-closure.json")
        preparation = json.loads((root / "out/case-0/template.json").read_text())["preparation"]
        if (preparation["api_complete"], preparation["write_rows"], preparation["write_calls"],
                preparation["pending"]) != (True, 65, 2, 0):
            raise RuntimeError("existing real preparation/drain/close controls failed, NOT_RED")
        if len(preparation["coverage"]["rows"]) != 65 or preparation["coverage"]["profile"]["dimension"] != 384:
            raise RuntimeError("existing physical/default-profile controls failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed during pass calls, NOT_RED")
        store = root / "out/cell/store.sqlite"
        with sqlite3.connect(store.as_uri() + "?mode=ro", uri=True) as connection:
            actual = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL ORDER BY write_cursor").fetchall()
            joined = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor").fetchone()[0]
        if actual != [(row["id"], row["body"]) for row in records] or joined != 65:
            raise RuntimeError("independent cell-copy canonical/cursor/span controls failed, NOT_RED")
        completed = sorted((path for path in (root / "out/cell").glob("pass-*")
                            if (path / "api-proof.json").exists()), key=lambda path: int(path.name.split("-")[1]))
        if not completed:
            raise RuntimeError("existing single-pass caller did not complete, NOT_RED")
        observed = []
        allowed = {"l:" + record["id"] for record in records}
        for ordinal, directory in enumerate(completed):
            proof = json.loads((directory / "api-proof.json").read_text())
            mode = proof["mode"]
            issues = []
            warmups = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            if issues or (len(warmups), len(timed), proof["api_calls"], proof["timed_api_calls"], proof["warmup_api_calls"]) != (2, 3, 5, 3, 2):
                raise RuntimeError("existing actual single-pass counts/framing controls failed, NOT_RED")
            if proof["selected_cuda_uuid"] != GPU or proof["cuda_compiled"] is not True:
                raise RuntimeError("required actual Default CUDA identity failed, NOT_RED")
            if proof["open_ns"] <= 0 or proof["close_ns"] <= 0 or proof["store"] != str(store):
                raise RuntimeError("existing real open/close/cell-store controls failed, NOT_RED")
            for row in [*warmups, *timed]:
                if row["ok"] is not True or row["elapsed_ns"] <= 0:
                    raise RuntimeError("existing successful API/timer control failed, NOT_RED")
                if mode.endswith("_mean"):
                    if (row["dimension"], row["finite"]) != (384, True) or row["norm_squared"] <= 0:
                        raise RuntimeError("existing actual mean API value controls failed, NOT_RED")
                else:
                    if len(row["ids"]) != len(set(row["ids"])) or not set(row["ids"]) <= allowed:
                        raise RuntimeError("existing real result identity controls failed, NOT_RED")
                    if mode.endswith("_hybrid") and (row["returned_hits"], row["soft_fallback"], row["degradations"]) != (10, "None", "[]"):
                        raise RuntimeError("existing actual hybrid API controls failed, NOT_RED")
            observed.append((directory, proof, warmups, timed))
        print(json.dumps({"root": str(root), "repeat": repeat, "completed_passes": len(observed),
                          "actual_modes": [row[1]["mode"] for row in observed]}), flush=True)
        self.assertEqual(len(observed), 6, "missing actual six-pass G1 execution")
        self.assertEqual([row[1]["mode"] for row in observed], order)
        for ordinal, (directory, proof, warmups, timed) in enumerate(observed):
            mode = order[ordinal]
            self.assertEqual(directory.name, f"pass-{ordinal}")
            self.assertEqual(proof["store"], str(store))
            self.assertEqual([(row["query_id"], row["query_fingerprint"]) for row in timed],
                             [(identity, hashlib.sha256(query.encode()).hexdigest()) for identity, query in QUESTIONS])
            self.assertEqual([(row["query_id"], row["query_fingerprint"]) for row in warmups],
                             [(identity, hashlib.sha256(query.encode()).hexdigest()) for identity, query in QUESTIONS[:2]])
            self.assertEqual([row["call_index"] for row in [*warmups, *timed]], [0, 1, 2, 3, 4])
            self.assertEqual([row["sample_index"] for row in timed], [0, 1, 2])
            self.assertEqual({row["adapter"] for row in [*warmups, *timed]}, {"engine" if mode.startswith("engine_") else "sdk"})
            if mode.endswith("_text"):
                self.assertEqual([row["ids"] for row in timed], [TEXT_IDS[query] for _, query in QUESTIONS])
            self.assertLessEqual(sum(row["elapsed_ns"] for row in [*warmups, *timed]), proof["loop_ns"])
            self.assertEqual(proof["raw_persistence"]["records"], 5)
            self.assertEqual(proof["raw_persistence"]["checkpoints"], 2)
        self.assertEqual(sum(row[1]["api_calls"] for row in observed), 30)
        self.assertEqual(sum(row[1]["timed_api_calls"] for row in observed), 18)
        self.assertEqual(sum(row[1]["warmup_api_calls"] for row in observed), 12)

    def test_literal_core_then_sdk_six_pass_execution(self):
        self.require_plan(0)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(repeat=st.integers(min_value=0, max_value=1))
    def test_bounded_repeat_preserves_forward_and_reverse_adapter_mode_order(self, repeat):
        self.require_plan(repeat)


if __name__ == "__main__":
    unittest.main()
