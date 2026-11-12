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


def storage_cli(peak_bytes, retained_bytes):
    payload = {"lower_n":100_000,"target_n":300_000,"elapsed_ns":1,
               "peak_bytes":peak_bytes,"retained_family_bytes":retained_bytes}
    child = subprocess.run([sys.executable,str(TOOL)],input=json.dumps(payload),
                           capture_output=True,text=True,timeout=5)
    if child.returncode != 0:
        raise AssertionError(child.stderr)
    return json.loads(child.stdout).get("storage", {})


class StorageProjectionTests(unittest.TestCase):
    def test_18gib_additional_and_existing_1gib_declines_fixed_16gib_family(self):
        gib = 1024**3
        result = storage_cli(4*gib,gib)
        print(json.dumps(result),flush=True)
        self.assertEqual(result.get("fits"),False)
        self.assertEqual(result["additional_bytes"],18*gib)
        self.assertEqual(result["family_bytes"],19*gib)
        self.assertEqual(result["cap_bytes"],16*gib)

    def test_4point5gib_additional_and_existing_10gib_fits_16gib_family(self):
        gib = 1024**3
        result = storage_cli(gib,10*gib)
        self.assertEqual(result.get("fits"),True)
        self.assertEqual(result["additional_bytes"],9*gib//2)
        self.assertEqual(result["family_bytes"],29*gib//2)

    def test_fractional_additional_byte_rounds_up(self):
        result = storage_cli(1,0)
        self.assertEqual(result.get("additional_bytes"),5)

    @settings(max_examples=8,deadline=None)
    @given(retained=st.integers(min_value=0,max_value=20*1024**3),
           increase=st.integers(min_value=0,max_value=2*1024**3))
    def test_more_retained_evidence_cannot_improve_storage_eligibility(self, retained, increase):
        before = storage_cli(1024**3,retained)
        after = storage_cli(1024**3,retained+increase)
        self.assertIsInstance(before.get("fits"),bool,"missing storage-feasibility projection")
        self.assertIsInstance(after.get("fits"),bool)
        self.assertGreaterEqual(after["family_bytes"],before["family_bytes"])
        self.assertFalse(before["fits"] is False and after["fits"] is True)


def clock_cli(consumed_ns):
    payload = {"lower_n":100_000,"target_n":300_000,"elapsed_ns":10_000_000_000,
               "shared_elapsed_ns":consumed_ns}
    child = subprocess.run([sys.executable,str(TOOL)],input=json.dumps(payload),
                           capture_output=True,text=True,timeout=5)
    if child.returncode != 0:
        raise AssertionError(child.stderr)
    return json.loads(child.stdout).get("shared", {})


class SharedClockTests(unittest.TestCase):
    def test_45_second_forecast_plus_30_cleanup_cannot_fit_last_50_seconds(self):
        result = clock_cli(14_350_000_000_000)
        print(json.dumps(result),flush=True)
        self.assertEqual(result.get("fits"),False)
        self.assertEqual(result["required_ns"],75_000_000_000)
        self.assertEqual(result["remaining_ns"],50_000_000_000)
        self.assertEqual(result["cap_ns"],14_400_000_000_000)

    def test_prior_prep_waits_are_kept_and_75_seconds_fits_remaining_400(self):
        result = clock_cli(14_000_000_000_000)
        self.assertEqual(result.get("fits"),True)
        self.assertEqual(result["remaining_ns"],400_000_000_000)
        self.assertEqual(result["required_ns"],75_000_000_000)

    def test_spent_shared_clock_has_zero_remaining_and_declines(self):
        result = clock_cli(14_401_000_000_000)
        self.assertEqual(result.get("fits"),False)
        self.assertEqual(result["remaining_ns"],0)

    @settings(max_examples=8,deadline=None)
    @given(consumed=st.integers(min_value=0,max_value=14_500_000_000_000),
           increase=st.integers(min_value=0,max_value=100_000_000_000))
    def test_more_consumed_clock_cannot_improve_eligibility(self, consumed, increase):
        before = clock_cli(consumed)
        after = clock_cli(consumed+increase)
        self.assertIsInstance(before.get("fits"),bool,"missing shared-clock feasibility")
        self.assertIsInstance(after.get("fits"),bool)
        self.assertLessEqual(after["remaining_ns"],before["remaining_ns"])
        self.assertFalse(before["fits"] is False and after["fits"] is True)



def admission_cli(**changes):
    payload = {"lower_n":100_000,"target_n":300_000,"elapsed_ns":10_000_000_000,
               "peak_bytes":1024**3,"retained_family_bytes":0,
               "shared_elapsed_ns":14_000_000_000_000,"lower_status":"complete"}
    for key,value in changes.items():
        if value is None:
            payload.pop(key,None)
        else:
            payload[key]=value
    child = subprocess.run([sys.executable,str(TOOL)],input=json.dumps(payload),
                           capture_output=True,text=True,timeout=5)
    if child.returncode != 0:
        raise AssertionError(child.stderr)
    return json.loads(child.stdout).get("admission", {})


class AdmissionReportTests(unittest.TestCase):
    def test_complete_lower_with_all_three_projections_fitting_is_only_a_projection(self):
        result = admission_cli()
        print(json.dumps(result),flush=True)
        self.assertEqual(result.get("eligible"),True)
        self.assertEqual(result["disposition"],"ELIGIBLE_PROJECTION")

    def test_any_resource_dimension_refuses_even_with_complete_lower(self):
        for changes in ({"elapsed_ns":210_000_000_000},
                        {"peak_bytes":4*1024**3},
                        {"shared_elapsed_ns":14_350_000_000_000}):
            with self.subTest(changes=changes):
                result = admission_cli(**changes)
                self.assertEqual(result.get("eligible"),False)
                self.assertEqual(result["disposition"],"NOT_RUN_RESOURCE")

    def test_failed_or_missing_lower_completion_refuses_despite_tiny_forecasts(self):
        for status in ("failed",None):
            with self.subTest(status=status):
                result = admission_cli(lower_status=status)
                self.assertEqual(result.get("eligible"),False)
                self.assertEqual(result["disposition"],"NOT_RUN_RESOURCE")

    @settings(max_examples=8,deadline=None)
    @given(missing=st.sets(st.sampled_from(["peak_bytes","retained_family_bytes",
                                          "shared_elapsed_ns"]),min_size=1))
    def test_incomplete_resource_evidence_never_admits_a_target(self, missing):
        result = admission_cli(**dict.fromkeys(missing))
        self.assertEqual(result.get("eligible"),False)
        self.assertEqual(result["disposition"],"NOT_RUN_RESOURCE")


if __name__ == "__main__":
    unittest.main()
