"""Actual harmless-child command protocol; no database/model behavior is mocked."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_campaign import make_g_plan
from g_driver import execute_stage


class GWriteDispatchTests(unittest.TestCase):
    def test_write_cell_prepares_fresh_seed_before_current_write_driver_on_same_new_store(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = next(row for row in make_g_plan(spec) if row["stage"] == "G-W")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            selected = root / "previous-subject.sqlite"
            selected.write_bytes(b"retained prior subject; never open/copy/reuse")
            calls = root / "calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "config=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps(config)+'\\n')",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            manifest = {"command": [sys.executable, "-c", code], "corpus": "frozen-corpus.jsonl",
                        "queries": "frozen-queries.jsonl", "selected_store": str(selected)}
            attempt = root / "attempt-0001"
            result = execute_stage(cell, manifest, attempt, time.monotonic_ns() + 900 * 10**9)
            actual = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual([row["family"] for row in actual], ["G-seed", "G-W"],
                             "G-W skipped fresh governed seed before its write driver")
            self.assertEqual(actual[0]["adapter"], cell["adapters"][0])
            self.assertEqual(actual[0]["n"], 100000)
            self.assertEqual(actual[1]["store"], str(attempt / "seed" / "store.sqlite"))
            self.assertNotEqual(actual[1]["store"], str(selected))
            for row in actual:
                self.assertEqual((row["provider"], row["negative_control"]), ("default", False))
                self.assertEqual((row["corpus"], row["queries"]),
                                 ("frozen-corpus.jsonl", "frozen-queries.jsonl"))
                self.assertGreater(row["remaining_cell_ms"], 0)
                self.assertLessEqual(row["remaining_cell_ms"], 900000)
            self.assertLess(actual[1]["remaining_cell_ms"], actual[0]["remaining_cell_ms"])
            self.assertEqual(selected.read_bytes(), b"retained prior subject; never open/copy/reuse")
            self.assertTrue(result["complete"])
            self.assertEqual(len(result["steps"]), 2)
            self.assertTrue(all(Path(step["config"]).is_file() and Path(step["output"]).is_dir()
                                for step in result["steps"]))


if __name__ == "__main__":
    unittest.main()

class GProgramOutputContractTests(unittest.TestCase):
    def test_normal_program_step_receives_nonexistent_output_and_owns_its_creation(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = next(row for row in make_g_plan(spec) if row["stage"] == "G-W")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "output-contract.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps({{'existed':out.exists()}})+'\\n')",
                "assert not out.exists(), 'normal G driver output must be fresh'",
                "out.mkdir(parents=True)",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            manifest = {"command": [sys.executable, "-c", code], "corpus": "frozen-corpus.jsonl",
                        "queries": "frozen-queries.jsonl", "selected_store": str(root / "prior.sqlite")}
            result = execute_stage(cell, manifest, root / "attempt", time.monotonic_ns() + 900 * 10**9)
            actual = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual(actual, [{"existed": False}, {"existed": False}],
                             "controller precreated the normal G driver output")
            self.assertTrue(result["complete"])
            self.assertTrue(all(Path(step["config"]).is_file() and Path(step["output"]).is_dir()
                                and Path(step["config"]).parent != Path(step["output"])
                                for step in result["steps"]))
