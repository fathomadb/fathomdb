"""Manual G2 retrieval eligibility on actual mixed doc/reset-note populations."""

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


class KindFilterTests(unittest.TestCase):
    def require_doc_retrieval(self, documents, warmups):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-kind-filter-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, documents)]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text(json.dumps({"id": 42, "query": "revisionreset"}) + "\n" +
                           json.dumps({"id": 7, "query": "alphaonly épreuve"}) + "\n")
        modes = ["sdk_text", "sdk_hybrid", "sdk_mean"]
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G_FEED_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}], "queries": str(queries),
                                     "modes": modes, "population": "G2", "schedule": "readonly", "warmups": warmups}))
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
        if (preparation["api_complete"], preparation["write_rows"], preparation["pending"]) != (True, documents, 0):
            raise RuntimeError("existing template preparation failed, NOT_RED")
        if hashlib.sha256((root / "out/case-0/template.sqlite").read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source changed, NOT_RED")
        docs = {"l:" + row["id"] for row in records}
        notes = {f"l:s30marker-{slot}" for slot in range(10)}
        observed = {}
        for mode in modes:
            directory = root / "out" / mode
            proof = json.loads((directory / "api-proof.json").read_text())
            issues = []
            warm = read_jsonl_records(directory / "warmups.jsonl", issues)
            timed = read_jsonl_records(directory / "samples.jsonl", issues)
            if issues or (len(warm), len(timed), proof["api_calls"], proof["warmup_api_calls"]) != (warmups, 2, warmups + 2, warmups):
                raise RuntimeError("existing actual call/framing controls failed, NOT_RED")
            if proof["selected_cuda_uuid"] != GPU or proof["cuda_compiled"] is not True:
                raise RuntimeError("required actual CUDA identity failed, NOT_RED")
            if (proof["g2_reset"]["reset_write_calls"], proof["g2_reset"]["reset_rows"], proof["g2_reset"]["pending_after_drain"]) != (1, 10, 0):
                raise RuntimeError("existing actual reset/drain controls failed, NOT_RED")
            with sqlite3.connect((directory / "store.sqlite").as_uri() + "?mode=ro", uri=True) as connection:
                counts = connection.execute("SELECT c.kind,count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.state='active' AND c.superseded_at IS NULL GROUP BY c.kind ORDER BY c.kind").fetchall()
                markers = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL ORDER BY logical_id").fetchall()
            if counts != [("doc", documents), ("note", 10)] or markers != [(f"s30marker-{slot}", f"s30markercurrent{slot} revisionreset") for slot in range(10)]:
                raise RuntimeError("independent actual mixed population controls failed, NOT_RED")
            for row in [*warm, *timed]:
                if row["ok"] is not True or row["elapsed_ns"] <= 0:
                    raise RuntimeError("existing actual API/timer controls failed, NOT_RED")
                if mode == "sdk_mean":
                    if (row["dimension"], row["finite"]) != (384, True) or row["norm_squared"] <= 0:
                        raise RuntimeError("existing actual mean value controls failed, NOT_RED")
                else:
                    if len(row["ids"]) != len(set(row["ids"])) or not set(row["ids"]) <= docs | notes or row["returned_hits"] != len(row["ids"]):
                        raise RuntimeError("existing actual search value controls failed, NOT_RED")
                    if mode == "sdk_hybrid" and (row["soft_fallback"], row["degradations"]) != ("None", "[]"):
                        raise RuntimeError("existing non-degraded hybrid controls failed, NOT_RED")
            if [row["query_id"] for row in timed] != [42, 7]:
                raise RuntimeError("existing actual query forwarding failed, NOT_RED")
            observed[mode] = (warm, timed)
        print(json.dumps({"root": str(root), "actual_first_query_ids": {mode: rows[1][0].get("ids") for mode, rows in observed.items()}}), flush=True)
        for mode in ["sdk_text", "sdk_hybrid"]:
            with self.subTest(mode=mode):
                warm, timed = observed[mode]
                self.assertTrue(all(set(row["ids"]) <= docs for row in [*warm, *timed]),
                                "missing generic G2 doc-kind restriction: real note IDs returned")
                self.assertEqual({row["kind_filter"] for row in [*warm, *timed]}, {"doc"})
                if mode == "sdk_text":
                    self.assertEqual([row["ids"] for row in timed], [[], ["l:alpha"]])
                else:
                    self.assertEqual([set(row["ids"]) for row in timed], [docs, docs])
        for row in [*observed["sdk_mean"][0], *observed["sdk_mean"][1]]:
            self.assertNotIn("kind_filter", row)
            self.assertNotIn("ids", row)

    def test_literal_mixed_population_retrieval_excludes_notes_mean_unfiltered(self):
        self.require_doc_retrieval(2, 2)

    @settings(max_examples=2, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(case=st.sampled_from([(3, 0), (8, 1)]))
    def test_bounded_document_counts_and_warmups_keep_retrieval_eligibility(self, case):
        self.require_doc_retrieval(*case)


if __name__ == "__main__":
    unittest.main()
