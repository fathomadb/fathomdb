"""Manual generic G2 mode/schedule sequence on one writable cell-store copy."""

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


class G2PassPlanTests(unittest.TestCase):
    def require_plan(self, repeat):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-g2-pass-plan-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text("".join(json.dumps({"id": 1000 + index, "query": "alphaonly épreuve"}) + "\n"
                                   for index in range(20)))
        modes = ["sdk_hybrid", "sdk_text", "sdk_mean"]
        schedules = ["readonly", "write"]
        if repeat:
            modes.reverse()
            schedules.reverse()
        pairs = [(mode, schedule) for mode in modes for schedule in schedules]
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G1_PLAN_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "queries": str(queries),
                                     "mode": pairs[0][0], "passes": [mode for mode, _ in pairs],
                                     "schedules": [schedule for _, schedule in pairs],
                                     "population": "G2", "schedule": "readonly", "warmups": 2}))
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
        identity = {"root": str(root), "repeat": repeat,
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
        store = root / "out/cell/store.sqlite"
        if list((root / "out/cell").rglob("store.sqlite")) != [store]:
            raise RuntimeError("existing one-cell-copy routing failed, NOT_RED")
        observed = []
        for ordinal, (mode, _) in enumerate(pairs):
            directory = root / f"out/cell/pass-{ordinal}"
            proof = json.loads((directory / "api-proof.json").read_text())
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            phases = read_jsonl_records(directory / "batches.jsonl", issues)
            probes = read_jsonl_records(directory / "marker-probes.jsonl", issues)
            if issues or (len(warm), len(timed), proof["api_calls"], proof["timed_api_calls"], proof["warmup_api_calls"]) != (2, 20, 22, 20, 2):
                raise RuntimeError("existing actual pass call/framing controls failed, NOT_RED")
            if (proof["mode"], proof["store"], proof["selected_cuda_uuid"], proof["cuda_compiled"]) != (mode, str(store), GPU, True):
                raise RuntimeError("existing mode/store/actual CUDA identity failed, NOT_RED")
            reset = proof["g2_reset"]
            if (reset["reset_write_calls"], reset["reset_rows"], reset["pending_after_drain"], reset["coverage"]["current_by_kind"]) != (1, 10, 0, {"doc": 2, "note": 10}):
                raise RuntimeError("existing actual reset/drain/current controls failed, NOT_RED")
            for row in [*warm, *timed]:
                if row["ok"] is not True or row["elapsed_ns"] <= 0 or row["query_fingerprint"] != hashlib.sha256("alphaonly épreuve".encode()).hexdigest():
                    raise RuntimeError("existing actual API/input/timer controls failed, NOT_RED")
                if mode == "sdk_mean":
                    if (row["dimension"], row["finite"]) != (384, True) or row["norm_squared"] <= 0 or "kind_filter" in row:
                        raise RuntimeError("existing unfiltered mean values failed, NOT_RED")
                elif row["kind_filter"] != "doc":
                    raise RuntimeError("existing doc-kind restriction failed, NOT_RED")
                elif mode == "sdk_text" and row["ids"] != ["l:alpha"]:
                    raise RuntimeError("existing literal text result failed, NOT_RED")
                elif mode == "sdk_hybrid" and (set(row["ids"]), row["soft_fallback"], row["degradations"]) != ({"l:alpha", "l:beta"}, "None", "[]"):
                    raise RuntimeError("existing eligible hybrid values failed, NOT_RED")
            if [row["query_id"] for row in timed] != list(range(1000, 1020)):
                raise RuntimeError("existing actual query order failed, NOT_RED")
            observed.append((proof, warm, timed, phases, probes))
        with sqlite3.connect(store.as_uri() + "?mode=ro", uri=True) as connection:
            counts = connection.execute("SELECT c.kind,count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.state='active' AND c.superseded_at IS NULL GROUP BY c.kind ORDER BY c.kind").fetchall()
            mappings = connection.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0]
            notes = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL ORDER BY logical_id").fetchall()
        actual_cohorts = [sum(row.get("event") == "governed_write" for row in rows[3]) for rows in observed]
        if counts != [("doc", 2), ("note", 10)] or mappings != 62 + sum(actual_cohorts) * 10:
            raise RuntimeError("independent current/retained cursor accounting failed, NOT_RED")
        print(json.dumps({"root": str(root), "pairs": pairs, "actual_cohorts": actual_cohorts,
                          "actual_retained_mappings": mappings}), flush=True)
        self.assertEqual(actual_cohorts, [2 if schedule == "write" else 0 for _, schedule in pairs],
                         "missing actual per-pass G2 schedule forwarding on one cell copy")
        prior_note_rows = 0
        for (_, schedule), (proof, warm, timed, phases, probes) in zip(pairs, observed, strict=True):
            self.assertEqual(proof["g2_reset"]["coverage"]["historical_physical_vectors"], prior_note_rows)
            self.assertEqual(proof["g2_reset"]["coverage"]["total_physical_vectors"], 12 + prior_note_rows)
            placement = proof["g2_placement"]
            self.assertLessEqual(placement["reset_end_ns"], placement["first_warmup_record_start_ns"])
            self.assertLess(placement["first_warmup_record_start_ns"], placement["first_timed_record_start_ns"])
            prior_note_rows += 10
            if schedule == "readonly":
                self.assertEqual((phases, probes), ([], []))
                self.assertNotIn("schedule", proof)
                for row in [*warm, *timed]:
                    self.assertNotIn("record_start_ns", row)
            else:
                self.assertEqual([row["event"] for row in phases], ["governed_write", "projection_drained", "active_coverage"] * 2)
                self.assertEqual(len(probes), 20)
                self.assertEqual(proof["schedule"]["scheduled_write_calls"], 2)
                self.assertEqual(proof["schedule"]["scheduled_write_rows"], 20)
                for cohort in range(2):
                    completed = (cohort + 1) * 10
                    write, _, coverage = phases[cohort * 3:cohort * 3 + 3]
                    self.assertLessEqual(timed[completed - 1]["record_ready_ns"], write["start_ns"])
                    self.assertEqual(write["after_natural_calls"], completed)
                    self.assertEqual(coverage["current_by_kind"], {"doc": 2, "note": 10})
                    self.assertEqual(coverage["historical_physical_vectors"], prior_note_rows + cohort * 10)
                    if completed < 20:
                        self.assertLessEqual(coverage["end_ns"], timed[completed]["record_start_ns"])
                prior_note_rows += 20
        self.assertEqual((mappings, prior_note_rows), (122, 120))
        final_revision = "revision1" if pairs[-1][1] == "write" else "revisionreset"
        self.assertEqual(notes, [(f"s30marker-{slot}", f"s30markercurrent{slot} {final_revision}") for slot in range(10)])
        self.assertEqual(sum(len(rows[4]) for rows in observed), 60)

    def test_literal_forward_sdk_modes_with_readonly_then_write_on_one_copy(self):
        self.require_plan(0)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(repeat=st.integers(min_value=0, max_value=1))
    def test_bounded_repeat_reverses_modes_and_schedules_preserving_history(self, repeat):
        self.require_plan(repeat)


if __name__ == "__main__":
    unittest.main()
