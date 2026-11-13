"""Actual six-mode closed-template results through the owned G controller."""

import json
from pathlib import Path
import sys
import unittest

from hypothesis import Phase, given, settings, strategies as st
import test_g_campaign as source

sys.path.insert(0,str(Path(__file__).parents[1]))
from gd_results import summarize  # noqa: E402


class GResultTests(source._NativeGFixture):
    check_copy=source.CopiedGCellTests.check_copy
    check_owned_preparation=source.OwnedGTTests.check_owned_preparation

    def check_report(self,suffix):
        manifest,root,outcome=self.check_copy(suffix)
        report=summarize(manifest,root)
        groups=[row for row in report["cells"] if row.get("cell_id")=="g1-source"]
        self.assertEqual(len(groups),6,"missing actual G pass mode/adapter result groups")
        template_hash=json.loads((Path(outcome["attempt"])/"copy.json").read_text())["source_main_sha256"]
        for index,(group,mode) in enumerate(zip(groups,source.MODES,strict=True)):
            directory=Path(outcome["attempt"])/f"pass-{index}"
            raw=[json.loads(line) for line in (directory/"samples.jsonl").read_text().splitlines()]
            proof=json.loads((directory/"api-proof.json").read_text())
            self.assertEqual((group["mode"],group["adapter"]),(mode.split("_",1)[1],mode.split("_",1)[0]))
            self.assertEqual(group["dataset_sha256"],template_hash)
            self.assertEqual(group["status"],"complete")
            self.assertEqual(group["queries"],[{"query_id":row["query_id"],"sample_index":row["sample_index"],
                "query_fingerprint":row["query_fingerprint"],"elapsed_ns":row["elapsed_ns"],"ok":True,"error":None} for row in raw])
            self.assertEqual(group["statistics"]["timed"]["requests"],3)
            self.assertEqual(group["statistics"]["warmup"]["requests"],2)
            self.assertEqual(group["statistics"]["throughput_all_per_s"],proof["api_calls"]*1e9/proof["loop_ns"])
            self.assertEqual(group["statistics"]["wall_ns"],proof["loop_ns"])
            self.assertEqual(group["statistics"]["throughput_scope"],proof["throughput_scope"])
            self.assertEqual(group["observer"]["reason"],"feature_absent")
            self.assertIsNone(group["observer"]["query_records"])
            self.assertEqual(group["issues"],[])

    def test_actual_six_modes_preserve_raw_latency_and_pass_rate_scope(self):
        self.check_report("result é")

    @settings(max_examples=2,deadline=None,database=None,derandomize=True,phases=[Phase.generate])
    @given(st.sampled_from(["β","中"]))
    def test_closed_source_unicode_population_keeps_each_mode_group(self,suffix):
        self.check_report(suffix)


if __name__=="__main__":
    unittest.main()
