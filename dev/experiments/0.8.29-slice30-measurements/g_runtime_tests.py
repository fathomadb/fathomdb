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


    def test_shared_seed_driver_declares_reports_and_reopens_without_false_dense_readiness(self):
        binary = Path(os.environ["FATHOMDB_SLICE30_G_BINARY"]).resolve(strict=True)
        for adapter in ["engine", "sdk"]:
            with self.subTest(adapter=adapter), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                corpus, queries = root / "corpus.jsonl", root / "queries.jsonl"
                corpus.write_text('"alpha needle"\n"beta"\n')
                queries.write_text('"needle"\n')
                config = root / "cell.json"
                config.write_text(json.dumps({"family": "G-seed", "provider": "none",
                                              "negative_control": True, "adapter": adapter,
                                              "n": 2, "corpus": str(corpus),
                                              "queries": str(queries)}))
                child = subprocess.run([str(binary), str(config), str(root / "out")],
                                       text=True, capture_output=True, timeout=30, check=False)
                self.assertEqual(child.returncode, 0, child.stdout + child.stderr)
                result = json.loads((root / "out" / "result.json").read_text())
                self.assertTrue(result["projection_declared"],
                                "shared G seed driver did not declare the public doc vector projection")
                self.assertTrue(result["complete"])
                self.assertEqual(result["requested_provider"], "none")
                self.assertEqual(result["natural_records"], 2)
                self.assertEqual(result["lexical_probe"]["ids"], ["l:s30doc-0"])
                self.assertFalse(result["qualified_dense_fixture"])
                for phase in ["open_ns", "configure_ns", "write_ns", "drain_ns",
                              "close_ns", "reopen_ns"]:
                    self.assertGreater(result[phase], 0)
                for fixture in [result["before_close"], result["after_reopen"]]:
                    self.assertEqual(fixture["natural_records"], 2)
                    self.assertEqual(fixture["vector_rows"], 0)
                    self.assertEqual(fixture["span_rows"], 0)
                    self.assertEqual(fixture["physical_vector_rows"], 0)
                    self.assertEqual(fixture["format"], 37)
                    self.assertFalse(fixture["usable_embedder"])
                    self.assertFalse(fixture["qualified_dense_fixture"])


if __name__ == "__main__":
    unittest.main()
