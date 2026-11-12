"""Manual generic SDK per-pass reset on one owned copy, before warmup/timed calls."""

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


class GenericResetTests(unittest.TestCase):
    def require_reset(self, documents, warmups):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-generic-reset-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, documents)]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text("".join(json.dumps({"id": identity, "query": query}) + "\n"
                                   for identity, query in QUESTIONS))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G1_PLAN_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "queries": str(queries),
                                     "mode": "sdk_text", "passes": ["sdk_text", "sdk_text"],
                                     "schedule": "readonly", "population": "G2", "warmups": warmups}))
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
        identity = {"root": str(root), "documents": documents, "warmups": warmups,
                    "oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                    "queries_sha256": hashlib.sha256(queries.read_bytes()).hexdigest(),
                    "records_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
        (root / "oracle-identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        print(json.dumps(identity), flush=True)
        run(config, root, binary=binary, feature_path=task / "proofs/gd-g-host-feature-closure.json")
        preparation = json.loads((root / "out/case-0/template.json").read_text())["preparation"]
        if (preparation["api_complete"], preparation["write_rows"], preparation["write_calls"],
                preparation["pending"]) != (True, documents, (documents + 63) // 64, 0):
            raise RuntimeError("existing real preparation/drain/close controls failed, NOT_RED")
        if len(preparation["coverage"]["rows"]) != documents or preparation["coverage"]["profile"]["dimension"] != 384:
            raise RuntimeError("existing physical/profile controls failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed, NOT_RED")
        store = root / "out/cell/store.sqlite"
        observed = []
        for ordinal in range(2):
            directory = root / f"out/cell/pass-{ordinal}"
            proof = json.loads((directory / "api-proof.json").read_text())
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            if issues or (len(warm), len(timed), proof["api_calls"], proof["warmup_api_calls"], proof["timed_api_calls"]) != (warmups, 3, warmups + 3, warmups, 3):
                raise RuntimeError("existing successful warmup/timed framing controls failed, NOT_RED")
            if proof["selected_cuda_uuid"] != GPU or proof["cuda_compiled"] is not True or proof["store"] != str(store):
                raise RuntimeError("required actual CUDA/same-copy identity failed, NOT_RED")
            for row in [*warm, *timed]:
                if row["ok"] is not True or row["elapsed_ns"] <= 0 or row["ids"] != TEXT_IDS[QUESTIONS[row["sample_index"]][1]]:
                    raise RuntimeError("existing actual literal text API controls failed, NOT_RED")
            if [row["query_id"] for row in timed] != [identity for identity, _ in QUESTIONS]:
                raise RuntimeError("existing actual query forwarding failed, NOT_RED")
            if proof["raw_persistence"]["records"] != warmups + 3:
                raise RuntimeError("existing persisted record count failed, NOT_RED")
            observed.append(proof)
        with sqlite3.connect(store.as_uri() + "?mode=ro", uri=True) as connection:
            docs = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='doc' AND state='active' AND superseded_at IS NULL ORDER BY write_cursor").fetchall()
            joined_docs = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.kind='doc' AND c.superseded_at IS NULL").fetchone()[0]
            notes = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='note' AND state='active' AND superseded_at IS NULL ORDER BY logical_id").fetchall()
            current_notes = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.kind='note' AND c.superseded_at IS NULL").fetchone()[0]
            historical = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor WHERE c.superseded_at IS NOT NULL").fetchone()[0]
            total = connection.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0]
        if docs != [(row["id"], row["body"]) for row in records] or joined_docs != documents:
            raise RuntimeError("independent canonical/doc cursor/span copy controls failed, NOT_RED")
        print(json.dumps({"root": str(root), "actual_current_notes": len(notes), "actual_history": historical,
                          "actual_retained_mapping_rows": total, "completed_successful_passes": len(observed)}), flush=True)
        self.assertEqual(len(notes), 10, "missing actual governed generic per-pass reset")
        self.assertEqual(notes, [(f"s30marker-{slot}", f"s30markercurrent{slot} revisionreset") for slot in range(10)])
        self.assertEqual((current_notes, historical, total), (10, 10, documents + 20))
        for ordinal, proof in enumerate(observed):
            reset = proof.get("g2_reset", {})
            self.assertEqual(reset.get("reset_write_calls", 0), 1)
            self.assertEqual(reset.get("reset_rows", 0), 10)
            self.assertEqual(reset["coverage"]["current_by_kind"], {"doc": documents, "note": 10})
            self.assertEqual(reset["coverage"]["historical_physical_vectors"], ordinal * 10)
            self.assertEqual(reset["coverage"]["total_physical_vectors"], documents + (ordinal + 1) * 10)
            self.assertEqual(reset["pending_after_drain"], 0)
            self.assertGreater(reset["reset_write_ns"], 0)
            self.assertGreater(reset["reset_drain_ns"], 0)
            placement = proof["g2_placement"]
            self.assertGreaterEqual(placement["reset_start_ns"], 0)
            self.assertLess(placement["reset_start_ns"], placement["reset_end_ns"])
            self.assertLessEqual(placement["reset_end_ns"], placement["first_timed_record_start_ns"])
            if warmups:
                self.assertEqual(placement["first_warmup_disposition"], "observed")
                self.assertLessEqual(placement["reset_end_ns"], placement["first_warmup_record_start_ns"])
                self.assertLess(placement["first_warmup_record_start_ns"], placement["first_timed_record_start_ns"])
            else:
                self.assertEqual(placement["first_warmup_disposition"], "not_applicable")
                self.assertIsNone(placement["first_warmup_record_start_ns"])
            self.assertEqual(proof["throughput_scope"], "warmup and timed API calls plus input identity/validation/serialization/checkpoints; reset/open/close separate")

    def test_literal_two_same_store_resets_before_positive_warmups(self):
        self.require_reset(2, 2)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(case=st.sampled_from([(3, 0), (65, 2)]))
    def test_bounded_documents_and_zero_or_positive_warmups_keep_current_history_separate(self, case):
        self.require_reset(*case)


if __name__ == "__main__":
    unittest.main()
