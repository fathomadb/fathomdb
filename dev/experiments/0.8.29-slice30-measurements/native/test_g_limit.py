"""Manual private-G limit control: successful real APIs before requested-limit assertions."""

import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest

from g_template_fixture import run
from test_g_mean import GPU


class RetrievalLimitTests(unittest.TestCase):
    def test_core_and_sdk_obey_requested_ten_on_closed_65_vector_fixture(self):
        task = Path(os.environ["S30_GD_TASK"])
        root = Path(tempfile.mkdtemp(prefix="s30-g-limit-"))
        records = [{"id": f"literal-{index}", "kind": "doc",
                    "body": f"naturalevidence record {index} épreuve"} for index in range(65)]
        source = root / "records.jsonl"
        source.write_text("".join(json.dumps(record) + "\n" for record in records))
        config = root / "cell.json"
        config.write_text(json.dumps({"family": "G_LIMIT", "gpu_uuid": GPU,
                                     "templates": [{"records": str(source)}],
                                     "query": "naturalevidence", "limit": 10}))
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
        identity = {"oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "root": str(root)}
        (root / "oracle-identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        print(json.dumps(identity), flush=True)
        run(config, root, binary=binary, feature_path=task / "proofs/gd-g-host-feature-closure.json")
        preparation = json.loads((root / "out/case-0/template.json").read_text())["preparation"]
        if (preparation["api_complete"], preparation["write_rows"], preparation["write_calls"],
                preparation["pending"]) != (True, 65, 2, 0):
            raise RuntimeError("existing governed preparation/drain/close controls failed, NOT_RED")
        coverage = preparation["coverage"]
        if len(coverage["rows"]) != 65 or coverage["profile"]["dimension"] != 384:
            raise RuntimeError("existing physical/default-profile controls failed, NOT_RED")
        for mode in ["engine_hybrid", "sdk_hybrid"]:
            store = root / "out" / mode / "store.sqlite"
            with sqlite3.connect(store.as_uri() + "?mode=ro", uri=True) as connection:
                actual = connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='doc' AND state='active' AND superseded_at IS NULL ORDER BY write_cursor").fetchall()
                joined = connection.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor").fetchone()[0]
            if actual != [(row["id"], row["body"]) for row in records] or joined != 65:
                raise RuntimeError("independent closed-copy canonical/mapping/span controls failed, NOT_RED")
        rows = json.loads((root / "out/limit-proof.json").read_text())
        expected_ids = {"l:" + record["id"] for record in records}
        if [row["mode"] for row in rows] != ["engine_hybrid", "sdk_hybrid"]:
            raise RuntimeError("existing actual API controls missing, NOT_RED")
        for row in rows:
            if (row["query"], row["requested_limit"], row["selected_cuda_uuid"], row["cuda_compiled"],
                    row["soft_fallback"], row["degradations"]) != ("naturalevidence", 10, GPU, True, "None", "[]"):
                raise RuntimeError("actual API/CUDA/non-degraded controls failed, NOT_RED")
            if not row["ids"] or len(set(row["ids"])) != len(row["ids"]) or not set(row["ids"]) <= expected_ids or row["elapsed_ns"] <= 0:
                raise RuntimeError("actual result identities/timer controls failed, NOT_RED")
        print(json.dumps({"existing_controls": "passed", "actual": rows}), flush=True)
        for row in rows:
            with self.subTest(mode=row["mode"]):
                self.assertEqual(row["returned_hits"], 10, "private caller ignored requested retrieval limit")
                self.assertEqual(len(row["ids"]), 10)


if __name__ == "__main__":
    unittest.main()
