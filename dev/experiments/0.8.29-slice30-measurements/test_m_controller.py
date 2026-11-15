"""Actual harmless children observe the accepted M table; no natural input."""

from collections import Counter
import json
from pathlib import Path
import sys
import tempfile
import unittest

from m_controller import run_table


RECORD_CELL = """
import json, pathlib, sys
cell = json.loads(pathlib.Path(sys.argv[1]).read_text())
out = pathlib.Path(sys.argv[2])
(out/'observed.json').write_text(json.dumps({'cell': cell, 'attempt': str(out)}))
(out/'result.json').write_text(json.dumps({'complete': True, 'scope': 'harmless-source-control'}))
"""


class InventoryDispatchTests(unittest.TestCase):
    """Counts and balanced order come from actual dispatched child configurations."""

    def test_all_46_cells_keep_exact_populations_and_order(self):
        with tempfile.TemporaryDirectory() as temporary:
            results = run_table(Path(temporary), [sys.executable, "-c", RECORD_CELL])
            self.assertEqual(len(results), 46, "current selector omitted required M cells")
            observed = [json.loads((Path(row["attempt"]) / "observed.json").read_text())
                        for row in results]
            cells = [row["cell"] for row in observed]
            self.assertEqual(Counter(cell["family"] for cell in cells),
                             {"M0": 2, "M-R": 20, "M-C": 4, "M-L": 8, "M-E": 12})
            self.assertEqual(len({row["attempt"] for row in observed}), 46)
            self.assertTrue(all(row["status"] == "complete" for row in results))
            natural = [cell for cell in cells if cell["family"] in ("M-R", "M-C")]
            self.assertEqual(sum(cell["first"] + cell["warmups"] + cell["timed"]
                                 for cell in natural), 36024)
            self.assertEqual(sum(cell["timed"] for cell in natural), 24000)
            self.assertEqual(Counter(cell["search_mode"] for cell in natural),
                             {"hybrid": 22, "text": 2})
            repeat1 = [cell for cell in cells if cell["family"] == "M-R" and cell["repeat"] == 1]
            repeat2 = [cell for cell in cells if cell["family"] == "M-R" and cell["repeat"] == 2]
            self.assertEqual([(cell["concurrency"], cell["log"]) for cell in repeat1],
                             [(1,"warn"),(1,"debug"),(4,"warn"),(4,"debug"),(8,"warn"),
                              (8,"debug"),(16,"warn"),(16,"debug"),(32,"warn"),(32,"debug")])
            self.assertEqual([(cell["concurrency"], cell["log"]) for cell in repeat2],
                             [(32,"debug"),(32,"warn"),(16,"debug"),(16,"warn"),(8,"debug"),
                              (8,"warn"),(4,"debug"),(4,"warn"),(1,"debug"),(1,"warn")])
            controls = [cell for cell in cells if cell["family"] == "M-C"]
            self.assertEqual([(cell["repeat"], cell["search_mode"], cell["result_format"])
                              for cell in controls],
                             [(1,"text","text"),(1,"hybrid","structured"),
                              (2,"hybrid","structured"),(2,"text","text")])
            self.assertEqual(sum(cell["copies"] for cell in cells), 34)
            for cell in cells:
                self.assertEqual(cell["config"]["admission_cap"], 4)
                self.assertEqual(cell["config"]["queue_limit"], 32)
                self.assertEqual(cell["config"]["call_deadline_ms"], 10000)
                self.assertEqual(cell["config"]["run_limit_ms"], 30000)
                if cell["family"] in ("M0", "M-E"):
                    self.assertFalse(cell["config"]["use_default_embedder"])


if __name__ == "__main__":
    unittest.main()
