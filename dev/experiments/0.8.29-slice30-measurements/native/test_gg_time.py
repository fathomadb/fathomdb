"""Manual GG baseline/window view stages with literal controls and real diagnostics."""

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

STAGES = [("baseline", 100, True), ("before", 80, False), ("inside", 100, True), ("after", 120, False)]


class TimeStageTests(unittest.TestCase):
    def require_stages(self, warmups, samples):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-gg-time-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, 65)]
        if any("s30ggwindow" in row["body"] for row in records):
            raise RuntimeError("literal control term is not absent from source fixture, NOT_RED")
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "GG_TIME_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "warmups": warmups,
                                     "samples": samples, "observer": True}))
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
        feature = task / "proofs/gd-g-host-feature-closure.json"
        identity = {"root": str(root), "warmups": warmups, "samples": samples,
                    "oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "feature_sha256": hashlib.sha256(feature.read_bytes()).hexdigest(),
                    "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                    "records_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
        (root / "oracle-identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        print(json.dumps(identity), flush=True)
        run(config, root, binary=binary, feature_path=feature, diagnostic=True)
        preparation = json.loads((root / "out/case-0/template.json").read_text())["preparation"]
        if (preparation["api_complete"], preparation["write_calls"], preparation["write_rows"], preparation["pending"]) != (True, 2, 65, 0):
            raise RuntimeError("existing real preparation/drain controls failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed, NOT_RED")
        controls = json.loads((root / "out/time/controls.json").read_text())
        if controls["selected_cuda_uuid"] != GPU or (controls["governed_write_calls"], controls["write_rows"], controls["pending"]) != (1, 1, 0):
            raise RuntimeError("existing actual window-write/drain/device controls failed, NOT_RED")
        actual = controls["controls"]
        if [(row["stage"], row["valid_as_of"], row["ids"]) for row in actual] != [(stage, asof, ["l:gg-window"] if stage == "inside" else []) for stage, asof, _ in STAGES]:
            raise RuntimeError("actual literal public-view visibility controls failed, NOT_RED")
        if any(row["elapsed_ns"] <= 0 for row in actual) or controls["write_ns"] <= 0 or controls["drain_ns"] <= 0:
            raise RuntimeError("actual control API clocks failed, NOT_RED")
        if (controls["baseline_coverage"]["current_vector_cursors"], controls["after_coverage"]["current_vector_cursors"], controls["after_coverage"]["historical_physical_vectors"]) != (65, 66, 0):
            raise RuntimeError("existing physical/current/history controls failed, NOT_RED")
        with sqlite3.connect((root / "out/time/store.sqlite").as_uri() + "?mode=ro", uri=True) as connection:
            window = connection.execute("SELECT body,valid_from,valid_until FROM canonical_nodes WHERE logical_id='gg-window' AND superseded_at IS NULL").fetchall()
            joined = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.state='active' AND c.superseded_at IS NULL").fetchone()[0]
            visible = [connection.execute("SELECT count(*) FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL AND (valid_from IS NULL OR valid_from<=?) AND (valid_until IS NULL OR valid_until>?)", (asof, asof)).fetchone()[0] for _, asof, _ in STAGES[1:]]
        if window != [("s30ggwindow épreuve", 90, 110)] or joined != 66 or visible != [65, 66, 65]:
            raise RuntimeError("independent actual window/current/visible SQL controls failed, NOT_RED")
        observations = []
        for stage, asof, eligible in STAGES:
            directory = root / "out/time" / stage
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            proof_path = directory / "api-proof.json"
            proof = json.loads(proof_path.read_text()) if proof_path.exists() else {}
            observations.append((stage, asof, eligible, warm, timed, proof, issues))
        print(json.dumps({"root": str(root), "actual_stage_calls": {row[0]: len(row[3]) + len(row[4]) for row in observations}}), flush=True)
        engine_ids = set()
        call_ids = []
        generations = {}
        for stage, asof, eligible, warm, timed, proof, issues in observations:
            with self.subTest(stage=stage):
                self.assertEqual(len(timed), samples, "missing actual timed GG hybrid stage calls after successful public-view controls")
                self.assertEqual(len(warm), warmups)
                self.assertEqual(issues, [])
                self.assertEqual((proof["api_calls"], proof["warmup_calls"], proof["timed_calls"]), (warmups + samples, warmups, samples))
                self.assertEqual(proof["valid_as_of"], asof)
                self.assertEqual(proof["stage"], stage)
                for index, row in enumerate([*warm, *timed]):
                    self.assertTrue(row["ok"])
                    self.assertEqual((row["adapter"], row["mode"], row["stage"], row["valid_as_of"]), ("engine", "hybrid", stage, asof))
                    self.assertEqual(row["call_index"], index)
                    self.assertGreater(row["elapsed_ns"], 0)
                    self.assertEqual(row["query_fingerprint"], hashlib.sha256("s30ggwindow".encode()).hexdigest())
                    allowed = {"l:" + record["id"] for record in records}
                    if stage == "inside":
                        allowed.add("l:gg-window")
                    self.assertTrue(set(row["ids"]) <= allowed)
                    self.assertEqual(len(row["ids"]), len(set(row["ids"])))
                    self.assertEqual((row["returned_hits"], row["soft_fallback"], row["degradations"]), (10, "None", "[]"))
                    observation = row["observer"]
                    self.assertEqual((observation["status"], observation["dropped_delta"], observation["unpaired_records"]), ("observed", 0, 0))
                    self.assertEqual(len(observation["records"]), 1)
                    event = observation["records"][0]
                    self.assertEqual(event["snapshot"]["effective_at"], asof)
                    self.assertTrue(event["predicates"]["all_nodes_directly_eligible"]["invoked"])
                    self.assertEqual(event["predicates"]["all_nodes_directly_eligible"]["value"], eligible)
                    self.assertGreater(event["predicates"]["all_nodes_directly_eligible"]["elapsed_ns"], 0)
                    engine_ids.add(event["engine_id"])
                    call_ids.append(event["call_id"])
                    generations.setdefault(stage, set()).add(event["snapshot"]["generation"])
                self.assertEqual(proof["raw_persistence"]["records"], warmups + samples)
                self.assertEqual(proof["raw_persistence"]["checkpoints"], 2 if warmups else 1)
                self.assertLessEqual(sum(row["elapsed_ns"] for row in [*warm, *timed]), proof["loop_ns"])
        if len(generations) == 4:
            self.assertEqual(len(engine_ids), 1)
            self.assertEqual(call_ids, list(range(1, 4 * (warmups + samples) + 1)))
            self.assertTrue(all(len(values) == 1 for values in generations.values()))
            self.assertLess(next(iter(generations["baseline"])), next(iter(generations["before"])))
            self.assertEqual(generations["before"], generations["inside"])
            self.assertEqual(generations["inside"], generations["after"])

    def test_literal_baseline_then_before_inside_after_window_stages(self):
        self.require_stages(1, 2)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(case=st.sampled_from([(0, 1), (1, 3)]))
    def test_bounded_warmup_timed_counts_preserve_four_view_routes(self, case):
        self.require_stages(*case)


if __name__ == "__main__":
    unittest.main()
