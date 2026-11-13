"""Explicit normal-program G controls; never model execution or silent skips."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class GNormalCliTests(unittest.TestCase):
    def test_actual_engine_sdk_observer_and_nested_unwind_are_reported_by_normal_cli(self):
        binary = Path(os.environ["FATHOMDB_SLICE30_G_BINARY"]).resolve(strict=True)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, queries = root / "corpus.jsonl", root / "queries.jsonl"
            corpus.write_text('"alpha needle"\n"beta"\n')
            queries.write_text('"needle"\n')
            config = root / "cell.json"
            config.write_text(json.dumps({"family": "G-control", "provider": "none", "n": 2,
                                          "corpus": str(corpus), "queries": str(queries),
                                          "observe": True, "nested_control": True}))
            child = subprocess.run([str(binary), str(config), str(root / "out")],
                                   text=True, capture_output=True, timeout=30, check=False)
            self.assertEqual(child.returncode, 0, child.stdout + child.stderr)
            result = json.loads((root / "out" / "result.json").read_text())
            self.assertTrue(result["complete"])
            self.assertEqual(result["control"], "negative_no_provider")
            self.assertFalse(result["qualified_dense_fixture"])
            self.assertEqual([(row["adapter"], row["route"]) for row in result["samples"]],
                             [(adapter, route) for adapter in ["engine", "sdk"]
                              for route in ["text", "hybrid", "embed"]])
            self.assertEqual(len(result["observation_batches"]), 6)
            for sample, trace in zip(result["samples"], result["observation_batches"]):
                route = sample["route"]
                self.assertGreater(len(trace["records"]), 0,
                                   "normal G CLI did not collect actual engine/SDK observations")
                self.assertEqual(trace["operation_id"], sample["operation_id"])
                self.assertTrue(all(row["operation_id"] == sample["operation_id"]
                                    for row in trace["records"]))
                api = [row for row in trace["records"] if row["phase"] == f"api_{route}"]
                self.assertEqual(len(api), 1)
                self.assertEqual(api[0]["status"], "error" if route == "embed" else "ok")
                if route != "embed":
                    self.assertEqual(sample["ids"], ["l:s30doc-0"])
                    self.assertEqual(sum(row["phase"] == "observer_snapshot_capture"
                                         for row in trace["records"]), 1)
                else:
                    self.assertEqual(sample["error_kind"], "EmbedderNotConfiguredError")
                self.assertEqual(trace["dropped_records"], 0)
                self.assertEqual(trace["issues"], [])
            self.assertEqual(len(result["nested_controls"]), 2)
            for control in result["nested_controls"]:
                api = [row for row in control["records"] if row["phase"] == "api_text"]
                self.assertEqual([row["operation_id"] for row in api], [7001, 7002, 7001])
                self.assertTrue(all(row["status"] == "ok" for row in api))
                self.assertEqual(control["dropped_records"], 0)
                self.assertEqual(control["issues"], [])
            self.assertTrue(all(fixture["natural_records"] == 2
                                and fixture["format"] == 37
                                and fixture["qualified_dense_fixture"] is False
                                for fixture in result["fixtures"]))


if __name__ == "__main__":
    unittest.main()
