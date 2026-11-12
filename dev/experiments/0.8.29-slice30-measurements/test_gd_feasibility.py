"""Hand-computed lower-scale time bounds and monotonicity, no native workload."""

import json
from pathlib import Path
import subprocess
import sys
import unittest

from hypothesis import given, settings, strategies as st


TOOL = Path(__file__).with_name("gd_feasibility.py")


def cli(lower_n, target_n, elapsed_ns):
    payload = {"lower_n":lower_n,"target_n":target_n,"elapsed_ns":elapsed_ns}
    child = subprocess.run([sys.executable,str(TOOL)],input=json.dumps(payload),
                           capture_output=True,text=True,timeout=5)
    if child.returncode != 0:
        raise AssertionError(child.stderr)
    return json.loads(child.stdout)


class TimeProjectionTests(unittest.TestCase):
    def test_300k_declines_945_second_bound_from_complete_100k_210_seconds(self):
        result = cli(100_000,300_000,210_000_000_000)
        print(json.dumps(result),flush=True)
        self.assertEqual(result["time"]["fits"],False)
        self.assertEqual(result["time"]["projected_ns"],945_000_000_000)
        self.assertEqual(result["time"]["cap_ns"],870_000_000_000)

    def test_100k_time_dimension_fits_150_second_bound_from_10k_10_seconds(self):
        result = cli(10_000,100_000,10_000_000_000)
        print(json.dumps(result),flush=True)
        self.assertEqual(result["time"]["fits"],True)
        self.assertEqual(result["time"]["projected_ns"],150_000_000_000)

    def test_subnanosecond_fraction_rounds_up_conservatively(self):
        result = cli(100_000,300_000,1)
        self.assertEqual(result["time"]["projected_ns"],5)

    @settings(max_examples=8,deadline=None)
    @given(first=st.integers(min_value=1,max_value=300_000_000_000),
           increase=st.integers(min_value=0,max_value=100_000_000_000))
    def test_more_observed_time_cannot_improve_eligibility(self, first, increase):
        before = cli(100_000,300_000,first)["time"]
        after = cli(100_000,300_000,first+increase)["time"]
        self.assertIsInstance(before["fits"],bool,"missing time-feasibility projection")
        self.assertIsInstance(after["fits"],bool)
        self.assertGreaterEqual(after["projected_ns"],before["projected_ns"])
        self.assertFalse(before["fits"] is False and after["fits"] is True)


if __name__ == "__main__":
    unittest.main()
