"""Literal public pair inventory/report oracles; never natural qualification."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from g_pair import plan
from g_campaign import make_g_plan
from campaign import make_plan
from g_freeze import FEATURES


class PairInventoryTests(unittest.TestCase):
    def test_exact_primary_diagnostics_calibration_and_original_inventory(self):
        actual = plan()
        self.assertEqual(len(actual["primary"]), 136)
        self.assertEqual(len({row["origin_id"] for row in actual["primary"]}), 68)
        self.assertEqual(len(actual["diagnostics"]), 24)
        self.assertEqual(len({row["origin_id"] for row in actual["diagnostics"]}), 12)
        self.assertEqual(len(actual["calibration"]), 4)
        self.assertEqual(len({row["id"] for rows in actual.values() for row in rows}), 164)
        by_family = {name: sum(row["family"] == name for row in actual["primary"]) for name in ["P1", "P2", "G-R", "G-W"]}
        self.assertEqual(by_family, {"P1":8, "P2":24, "G-R":8, "G-W":96})
        for rows in actual.values():
            for row in rows:
                self.assertEqual((row["warmup"],row["samples"]), (500,1000))
                self.assertNotIn("embed", row["routes"])
                self.assertEqual(row["observe"], row["artifact"] == "diagnostic")
        for origin in {row["origin_id"] for row in actual["primary"]}:
            pair = [row for row in actual["primary"] if row["origin_id"] == origin]
            self.assertEqual([row["slice50_pair_mode"] for row in pair], ["eager","winner"] if origin.endswith("r1") else ["winner","eager"])
        for row in actual["calibration"]:
            self.assertEqual(row["origin_id"], "g-r-n10000-r0")
            self.assertEqual(row["n"], 10000)
            self.assertEqual(row["adapters"], ["engine","sdk"])
            self.assertEqual(row["routes"], ["text","hybrid"])
        tools = Path(__file__).parent
        self.assertEqual(len(make_g_plan(json.loads((tools/'g-cells.json').read_text()))),84)
        self.assertEqual(len(make_plan(json.loads((tools/'cells.json').read_text()),2)),16)
        self.assertEqual(FEATURES,{"ordinary":["embed-cuda"],"diagnostic":["embed-cuda","slice30-observer"]})

    def test_public_report_keeps_every_unrun_obligation_and_no_false_credit(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run([sys.executable, str(Path(__file__).with_name('g_runner.py')), 'pair-report', temporary],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            data=json.loads(result.stdout)
            self.assertFalse(data['complete'])
            self.assertEqual(len(data['primary']),136)
            self.assertEqual(len(data['diagnostics']),24)
            self.assertEqual(len(data['calibration']),4)
            self.assertTrue(all(row['status']=='not_run' for key in ['primary','diagnostics','calibration'] for row in data[key]))


if __name__ == '__main__':
    unittest.main()
