"""Actual six-mode closed-template results through the owned G controller."""

import fcntl
import hashlib
import json
import os
import time
from pathlib import Path
import sys
import unittest

from hypothesis import Phase, given, settings, strategies as st
import test_g_campaign as source

sys.path.insert(0,str(Path(__file__).parents[1]))
from gd_results import observer_summary, summarize  # noqa: E402


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


class GObserverResultTests(source._NativeGFixture):
    check_copy=source.CopiedGCellTests.check_copy
    check_owned_preparation=source.OwnedGTTests.check_owned_preparation

    def test_actual_same_template_normal_off_on_keeps_transport_and_raw_pairs(self):
        manifest,root,_=self.check_copy("observer report é")
        artifacts=Path(os.environ["S30_GO_ARTIFACTS"])
        binary=artifacts/"go"
        closure=json.loads((artifacts/"go-features.json").read_text())
        digest=hashlib.sha256(binary.read_bytes()).hexdigest()
        self.assertEqual(closure["binary_sha256"],digest,"actual GO artifact setup NOT_RED")
        self.assertFalse(closure["normal_gn"])
        self.assertFalse(any(feature=="test-hooks" or feature.startswith("tc5-benchmark") for row in closure["artifacts"] for feature in row["features"]))
        self.assertTrue(any(row["target"]["name"]=="fathomdb_engine" and "slice30-measurement" in row["features"] for row in closure["artifacts"]))
        print(f"actual GO result artifact {binary} sha256={digest}",flush=True)
        manifest["artifacts"]["go"]={"path":str(binary),"sha256":digest}
        base=manifest["cells"][-1]
        core=["engine_mean","engine_hybrid","engine_text"]
        cells:list[dict]=[dict(base,id="go-off",artifact="go",passes=core,observer=False),
               dict(base,id="go-on",artifact="go",passes=list(reversed(core)),observer=True)]
        snapshots=[]
        def monitor(pid):
            snapshots.append(source._accounting(pid,min(self.work_end,time.monotonic()+5)))
            return None
        with (self.task/"locks"/(source.GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=source._accounting(None,min(self.work_end,time.monotonic()+5))
            if before["util"] or before["total_MiB"]-before["used_MiB"]<8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            outcomes=source.run_manifest(dict(manifest,cells=cells),root,monitor=monitor,resources=self.resources)
        for outcome,cell in zip(outcomes,cells,strict=True):
            self.assertEqual(outcome["status"],"complete","real GO cell control NOT_RED")
            self.assertTrue(outcome["cleanup"]["complete"])
            self.assertTrue(any(row["pid"]==outcome["pid"] and row["own_MiB"]>0 and row["util"]>0 for row in snapshots),"actual GPU engagement absent NOT_RED")
            for ordinal,mode in enumerate(cell["passes"]):
                directory=Path(outcome["attempt"])/f"pass-{ordinal}"
                raw=[json.loads(line) for stream in ["warmups","samples"] for line in (directory/f"{stream}.jsonl").read_text().splitlines()]
                self.assertEqual(len(raw),5)
                self.assertTrue(all(row["ok"] and row["elapsed_ns"]>0 for row in raw))
                for row in raw:
                    self.assertEqual(len(row["observer"]["records"]),int(cell["observer"] and mode!="engine_mean"))
                    self.assertEqual((row["observer"]["dropped_delta"],row["observer"]["unpaired_records"]),(0,0))
        manifest["cells"].extend(cells)
        report=summarize(manifest,root)
        groups=[row for row in report["cells"] if row.get("cell_id") in {"go-off","go-on"}]
        self.assertEqual(len(groups),6)
        for group in groups:
            with self.subTest(cell=group["cell_id"],mode=group["mode"]):
                observed=group["observer"]
                self.assertIs(observed["available"],group["observer_enabled"],"missing actual GO transport availability")
                self.assertEqual(group["status"],"complete")
                if group["observer_enabled"]:
                    self.assertEqual(observed["query_records"],0 if group["mode"]=="mean" else 5)
                    self.assertEqual(observed["dropped"],0)
                    self.assertTrue(observed["attribution_complete"])
                    self.assertTrue(all("observer" in row for row in group["queries"]))
                else:
                    self.assertIsNone(observed["query_records"])
                    self.assertEqual(observed["reason"],"observer_disabled")
        self.assertEqual(len(report["pairs"]),6)
        for pair in report["pairs"]:
            self.assertEqual(pair["matched_calls"],3)
            self.assertTrue(pair["complete"])


class ObserverDeltaTests(unittest.TestCase):
    def test_report_uses_deltas_instead_of_summing_lifetime_counters(self):
        calls=[{"observer":{"records_available":True,"status":"observed","records":[{}],
                 "dropped_before":10,"dropped_after":12,"dropped_delta":2,"unpaired_records":0,"issues":[]}},
               {"observer":{"records_available":True,"status":"observed","records":[{}],
                 "dropped_before":12,"dropped_after":13,"dropped_delta":1,"unpaired_records":0,"issues":[]}}]
        result=observer_summary(calls,True)
        self.assertEqual(result.get("dropped"),3,"missing observer lifetime-delta report")
        self.assertEqual(result["query_records"],2)
        self.assertFalse(result["attribution_complete"])

    @settings(max_examples=3,deadline=None,database=None,derandomize=True,phases=[Phase.generate])
    @given(st.lists(st.integers(min_value=0,max_value=3),min_size=1,max_size=4))
    def test_delta_sum_and_unavailable_transport_remain_explicit(self,deltas):
        calls=[{"observer":{"records_available":True,"status":"observed","records":[{}],
                 "dropped_delta":delta,"unpaired_records":0,"issues":[]}} for delta in deltas]
        result=observer_summary(calls,True)
        self.assertEqual(result.get("query_records"),len(deltas),"missing observer stream counts")
        self.assertEqual(result["dropped"],sum(deltas))
        self.assertEqual(result["attribution_complete"],sum(deltas)==0)
        unavailable=observer_summary([{"observer":{"records_available":False,"status":"unavailable",
                   "records":[],"dropped_delta":None,"unpaired_records":None,"issues":["transport unavailable"]}}],True)
        self.assertIsNone(unavailable["query_records"])
        self.assertIsNone(unavailable["dropped"])
        self.assertFalse(unavailable["attribution_complete"])
        self.assertTrue(unavailable["issues"])


if __name__=="__main__":
    unittest.main()
