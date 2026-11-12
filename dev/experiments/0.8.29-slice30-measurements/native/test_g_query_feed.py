"""Manual real six-mode query forwarding, literal text IDs and actual-input identity."""

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

MODES = ["engine_hybrid", "engine_text", "engine_mean", "sdk_hybrid", "sdk_text", "sdk_mean"]
QUESTIONS = [(41, "alphaonly épreuve"), (7, "betaonly βήτα"), (108, "s30nonematchingquery")]
TEXT_IDS = {QUESTIONS[0][1]: ["l:alpha"], QUESTIONS[1][1]: ["l:beta"], QUESTIONS[2][1]: []}


class QueryFeedTests(unittest.TestCase):
    def fixture(self, questions):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-query-feed-"))
        records = [{"id": "alpha", "kind": "doc", "body": "alphaonly épreuve"},
                   {"id": "beta", "kind": "doc", "body": "betaonly βήτα"}]
        records += [{"id": f"doc-{index}", "kind": "doc", "body": f"commonword natural record {index}"}
                    for index in range(2, 65)]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        queries = root / "queries.jsonl"
        queries.write_text("".join(json.dumps({"id": identity, "query": query}) + "\n"
                                   for identity, query in questions))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G_FEED_FIXTURE", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}],
                                     "queries": str(queries), "modes": MODES}))
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
            raise RuntimeError("existing real governed preparation/drain/close failed, NOT_RED")
        if len(preparation["coverage"]["rows"]) != 65 or preparation["coverage"]["profile"]["dimension"] != 384:
            raise RuntimeError("existing physical/default-profile controls failed, NOT_RED")
        prepared = root / "out/case-0/template.sqlite"
        if hashlib.sha256(prepared.read_bytes()).hexdigest() != preparation["closed_main_sha256"]:
            raise RuntimeError("closed source template changed during read-only calls, NOT_RED")
        fingerprints = {hashlib.sha256(query.encode()).hexdigest(): query for _, query in questions}
        allowed_ids = {"l:" + row["id"] for row in records}
        observations = {}
        for mode in MODES:
            directory = root / "out" / mode
            proof = json.loads((directory / "api-proof.json").read_text())
            issues = []
            rows = read_jsonl_records(directory / "samples.jsonl", issues)
            if proof["api_calls"] != len(questions) or issues or len(rows) != len(questions):
                raise RuntimeError("existing successful API/record framing controls failed, NOT_RED")
            if proof["selected_cuda_uuid"] != GPU or proof["cuda_compiled"] is not True:
                raise RuntimeError("required actual Default CUDA identity failed, NOT_RED")
            adapter = "engine" if mode.startswith("engine_") else "sdk"
            population = "hybrid" if mode.endswith("_hybrid") else "direct_text" if mode.endswith("_text") else "mean_embed"
            for index, row in enumerate(rows):
                if (row["adapter"], row["mode"], row["ok"], row["sample_index"]) != (adapter, population, True, index) or row["elapsed_ns"] <= 0:
                    raise RuntimeError("existing adapter/API/timer controls failed, NOT_RED")
                if row["query_fingerprint"] not in fingerprints:
                    raise RuntimeError("actual query is outside the fixed source fixture, NOT_RED")
                if population == "mean_embed":
                    if (row["dimension"], row["finite"]) != (384, True) or row["norm_squared"] <= 0:
                        raise RuntimeError("existing real mean API value controls failed, NOT_RED")
                else:
                    if len(row["ids"]) != len(set(row["ids"])) or not set(row["ids"]) <= allowed_ids or row["returned_hits"] != len(row["ids"]):
                        raise RuntimeError("existing real search identities/counts failed, NOT_RED")
                    if population == "hybrid":
                        if (row["returned_hits"], row["soft_fallback"], row["degradations"]) != (10, "None", "[]"):
                            raise RuntimeError("existing non-degraded hybrid API failed, NOT_RED")
                    elif row["ids"] != TEXT_IDS[fingerprints[row["query_fingerprint"]]]:
                        raise RuntimeError("literal text IDs do not match the actual called query, NOT_RED")
            with sqlite3.connect((directory / "store.sqlite").as_uri() + "?mode=ro", uri=True) as connection:
                actual = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='doc' AND state='active' AND superseded_at IS NULL ORDER BY write_cursor").fetchall()
                joined = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor").fetchone()[0]
            if actual != [(row["id"], row["body"]) for row in records] or joined != 65:
                raise RuntimeError("independent per-copy canonical/cursor/span controls failed, NOT_RED")
            observations[mode] = rows
        print(json.dumps({"root": str(root), "existing_controls": "all six modes passed",
                          "actual_id_sequences": {mode: [row["query_id"] for row in rows]
                                                  for mode, rows in observations.items()}}), flush=True)
        return observations

    def require_forwarding(self, observations, questions, mode):
        rows = observations[mode]
        expected = [(identity, hashlib.sha256(query.encode()).hexdigest()) for identity, query in questions]
        self.assertEqual([(row["query_id"], row["query_fingerprint"]) for row in rows], expected,
                         "missing actual query-file forwarding/order")
        if mode.endswith("_text"):
            self.assertEqual([row["ids"] for row in rows], [TEXT_IDS[query] for _, query in questions])
        self.assertEqual(len(rows), len(questions))

    def test_literal_actual_query_identity_and_text_results_across_six_modes(self):
        observations = self.fixture(QUESTIONS)
        for mode in MODES:
            with self.subTest(mode=mode):
                self.require_forwarding(observations, QUESTIONS, mode)

    @settings(max_examples=3, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(questions=st.permutations(QUESTIONS))
    def test_bounded_arbitrary_id_order_survives_real_api_forwarding(self, questions):
        observations = self.fixture(questions)
        for mode in MODES:
            self.require_forwarding(observations, questions, mode)


if __name__ == "__main__":
    unittest.main()
