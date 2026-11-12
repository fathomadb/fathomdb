"""Manual generic SDK G2 timed-group actuation, with unscheduled warmups and RO."""

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

MODES = ["sdk_text", "sdk_hybrid", "sdk_mean"]


class GenericActuationTests(unittest.TestCase):
    def fixture(self, calls, schedule):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-generic-actuation-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text("".join(json.dumps({"id": 1000 + index, "query": "alphaonly épreuve"}) + "\n"
                                   for index in range(calls)))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G_FEED_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "queries": str(queries),
                                     "modes": MODES, "population": "G2", "schedule": schedule, "warmups": 2}))
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
        identity = {"root": str(root), "calls": calls, "schedule": schedule,
                    "oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                    "queries_sha256": hashlib.sha256(queries.read_bytes()).hexdigest(),
                    "records_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
        (root / "oracle-identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        print(json.dumps(identity), flush=True)
        run(config, root, binary=binary, feature_path=task / "proofs/gd-g-host-feature-closure.json")
        preparation = json.loads((root / "out/case-0/template.json").read_text())["preparation"]
        if (preparation["api_complete"], preparation["write_rows"], preparation["pending"]) != (True, 2, 0):
            raise RuntimeError("existing actual closed-template preparation failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed, NOT_RED")
        observed = {}
        for mode in MODES:
            directory = root / "out" / mode
            proof = json.loads((directory / "api-proof.json").read_text())
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            phases = read_jsonl_records(directory / "batches.jsonl", issues)
            probes = read_jsonl_records(directory / "marker-probes.jsonl", issues)
            if issues or (len(warm), len(timed), proof["api_calls"], proof["timed_api_calls"], proof["warmup_api_calls"]) != (2, calls, calls + 2, calls, 2):
                raise RuntimeError("existing actual query/warmup framing controls failed, NOT_RED")
            if proof["selected_cuda_uuid"] != GPU or proof["cuda_compiled"] is not True:
                raise RuntimeError("required actual CUDA identity failed, NOT_RED")
            reset = proof["g2_reset"]
            if (reset["reset_write_calls"], reset["reset_rows"], reset["pending_after_drain"], reset["coverage"]["current_by_kind"]) != (1, 10, 0, {"doc": 2, "note": 10}):
                raise RuntimeError("existing reset/drain/current controls failed, NOT_RED")
            for row in [*warm, *timed]:
                if row["ok"] is not True or row["elapsed_ns"] <= 0 or row["query_fingerprint"] != hashlib.sha256("alphaonly épreuve".encode()).hexdigest():
                    raise RuntimeError("existing actual API/input/timer controls failed, NOT_RED")
                if mode == "sdk_mean":
                    if (row["dimension"], row["finite"]) != (384, True) or row["norm_squared"] <= 0 or "kind_filter" in row:
                        raise RuntimeError("existing unfiltered actual mean controls failed, NOT_RED")
                elif row["kind_filter"] != "doc" or row["ids"] != (["l:alpha"] if mode == "sdk_text" else row["ids"]):
                    raise RuntimeError("existing doc-kind/literal text controls failed, NOT_RED")
                elif mode == "sdk_hybrid" and (set(row["ids"]), row["returned_hits"], row["soft_fallback"], row["degradations"]) != ({"l:alpha", "l:beta"}, 2, "None", "[]"):
                    raise RuntimeError("existing eligible non-degraded hybrid controls failed, NOT_RED")
            if [row["query_id"] for row in timed] != list(range(1000, 1000 + calls)) or [row["call_index"] for row in [*warm, *timed]] != list(range(calls + 2)):
                raise RuntimeError("existing actual query/count/order controls failed, NOT_RED")
            with sqlite3.connect((directory / "store.sqlite").as_uri() + "?mode=ro", uri=True) as connection:
                counts = connection.execute("SELECT c.kind,count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.state='active' AND c.superseded_at IS NULL GROUP BY c.kind ORDER BY c.kind").fetchall()
                notes = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL ORDER BY logical_id").fetchall()
                mappings = connection.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0]
            if counts != [("doc", 2), ("note", 10)]:
                raise RuntimeError("independent actual current cursor/span controls failed, NOT_RED")
            observed[mode] = (directory, proof, warm, timed, phases, probes, notes, mappings)
        print(json.dumps({"root": str(root), "actual_query_calls_per_mode": calls + 2,
                          "actual_governed_cohorts": {mode: sum(row.get("event") == "governed_write" for row in rows[4]) for mode, rows in observed.items()},
                          "actual_probe_rows": {mode: len(rows[5]) for mode, rows in observed.items()}}), flush=True)
        return observed

    def require_actuation(self, calls):
        observations = self.fixture(calls, "write")
        cohorts = calls // 10
        for mode in MODES:
            with self.subTest(mode=mode):
                directory, proof, warm, timed, phases, probes, notes, mappings = observations[mode]
                writes = [row for row in phases if row.get("event") == "governed_write"]
                self.assertEqual(len(writes), cohorts, "missing actual generic G2 governed cohorts after completed TIMED groups")
                self.assertEqual(len(probes), cohorts * 10)
                self.assertEqual([row["event"] for row in phases], ["governed_write", "projection_drained", "active_coverage"] * cohorts)
                for end in range(len(phases) + 1):
                    events = [row["event"] for row in phases[:end]]
                    written, drained, covered = [events.count(event) for event in ["governed_write", "projection_drained", "active_coverage"]]
                    self.assertTrue(covered <= drained <= written)
                    self.assertLessEqual(written - covered, 1)
                    self.assertEqual(written == drained == covered == cohorts, end == len(phases))
                previous = proof["g2_placement"]["reset_end_ns"]
                for row in [*warm, *timed]:
                    self.assertLessEqual(previous, row["record_start_ns"])
                    self.assertLess(row["record_start_ns"], row["record_ready_ns"])
                    previous = row["record_ready_ns"]
                self.assertEqual([row["sample_index"] for row in timed], list(range(calls)))
                for cohort in range(cohorts):
                    completed = (cohort + 1) * 10
                    write, drain, coverage = phases[cohort * 3:cohort * 3 + 3]
                    self.assertEqual([row["after_natural_calls"] for row in [write, drain, coverage]], [completed] * 3)
                    self.assertEqual([row["cohort"] for row in [write, drain, coverage]], [cohort] * 3)
                    self.assertLessEqual(timed[completed - 1]["record_ready_ns"], write["start_ns"])
                    self.assertEqual(timed[completed - 1]["call_index"], completed + 1)
                    self.assertEqual(write["rows"], 10)
                    self.assertGreater(write["cursor"], 0)
                    self.assertLess(write["start_ns"], write["end_ns"])
                    self.assertEqual(write["write_ns"], write["end_ns"] - write["start_ns"])
                    self.assertLessEqual(write["end_ns"], drain["start_ns"])
                    self.assertLess(drain["start_ns"], drain["end_ns"])
                    self.assertEqual(drain["drain_ns"], drain["end_ns"] - drain["start_ns"])
                    self.assertEqual(drain["pending_after_drain"], 0)
                    previous = drain["end_ns"]
                    for slot, probe in enumerate(probes[cohort * 10:cohort * 10 + 10]):
                        self.assertEqual((probe["slot"], probe["cohort"], probe["after_natural_calls"]), (slot, cohort, completed))
                        self.assertEqual((probe["ok"], probe["marker_current"]), (True, True))
                        self.assertEqual(probe["ids"], [f"l:s30marker-{slot}"])
                        self.assertEqual(probe["bodies"], [f"s30markercurrent{slot} revision{cohort}"])
                        self.assertLessEqual(previous, probe["start_ns"])
                        self.assertLess(probe["start_ns"], probe["end_ns"])
                        self.assertEqual(probe["proof_ns"], probe["end_ns"] - probe["start_ns"])
                        previous = probe["end_ns"]
                    self.assertLessEqual(previous, coverage["start_ns"])
                    self.assertLess(coverage["start_ns"], coverage["end_ns"])
                    self.assertEqual(coverage["current_by_kind"], {"doc": 2, "note": 10})
                    self.assertEqual((coverage["current_vector_cursors"], coverage["total_physical_vectors"], coverage["historical_physical_vectors"]), (12, 12 + (cohort + 1) * 10, (cohort + 1) * 10))
                    if completed < calls:
                        self.assertLessEqual(coverage["end_ns"], timed[completed]["record_start_ns"])
                self.assertEqual(notes, [(f"s30marker-{slot}", f"s30markercurrent{slot} revision{cohorts - 1}") for slot in range(10)])
                self.assertEqual(mappings, 12 + cohorts * 10)
                self.assertEqual(proof["schedule"], {"mode": "write", "natural_calls": calls, "scheduled_write_calls": cohorts, "scheduled_write_rows": cohorts * 10, "marker_probes": cohorts * 10})
                self.assertEqual(proof["raw_persistence"]["records"], calls + 2 + cohorts * 13)
                self.assertEqual(proof["raw_persistence"]["checkpoints"], 1 + cohorts * 5)
                self.assertLessEqual(sum(row["elapsed_ns"] for row in [*warm, *timed]), proof["loop_ns"])
                self.assertEqual(proof["throughput_scope"], "warmup and timed API calls plus input identity/validation/serialization/checkpoints/scheduled writes/drain/probes/coverage; reset/open/close separate")

    def test_literal_three_sdk_modes_schedule_after_timed10_and20_not_global8_and18(self):
        self.require_actuation(20)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(calls=st.sampled_from([10, 30]))
    def test_bounded_timed_groups_preserve_actual_phase_prefix_counts_and_order(self, calls):
        self.require_actuation(calls)

    def test_readonly_control_has_no_scheduled_work_or_write_record_schema(self):
        for _, proof, warm, timed, phases, probes, notes, mappings in self.fixture(20, "readonly").values():
            self.assertEqual((phases, probes), ([], []))
            self.assertNotIn("schedule", proof)
            self.assertEqual(notes, [(f"s30marker-{slot}", f"s30markercurrent{slot} revisionreset") for slot in range(10)])
            self.assertEqual(mappings, 12)
            self.assertEqual(proof["raw_persistence"]["records"], 22)
            for row in [*warm, *timed]:
                self.assertNotIn("record_start_ns", row)
                self.assertNotIn("record_ready_ns", row)


if __name__ == "__main__":
    unittest.main()
