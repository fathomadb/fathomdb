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
                "out.mkdir(parents=True, exist_ok=True)",
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

class GReadDispatchTests(unittest.TestCase):
    def test_seed_read_and_natural_diagnostic_cells_bind_current_drivers_and_exact_subjects(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        plan = {row["id"]: row for row in make_g_plan(spec)}
        cases = [("g1-n10000", ["G-seed"]), ("g1-n100000", ["G-seed"]),
                 ("g-r-n10000-r0", ["G-R"]), ("g-r-n100000-r1", ["G-seed", "G-R"]),
                 ("g-o-natural-r0-on", ["G-O"])]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            queries = root / "all500.jsonl"
            questions = [f"question-{index}" for index in range(500)]
            queries.write_text("".join(json.dumps(value) + "\n" for value in questions))
            for identifier, families in cases:
                with self.subTest(cell=identifier):
                    cell = plan[identifier]
                    attempt = root / identifier
                    calls = root / f"{identifier}-calls.jsonl"
                    code = "\n".join([
                        "import json,pathlib,sys",
                        "config=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                        f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps(config)+'\\n')",
                        "assert not out.exists(); out.mkdir(parents=True)",
                        "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
                    ])
                    subjects = {f"g1-n{size}": str(root / f"qualified-n{size}" / "store.sqlite")
                                for size in [10000, 100000]}
                    manifest = {"command": [sys.executable, "-c", code],
                                "corpus": "frozen-corpus.jsonl", "queries": str(queries),
                                "selected_store": "wrong-global-subject.sqlite", "subjects": subjects}
                    result = execute_stage(cell, manifest, attempt, time.monotonic_ns() + 900 * 10**9)
                    actual = [json.loads(line) for line in calls.read_text().splitlines()]
                    self.assertEqual([row["family"] for row in actual], families,
                                     "G stage did not bind its required current normal-program drivers")
                    self.assertTrue(result["complete"])
                    if cell["kind"] == "seed":
                        self.assertEqual(actual[0]["adapter"], "sdk")
                    else:
                        read = actual[-1]
                        expected = (str(attempt / "seed" / "store.sqlite") if cell["fresh_seed"]
                                    else subjects[cell["seed_subject"]])
                        self.assertEqual(read["store"], expected)
                        self.assertEqual((read["adapters"], read["routes"]),
                                         (cell["adapters"], cell["routes"]))
                        if cell["kind"] == "natural":
                            self.assertEqual((read["samples"], read["warmup"], read["record_first_call"]),
                                             (50, 0, False))
                            selected = [json.loads(line) for line in Path(read["queries"]).read_text().splitlines()]
                            self.assertEqual(selected, questions[:50])
                        else:
                            self.assertEqual(read["queries"], str(queries))
                    self.assertEqual([json.loads(line) for line in queries.read_text().splitlines()], questions)

class GSmokeDispatchTests(unittest.TestCase):
    def test_tiny_smoke_binds_known_fixture_to_seed_and_all_current_routes_without_natural_input_change(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = make_g_plan(spec)[0]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, queries = root / "natural-corpus.jsonl", root / "natural-queries.jsonl"
            corpus.write_text('"retained natural body"\n')
            queries.write_text('"retained natural question"\n')
            calls = root / "calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "config=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps(config)+'\\n')",
                "assert not out.exists(); out.mkdir(parents=True)",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            result = execute_stage(cell, {"command": [sys.executable, "-c", code],
                                          "corpus": str(corpus), "queries": str(queries)},
                                   root / "attempt", time.monotonic_ns() + 900 * 10**9)
            actual = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual([row["family"] for row in actual], ["G-seed", "G-R"],
                             "tiny G0 did not invoke the existing seed/read drivers")
            self.assertEqual([json.loads(line) for line in Path(actual[0]["corpus"]).read_text().splitlines()],
                             ["alpha needle", "beta"])
            self.assertEqual([json.loads(line) for line in Path(actual[1]["queries"]).read_text().splitlines()],
                             ["needle"])
            self.assertEqual(actual[1]["corpus"], actual[0]["corpus"])
            self.assertEqual(actual[1]["store"], str(root / "attempt" / "seed" / "store.sqlite"))
            self.assertEqual((actual[1]["adapters"], actual[1]["routes"]),
                             (["engine", "sdk"], ["text", "hybrid", "embed"]))
            self.assertEqual((actual[1]["samples"], actual[1]["warmup"], actual[1]["record_first_call"]),
                             (16, 0, False))
            self.assertEqual(corpus.read_text(), '"retained natural body"\n')
            self.assertEqual(queries.read_text(), '"retained natural question"\n')
            self.assertTrue(result["complete"])

class GBranchDispatchTests(unittest.TestCase):
    def test_six_tiny_diagnostics_use_separate_source_and_commit_subjects_under_one_cell(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        cell = next(row for row in make_g_plan(spec) if row["kind"] == "branches")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "config=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps(config)+'\\n')",
                "assert not out.exists(); out.mkdir(parents=True)",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True,'boundary_epoch_s':2000000000}))",
            ])
            result = execute_stage(cell, {"command": [sys.executable, "-c", code],
                                          "corpus": "frozen-natural-corpus.jsonl", "queries": "frozen-natural-queries.jsonl"},
                                   root / "attempt", time.monotonic_ns() + 900 * 10**9)
            actual = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual([row["family"] for row in actual],
                             ["G-seed"] + ["G-O"] * 4 + ["G-source-fixture", "G-O"] + ["G-seed", "G-W"] * 4,
                             "G-O did not retain every required tiny branch command")
            self.assertEqual([row["diagnostic_case"] for row in actual],
                             ["base_seed", "direct_text", "hybrid_no_dependencies", "hybrid_kind_filter", "no_match",
                              "source_window", "source_window"] + ["commit_change"] * 8)
            self.assertEqual([json.loads(line) for line in Path(actual[0]["corpus"]).read_text().splitlines()],
                             [f"needle body{index}" for index in range(10)])
            self.assertEqual(actual[1]["routes"], ["text"])
            self.assertEqual(actual[2]["routes"], ["hybrid"])
            self.assertEqual((actual[3]["routes"], actual[3]["diagnostic_filter_kind"]), (["hybrid"], "note"))
            self.assertEqual(actual[4]["routes"], ["text", "hybrid"])
            self.assertEqual([json.loads(line) for line in Path(actual[4]["queries"]).read_text().splitlines()],
                             ["gdeliberatelyabsenttoken"])
            source = actual[6]
            self.assertEqual((source["routes"], source["samples"], source["diagnostic_valid_as_of_sequence"]),
                             (["text", "hybrid"], 2, [1999999999, 2000000000]))
            self.assertEqual(source["store"], str(Path(result["steps"][5]["output"]) / "store.sqlite"))
            writes = [row for row in actual if row["family"] == "G-W"]
            self.assertEqual([(row["adapters"], row["routes"]) for row in writes],
                             [([adapter], [route]) for adapter in ["engine", "sdk"] for route in ["text", "hybrid"]])
            self.assertEqual(len({row["store"] for row in writes}), 4)
            self.assertTrue(all(row["store"] != actual[1]["store"] and row["warmup"] == 1
                                and row["samples"] == 1 and row["batch_size"] == 1
                                and row["searches_per_batch"] == 1 and row["record_first_call"] is False
                                for row in writes))
            self.assertTrue(all(row["observe"] is True and row["provider"] == "default"
                                and row["negative_control"] is False for row in actual))
            self.assertTrue(result["complete"])
            self.assertEqual(len(result["steps"]), 15)
