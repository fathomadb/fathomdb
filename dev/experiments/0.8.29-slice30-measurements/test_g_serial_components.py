"""Real file lengths and unavailable closure observations, not native measurements."""

import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

from g_campaign import run_g_plan
from g_serial_storage import capture_closure, derive_model
from test_g_serial_controller import child, prepared
from test_g_serial_storage import model


class GSerialComponentTests(unittest.TestCase):
    def test_six_case_derivation_excludes_store_but_counts_every_serialization(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            records = []
            references = {}
            for index, basis in enumerate(model()["basis"]["records"]):
                attempt = root/basis["id"]
                kind = "read" if index < 2 else "write"
                output = attempt/kind
                output.mkdir(parents=True)
                baseline = 0 if index == 0 else 8192
                actual = {"preparation": {"copied_bytes": baseline}}
                (attempt/"result.json").write_bytes(json.dumps(actual, separators=(",", ":")).encode().ljust(40))
                (output/"result.json").write_bytes(b"{}".ljust(11))
                (attempt/(kind+"-native.log")).write_bytes(b"{}".ljust(7))
                count = basis["record_count"]
                (output/"samples.jsonl").write_bytes(b'{"x":1}\n'*count)
                (attempt/"control").write_bytes(b"x"*31)
                if baseline:
                    (attempt/"subject").mkdir()
                    db = sqlite3.connect(attempt/"subject/store.sqlite")
                    db.execute("create table t(x)")
                    db.commit()
                    db.close()
                cell = {"samples": 1000, "batch_size": 10 if index in (2, 3) else 1,
                        "searches_per_batch": 1 if index in (2, 3) else 100}
                row = {"id": basis["id"], "attempt": str(attempt), "cell": cell,
                       "origin": {"root": basis["id"]}}
                if index < 2:
                    references[basis["id"]] = row
                else:
                    row.update(actual=actual, peak_store_bytes=baseline+(10000 if index in (2, 3) else 30))
                    records.append(row)
            cal = {"records": records, "origin": {"root": "six-real-file-cases"}}
            actual = derive_model({"compiler_inputs": "protocol-inputs"}, cal, references)
            self.assertEqual(actual["baseline_bytes"], 8192)
            self.assertEqual(actual["raw_bytes_per_record"], 9)
            self.assertEqual(actual["residual_bytes"], 31)
            self.assertEqual(actual["closed_growth_per_row"], 0)
            self.assertEqual(actual["live_growth_per_row"], 3)
            for observed in actual["basis"]["records"]:
                self.assertEqual(observed["serialized_bytes"], 8*observed["record_count"]+58)

    def test_unresolved_writer_cannot_supply_closed_credit(self):
        with self.assertRaises(ValueError):
            capture_closure({"cleanup_uncertain": False, "live_owned_pids": [123], "child_exit": 0})
        with self.assertRaises(ValueError):
            capture_closure({"cleanup_uncertain": True, "live_owned_pids": [], "child_exit": 0})

    def test_unavailable_final_tuple_holds_the_next_child_without_requalifying_native_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            with patch("g_serial_storage.capture_closure", side_effect=OSError("final bytes unavailable")):
                rows = run_g_plan(spec, root/"results", child(root), allocation_root=root, reused=reused)
            self.assertEqual((root/"calls").read_text().splitlines(), ["g-w-b1-s1-engine-text-r0"])
            first = rows[12]
            self.assertEqual(first["error"], "serial_closure_unavailable")
            self.assertTrue(json.loads(Path(first["attempt"], "result.json").read_text())["complete"])
            budget = json.loads((root/"g-budget.json").read_text())
            self.assertEqual(budget["serial_storage_blocked"], "final bytes unavailable")
            self.assertIsNone(budget["inflight"])
