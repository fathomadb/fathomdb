"""Manual GG provenance/Deleted stages with independent retained/visible inventory."""

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


class GovernedStageTests(unittest.TestCase):
    def require_stages(self, warmups, samples):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-gg-governed-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, 65)]
        if any("s30gggoverned" in row["body"] for row in records):
            raise RuntimeError("literal control term not absent from base, NOT_RED")
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(row) + "\n" for row in records))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "GG_GOVERNED_FIXTURE", "gpu_uuid": GPU,
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
        if (preparation["api_complete"], preparation["write_rows"], preparation["pending"]) != (True, 65, 0):
            raise RuntimeError("existing real Default preparation failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed, NOT_RED")
        controls = json.loads((root / "out/governed/controls.json").read_text())
        if controls["selected_cuda_uuid"] != GPU or (controls["governed_write_calls"], controls["write_rows"], controls["dependency_registrations"], controls["transitions"], controls["pending"]) != (1, 2, 1, 1, 0):
            raise RuntimeError("existing real write/register/lifecycle/drain controls failed, NOT_RED")
        if [(row["stage"], row["valid_as_of"], row["ids"]) for row in controls["controls"]] != [("valid", 100, ["l:gg-derived", "l:gg-source"]), ("deleted", 100, [])]:
            raise RuntimeError("actual literal public-view eligibility failed, NOT_RED")
        for name in ["write_ns", "register_ns", "valid_drain_ns", "transition_ns", "deleted_drain_ns"]:
            if controls[name] <= 0:
                raise RuntimeError("actual ordinary API clock failed, NOT_RED")
        for name in ["valid_readiness", "deleted_readiness"]:
            if controls[name] != {"state": "ready", "usable_embedder": True, "pending_count": 0, "affected_kinds": [], "blocked": None}:
                raise RuntimeError("actual public readiness control failed, NOT_RED")
        for deleted, inventory in [(False, controls["valid_inventory"]), (True, controls["deleted_inventory"])]:
            expected = (67, {"active": 65, "deleted": 2}, 0, 66, 66, 66, 1, 1) if deleted else (67, {"active": 67, "deleted": 0}, 0, 67, 67, 67, 0, 0)
            actual = tuple(inventory[name] for name in ["canonical_rows", "current_by_state", "superseded_history", "mapped_vectors", "physical_vectors", "span_rows", "retained_deleted_source_vectors", "closed_derived_without_vectors"])
            if actual != expected or inventory["profile"]["dimension"] != 384:
                raise RuntimeError("existing observed canonical/retained/physical/profile inventory failed, NOT_RED")
            by_id = {row["id"]: row for row in inventory["rows"]}
            for name in ["gg-source", "gg-derived"]:
                if name not in by_id:
                    raise RuntimeError("required actual source/derived row absent, NOT_RED")
                row = by_id[name]
                count = 0 if deleted and name == "gg-derived" else 1
                if (row["state"], row["superseded_at"], row["mapping_rows"], row["span_rows"], row["node_fts_rows"], row["node_fts_v2_rows"]) != ("deleted" if deleted else "active", None, count, count, count, count):
                    raise RuntimeError("actual per-row lifecycle/projection disposition failed, NOT_RED")
            if inventory["closure"] != ([["complete", 1]] if deleted else []):
                raise RuntimeError("actual complete derived closure proof failed, NOT_RED")
        with sqlite3.connect((root / "out/governed/store.sqlite").as_uri() + "?mode=ro", uri=True) as connection:
            states = connection.execute("SELECT state,count(*) FROM canonical_nodes WHERE superseded_at IS NULL GROUP BY state ORDER BY state").fetchall()
            known = connection.execute("SELECT c.logical_id,c.kind,c.write_cursor,c.state,(SELECT count(*) FROM _fathomdb_vector_rows v WHERE v.kind=c.kind AND v.write_cursor=c.write_cursor),(SELECT count(*) FROM _fathomdb_vector_spans s WHERE s.write_cursor=c.write_cursor) FROM canonical_nodes c WHERE logical_id IN ('gg-source','gg-derived') ORDER BY logical_id").fetchall()
            totals = [connection.execute(f"SELECT count(*) FROM {table}").fetchone()[0] for table in ["canonical_nodes", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]]
            closure = connection.execute("SELECT phase,affected_count FROM _fathomdb_dependency_closures ORDER BY closure_sequence").fetchall()
            if states != [("active", 65), ("deleted", 2)] or totals != [67, 66, 66] or closure != [("complete", 1)]:
                raise RuntimeError("independent actual terminal current/retained/closure SQL failed, NOT_RED")
            if [(row[0], row[1], row[3], row[4], row[5]) for row in known] != [("gg-derived", "note", "deleted", 0, 0), ("gg-source", "doc", "deleted", 1, 1)]:
                raise RuntimeError("independent source-shadow versus derived-erasure SQL failed, NOT_RED")
        observations = []
        for stage in ["valid", "deleted"]:
            directory = root / "out/governed" / stage
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            proof_path = directory / "api-proof.json"
            proof = json.loads(proof_path.read_text()) if proof_path.exists() else {}
            observations.append((stage, warm, timed, proof, issues))
        print(json.dumps({"root": str(root), "actual_stage_calls": {row[0]: len(row[1]) + len(row[2]) for row in observations}}), flush=True)
        owners = set()
        call_ids = []
        generations = {}
        for stage, warm, timed, proof, issues in observations:
            with self.subTest(stage=stage):
                self.assertEqual(len(timed), samples, "missing actual governed GG measured stage calls after successful lifecycle controls")
                self.assertEqual(len(warm), warmups)
                self.assertEqual(issues, [])
                self.assertEqual((proof["api_calls"], proof["warmup_calls"], proof["timed_calls"], proof["valid_as_of"]), (warmups + samples, warmups, samples, 100))
                for index, row in enumerate([*warm, *timed]):
                    self.assertTrue(row["ok"])
                    self.assertEqual((row["stage"], row["valid_as_of"], row["call_index"]), (stage, 100, index))
                    self.assertEqual(row["query_fingerprint"], hashlib.sha256("s30gggoverned".encode()).hexdigest())
                    self.assertGreater(row["elapsed_ns"], 0)
                    observation = row["observer"]
                    self.assertEqual((observation["status"], observation["dropped_delta"], observation["unpaired_records"]), ("observed", 0, 0))
                    self.assertEqual(len(observation["records"]), 1)
                    event = observation["records"][0]
                    self.assertEqual(event["snapshot"]["effective_at"], 100)
                    self.assertTrue(event["predicates"]["has_source_dependencies"]["invoked"])
                    self.assertTrue(event["predicates"]["has_source_dependencies"]["value"])
                    fallback = event["predicates"]["vector_arm_requires_fallback"]
                    self.assertTrue(fallback["invoked"])
                    self.assertEqual(fallback["value"], stage == "deleted")
                    self.assertTrue(fallback["sql"]["invoked"])
                    self.assertEqual(fallback["sql"]["value"], stage == "deleted")
                    self.assertGreater(fallback["sql"]["elapsed_ns"], 0)
                    skipped = event["predicates"]["all_nodes_directly_eligible"]
                    self.assertFalse(skipped["invoked"])
                    self.assertEqual(skipped["reason"], "source_dependencies")
                    self.assertIsNone(skipped["value"])
                    self.assertIsNone(skipped["elapsed_ns"])
                    vector = event["blocks"]["vector"]
                    if stage == "deleted":
                        self.assertEqual(row["ids"], [])
                        self.assertIn("Vector", row["soft_fallback"])
                        self.assertFalse(vector["invoked"])
                        self.assertTrue(vector["degraded"])
                        self.assertEqual(vector["reason"], "eligibility_degraded")
                        self.assertIsNone(vector["rows"])
                    else:
                        self.assertEqual(row["soft_fallback"], "None")
                        self.assertTrue(vector["invoked"])
                        self.assertFalse(vector["degraded"])
                        self.assertEqual((row["returned_hits"], len(row["ids"])), (10, 10))
                        self.assertTrue(set(row["ids"]) <= {"l:" + record["id"] for record in records} | {"l:gg-source", "l:gg-derived"})
                    owners.add(event["engine_id"])
                    call_ids.append(event["call_id"])
                    generations.setdefault(stage, set()).add(event["snapshot"]["generation"])
                self.assertEqual(proof["raw_persistence"]["records"], warmups + samples)
                self.assertLessEqual(sum(row["elapsed_ns"] for row in [*warm, *timed]), proof["loop_ns"])
        if len(generations) == 2:
            self.assertEqual(len(owners), 1)
            self.assertEqual(call_ids, list(range(1, 2 * (warmups + samples) + 1)))
            self.assertTrue(all(len(values) == 1 for values in generations.values()))
            self.assertLess(next(iter(generations["valid"])), next(iter(generations["deleted"])))

    def test_literal_valid_then_deleted_dependency_stages(self):
        self.require_stages(1, 2)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(case=st.sampled_from([(0, 1), (1, 3)]))
    def test_bounded_counts_keep_lifecycle_and_sparse_degradation_explicit(self, case):
        self.require_stages(*case)


if __name__ == "__main__":
    unittest.main()
