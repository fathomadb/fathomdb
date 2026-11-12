"""Manual GO transport oracle: distinct GN, same-artifact off/on, real Core errors."""

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
from test_g_query_feed import QUESTIONS, TEXT_IDS

MODES = ["engine_hybrid", "engine_text", "engine_mean"]


class ObserverTests(unittest.TestCase):
    def series(self, artifact, enabled, questions):
        artifacts = Path(os.environ["S30_GO_ARTIFACTS"])
        binary = artifacts / artifact
        feature = artifacts / f"{artifact}-features.json"
        closure = json.loads(feature.read_text())
        diagnostic = artifact == "go"
        if closure["normal_gn"] is not (not diagnostic) or closure["binary_sha256"] != hashlib.sha256(binary.read_bytes()).hexdigest():
            raise RuntimeError("actual requested GN/GO artifact identity failed, NOT_RED")
        root = Path(tempfile.mkdtemp(prefix=f"s30-go-{artifact}-{'on' if enabled else 'off'}-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, 65)]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text("".join(json.dumps({"id": identity, "query": query}) + "\n" for identity, query in questions))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G_FEED_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "queries": str(queries),
                                     "modes": MODES, "warmups": 1, "observer": enabled,
                                     "error_controls": diagnostic}))
        identity = {"root": str(root), "artifact": artifact, "observer_enabled": enabled,
                    "oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "feature_sha256": hashlib.sha256(feature.read_bytes()).hexdigest(),
                    "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                    "queries_sha256": hashlib.sha256(queries.read_bytes()).hexdigest(),
                    "records_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
        (root / "oracle-identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        print(json.dumps(identity), flush=True)
        run(config, root, binary=binary, feature_path=feature, diagnostic=diagnostic)
        preparation = json.loads((root / "out/case-0/template.json").read_text())["preparation"]
        if (preparation["api_complete"], preparation["write_calls"], preparation["write_rows"], preparation["pending"]) != (True, 2, 65, 0):
            raise RuntimeError("existing actual preparation/drain controls failed, NOT_RED")
        if len(preparation["coverage"]["rows"]) != 65 or preparation["coverage"]["profile"]["dimension"] != 384:
            raise RuntimeError("existing actual physical/profile controls failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed, NOT_RED")
        actual_queries = {identity: query for identity, query in questions}
        allowed = {"l:" + row["id"] for row in records}
        observations = {}
        for mode in MODES:
            directory = root / "out" / mode
            proof = json.loads((directory / "api-proof.json").read_text())
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            if issues or (len(warm), len(timed), proof["api_calls"], proof["timed_api_calls"], proof["warmup_api_calls"]) != (1, len(questions), len(questions) + 1, len(questions), 1):
                raise RuntimeError("existing actual query framing/count controls failed, NOT_RED")
            if proof["selected_cuda_uuid"] != GPU or proof["cuda_compiled"] is not True:
                raise RuntimeError("required actual Default CUDA identity failed, NOT_RED")
            for row in [*warm, *timed]:
                query = actual_queries[row["query_id"]]
                if (row["ok"], row["adapter"]) != (True, "engine") or row["elapsed_ns"] <= 0 or row["query_fingerprint"] != hashlib.sha256(query.encode()).hexdigest():
                    raise RuntimeError("existing successful Core API/input/timer controls failed, NOT_RED")
                if mode == "engine_mean":
                    if (row["dimension"], row["finite"]) != (384, True) or row["norm_squared"] <= 0:
                        raise RuntimeError("existing actual mean controls failed, NOT_RED")
                elif mode == "engine_text":
                    if row["ids"] != TEXT_IDS[query]:
                        raise RuntimeError("existing literal direct-text controls failed, NOT_RED")
                elif len(row["ids"]) != len(set(row["ids"])) or not set(row["ids"]) <= allowed or (row["returned_hits"], row["soft_fallback"], row["degradations"]) != (10, "None", "[]"):
                    raise RuntimeError("existing actual hybrid controls failed, NOT_RED")
            if [row["query_id"] for row in timed] != [identity for identity, _ in questions]:
                raise RuntimeError("existing actual forwarding/order failed, NOT_RED")
            with sqlite3.connect((directory / "store.sqlite").as_uri() + "?mode=ro", uri=True) as connection:
                joined = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.kind='doc' AND c.superseded_at IS NULL").fetchone()[0]
            if joined != 65:
                raise RuntimeError("independent closed-copy current-cursor controls failed, NOT_RED")
            observations[mode] = (proof, warm, timed)
        errors = []
        if diagnostic:
            error_proof = json.loads((root / "out/error-controls/api-proof.json").read_text())
            errors = error_proof["controls"]
            if error_proof["selected_cuda_uuid"] != GPU or [row["error_class"] for row in errors] != ["InvalidFilter", "FrozenRead"]:
                raise RuntimeError("ordinary Core typed error controls failed, NOT_RED")
            for row in errors:
                if row["ok"] is not False or row["elapsed_ns"] <= 0 or row["error_class"] not in row["raw_error"]:
                    raise RuntimeError("original typed errors/timing not retained, NOT_RED")
            if "StateDrifted" not in errors[1]["raw_error"]:
                raise RuntimeError("ordinary frozen refusal reason was changed, NOT_RED")
        print(json.dumps({"root": str(root), "artifact": artifact, "enabled": enabled,
                          "actual_query_calls": sum(row[0]["api_calls"] for row in observations.values()),
                          "actual_query_records": {mode: sum(len(row.get("observer", {}).get("records", [])) for row in [*values[1], *values[2]]) for mode, values in observations.items()},
                          "actual_error_classes": [row["error_class"] for row in errors]}), flush=True)
        return observations, errors

    def require_observer(self, questions):
        gn, _ = self.series("gn", False, questions)
        off, off_errors = self.series("go", False, questions)
        on, errors = self.series("go", True, questions)
        for _, warm, timed in gn.values():
            for row in [*warm, *timed]:
                self.assertNotIn("observer", row, "GN must remain a distinct untraced population")
        for _, warm, timed in off.values():
            for row in [*warm, *timed]:
                self.assertEqual(row.get("observer", {}).get("records", []), [])
        for row in off_errors:
            self.assertEqual(row.get("observer", {}).get("records", []), [])
        engine_ids = set()
        for mode in ["engine_hybrid", "engine_text"]:
            with self.subTest(mode=mode):
                proof, warm, timed = on[mode]
                call_ids = []
                owners = set()
                previous_dropped = 0
                for row in [*warm, *timed]:
                    observation = row.get("observer", {})
                    records = observation.get("records", [])
                    self.assertEqual(len(records), 1, "missing correlated GO query record after successful real Core call")
                    record = records[0]
                    self.assertEqual(observation["status"], "observed")
                    self.assertEqual(observation["unpaired_records"], 0)
                    self.assertEqual(observation["capacity"], 32768)
                    self.assertEqual(observation["dropped_before"], previous_dropped)
                    self.assertEqual(observation["dropped_delta"], observation["dropped_after"] - previous_dropped)
                    self.assertEqual(observation["dropped_delta"], 0)
                    previous_dropped = observation["dropped_after"]
                    self.assertGreater(observation["transport_ns"], 0)
                    self.assertEqual((record["event"], record["outcome"], record["error_class"]), ("QueryComplete", "ok", None))
                    self.assertTrue(record["snapshot_available"])
                    self.assertEqual(record["snapshot"]["event"], "SnapshotStamp")
                    self.assertIsNone(record["snapshot_unavailable_reason"])
                    self.assertTrue(record["predicates"]["has_source_dependencies"]["invoked"])
                    self.assertIsInstance(record["worker"], int)
                    self.assertGreater(record["engine_id"], 0)
                    owners.add(record["engine_id"])
                    call_ids.append(record["call_id"])
                self.assertEqual(len(owners), 1)
                self.assertTrue(engine_ids.isdisjoint(owners))
                engine_ids.update(owners)
                self.assertEqual(call_ids, list(range(1, len(questions) + 2)))
                self.assertLessEqual(sum(row["elapsed_ns"] for row in [*warm, *timed]), proof["loop_ns"])
        for populations in [off, on]:
            with self.subTest(mode="engine_mean", enabled=populations is on):
                for row in [*populations["engine_mean"][1], *populations["engine_mean"][2]]:
                    observation = row.get("observer", {})
                    self.assertEqual(observation.get("status"), "not_applicable",
                                     "missing GO mean not-applicable disposition after real mean call")
                    self.assertEqual(observation["records"], [])
                    self.assertEqual(observation["dropped_delta"], 0)
        for row in errors:
            with self.subTest(error=row["error_class"]):
                records = row.get("observer", {}).get("records", [])
                self.assertEqual(len(records), 1,
                                 "missing correlated raw typed-error GO record after actual API refusal")
                record = records[0]
                self.assertEqual((record["outcome"], record["error_class"]), ("error", row["error_class"]))
                self.assertEqual(row["observer"]["dropped_delta"], 0)
                if row["error_class"] == "FrozenRead":
                    self.assertFalse(record["snapshot_available"])
                    self.assertIsNone(record["snapshot"])
                    self.assertEqual(record["snapshot_unavailable_reason"], "not_reached")
                else:
                    self.assertTrue(record["snapshot_available"])
        if all(len(row.get("observer", {}).get("records", [])) == 1 for row in errors):
            invalid, frozen = [row["observer"]["records"][0] for row in errors]
            self.assertEqual([invalid["call_id"], frozen["call_id"]], [1, 2])
            self.assertEqual(invalid["engine_id"], frozen["engine_id"])
            self.assertNotIn(invalid["engine_id"], engine_ids)

    def test_literal_gn_off_on_and_real_typed_error_transport(self):
        self.require_observer(QUESTIONS)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(questions=st.permutations(QUESTIONS))
    def test_bounded_query_permutations_preserve_call_ownership_and_snapshot_dispositions(self, questions):
        self.require_observer(questions)


if __name__ == "__main__":
    unittest.main()
