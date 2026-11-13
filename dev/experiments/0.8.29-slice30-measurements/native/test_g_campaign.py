"""Manual pinned-CUDA G manifest/controller integration, not a natural campaign."""

import fcntl
import hashlib
import json
import os
import subprocess
import shutil
from pathlib import Path
import sqlite3
from contextlib import closing
import sys
import tempfile
import time
import unittest

from hypothesis import Phase, given, settings, strategies as st

from test_g_text import GPU, _accounting

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from gd_campaign import _owned_stop_flags, _sample_resources, run_manifest  # noqa: E402

MODES = ["engine_mean", "sdk_mean", "engine_hybrid", "sdk_hybrid", "engine_text", "sdk_text"]


class ActualGCampaignTests(unittest.TestCase):
    def test_actual_g_manifest_completes_all_six_existing_modes(self):
        if os.environ.get("CUDA_VISIBLE_DEVICES") != GPU or os.environ.get("FATHOMDB_EMBED_DEVICE") != "cuda:0":
            raise RuntimeError("approved forced CUDA reservation missing NOT_RED")
        task = Path(os.environ["S30_GD_TASK"])
        features = json.loads(Path(os.environ["S30_G_CONTROLLER_FEATURES"]).read_text())
        binary = Path(features["binary"])
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        if features["binary_sha256"] != digest or features["normal_gn"] is not True:
            raise RuntimeError("actual normal GN artifact unavailable NOT_RED")
        for artifact in features["artifacts"]:
            if any(feature == "test-hooks" or feature.startswith("tc5-benchmark") or feature == "slice30-measurement" for feature in artifact["features"]):
                raise RuntimeError("normal G feature contamination NOT_RED")
        for row in json.loads((task/"receipts/gd-pinned-inputs.json").read_text())["copied"]:
            if hashlib.sha256(Path(row["destination"]).read_bytes()).hexdigest() != row["sha256"]:
                raise RuntimeError("existing pinned model bytes unavailable NOT_RED")
        root = Path(tempfile.mkdtemp(prefix="s30-g-controller-"))
        print(f"actual G controller {root} binary={digest}", flush=True)
        records = [{"id":"alpha", "kind":"doc", "body":"alphaonly épreuve"},
                   {"id":"beta", "kind":"doc", "body":"betaonly βήτα"}]
        records += [{"id":f"doc-{i}", "kind":"doc", "body":f"commonword natural record {i}"} for i in range(2,65)]
        corpus, questions = root/"records.jsonl", root/"queries.jsonl"
        corpus.write_text("".join(json.dumps(row)+"\n" for row in records))
        queries = [{"id":0, "query":"betaonly"}, {"id":1, "query":"alphaonly"}, {"id":2, "query":"missingnomatch"}]
        questions.write_text("".join(json.dumps(row)+"\n" for row in queries))
        cell = {"id":"g-six-modes", "artifact":"gn", "family":"G1_PLAN_FIXTURE", "gpu_uuid":GPU,
                "templates":[{"records":str(corpus)}], "queries":str(questions),
                "mode":MODES[0], "passes":MODES, "warmups":2, "repeat":0}
        manifest = {"schema":"slice30-gd/source-fixture-v1", "cell_seconds":150, "campaign_seconds":14400,
                    "artifacts":{"gn":{"path":str(binary),"sha256":digest,"features":["slice30-g"],"profile":"release"}}, "cells":[cell]}
        (root/"manifest.json").write_text(json.dumps(manifest))
        snapshots, errors = [], []
        deadline = time.monotonic()+150
        def monitor(pid):
            try:
                snapshots.append(_accounting(pid, deadline))
            except Exception as error:
                errors.append(str(error))
                return str(error)
            return None
        with (task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
            before = _accounting(None, deadline)
            if before["util"] != 0 or before["total_MiB"]-before["used_MiB"] < 8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            outcomes = run_manifest(manifest, root/"results", monitor=monitor)
        print(json.dumps({"root":str(root),"before":before,"gpu_samples":snapshots,"scope":"whole real preparation plus six API-mode series; no per-query GPU attribution"}), flush=True)
        if errors or outcomes[0]["exit"] != 0:
            raise RuntimeError(f"native/GPU accounting setup NOT_RED {errors} {outcomes}")
        if not any(row["pid"] == outcomes[0]["pid"] and row["own_MiB"] > 0 and row["util"] > 0 for row in snapshots):
            raise RuntimeError("actual own PID/UUID CUDA engagement absent NOT_RED")
        attempt = Path(outcomes[0]["attempt"])
        prepared = json.loads((attempt/"case-0/template.json").read_text())["preparation"]
        assert (prepared["api_complete"], prepared["write_rows"], prepared["write_calls"], prepared["pending"]) == (True,65,2,0)
        assert len(prepared["coverage"]["rows"]) == 65 and prepared["coverage"]["profile"]["dimension"] == 384
        with closing(sqlite3.connect(f"file:{attempt/'cell/store.sqlite'}?mode=ro", uri=True)) as db:
            actual = db.execute("SELECT logical_id,body FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY write_cursor").fetchall()
            assert actual == [(row["id"],row["body"]) for row in records]
        for ordinal, mode in enumerate(MODES):
            directory = attempt/"cell"/f"pass-{ordinal}"
            proof = json.loads((directory/"api-proof.json").read_text())
            assert proof["mode"] == mode and proof["selected_cuda_uuid"] == GPU and proof["cuda_compiled"] is True
            assert (proof["warmup_api_calls"],proof["timed_api_calls"],proof["api_calls"]) == (2,3,5)
            timed = [json.loads(line) for line in (directory/"samples.jsonl").read_text().splitlines()]
            warm = [json.loads(line) for line in (directory/"warmups.jsonl").read_text().splitlines()]
            assert len(warm) == 2 and len(timed) == 3
            assert [(row["query_id"],row["query_fingerprint"]) for row in timed] == [(row["id"],hashlib.sha256(row["query"].encode()).hexdigest()) for row in queries]
            assert all(row["ok"] and row["elapsed_ns"] > 0 for row in [*warm,*timed])
            if mode.endswith("_mean"):
                assert all(row["dimension"] == 384 and row["finite"] and row["norm_squared"] > 0 for row in [*warm,*timed])
            elif mode.endswith("_text"):
                assert [row["ids"] for row in timed] == [["l:beta"],["l:alpha"],[]]
            else:
                assert all((row["returned_hits"],row["soft_fallback"],row["degradations"]) == (10,"None","[]") for row in [*warm,*timed])
        print("actual normal G preparation/CUDA/SQL/all-six-mode API controls passed", flush=True)
        self.assertEqual(outcomes[0]["status"], "complete", "missing actual G manifest completion integration")


class _NativeGFixture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from test_gd_campaign import actual_resources
        cls.resources = actual_resources()
        cls.task = Path(os.environ["S30_GD_TASK"])
        cls.features = json.loads(Path(os.environ["S30_G_CONTROLLER_FEATURES"]).read_text())
        cls.binary = Path(cls.features["binary"])
        if cls.features["normal_gn"] is not True:
            raise RuntimeError("normal GN identity required NOT_RED")
        if os.environ.get("CUDA_VISIBLE_DEVICES") != GPU or os.environ.get("FATHOMDB_EMBED_DEVICE") != "cuda:0":
            raise RuntimeError("approved CUDA reservation required NOT_RED")
        for row in json.loads((cls.task/"receipts/gd-pinned-inputs.json").read_text())["copied"]:
            if hashlib.sha256(Path(row["destination"]).read_bytes()).hexdigest() != row["sha256"]:
                raise RuntimeError("pinned existing BGE files unavailable NOT_RED")
        cls.root = Path(tempfile.mkdtemp(prefix="s30-g-native-budget-"))
        cls.work_end = time.monotonic()+150
        cls.records = [{"id":f"budget-{i}","kind":"doc","body":f"actual BGE budget é {i}"} for i in range(65)]
        corpus = cls.root/"records.jsonl"
        corpus.write_text("".join(json.dumps(row)+"\n" for row in cls.records))
        cls.config = {"family":"G_TEMPLATE","gpu_uuid":GPU,"templates":[{"records":str(corpus)}],
            "work_deadline_ns":int(cls.work_end*1e9),"hard_deadline_ns":int((cls.work_end+30)*1e9),"shared_deadline_ns":int((cls.work_end+30)*1e9)}
        cls.ordinal = 0
        cls.stopped = None
        out = cls.invoke(dict(cls.config))
        if not (out/"case-0/template.sqlite").exists():
            raise RuntimeError("healthy actual BGE producer did not complete NOT_RED")
        cls.template = out/"case-0/template.sqlite"

    @classmethod
    def invoke(cls,config,relative_work_ns=None,allow_partial=False):
        from gd_resources import _cleanup_owned_group, _owned_subreaper
        if cls.stopped is not None:
            raise RuntimeError("source fixture stopped; no further native starts NOT_RED")
        _sample_resources(cls.resources,cls.work_end,start=True)
        case = cls.root/f"case-{cls.ordinal}"
        cls.ordinal += 1
        case.mkdir()
        path,out = case/"cell.json",case/"out"
        path.write_text(json.dumps(config))
        digest = hashlib.sha256(cls.binary.read_bytes()).hexdigest()
        if digest != cls.features["binary_sha256"]:
            raise RuntimeError("actual artifact differs from normal feature proof NOT_RED")
        print(f"G native admission {case} binary={digest}",flush=True)
        samples=[]
        with (cls.task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before = _accounting(None,min(cls.work_end,time.monotonic()+5))
            if before["util"] or before["total_MiB"]-before["used_MiB"] < 8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            with _owned_stop_flags() as stop, _owned_subreaper(),(case/"child.log").open("wb") as log:
                if relative_work_ns is not None:
                    config = dict(config,work_deadline_ns=min(config["work_deadline_ns"],time.monotonic_ns()+relative_work_ns))
                    path.write_text(json.dumps(config))
                child = subprocess.Popen([str(cls.binary),str(path),str(out)],stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                try:
                    while child.poll() is None:
                        if stop["signal"] is not None:
                            cls.stopped = stop["signal"]
                            raise RuntimeError("source fixture controlled stop NOT_RED")
                        if time.monotonic() >= cls.work_end:
                            raise RuntimeError("source work deadline NOT_RED")
                        samples.append(_accounting(child.pid,min(cls.work_end,time.monotonic()+5)))
                        try:
                            child.wait(timeout=min(0.1,max(0,cls.work_end-time.monotonic())))
                        except subprocess.TimeoutExpired:
                            pass
                finally:
                    cleanup = _cleanup_owned_group(child,deadline=min(cls.work_end+30,time.monotonic()+20))
                    cls.stopped = stop["signal"]
                if not cleanup["complete"] or child.returncode != 0:
                    raise RuntimeError(f"actual G runtime/setup NOT_RED {case} exit={child.returncode}")
        cls.last_gpu_samples = samples
        print(json.dumps({"pid":child.pid,"uuid":GPU,"peak_own_MiB":max((row["own_MiB"] for row in samples),default=0),
                          "positive_util":any(row["own_MiB"]>0 and row["util"]>0 for row in samples)}),flush=True)
        if (out/"case-0/template.sqlite").exists():
            if allow_partial and not (out/"case-0/template.json").exists() and (out/"budget.json").exists():
                return out
            report = json.loads((out/"case-0/template.json").read_text())["preparation"]
            if not (report["api_complete"] and report["selected_cuda_uuid"]==GPU and report["cuda_compiled"] and
                    report["write_rows"]==65 and report["write_calls"]==2 and report["pending"]==0 and
                    len(report["coverage"]["rows"])==65 and any(row["own_MiB"]>0 and row["util"]>0 for row in samples)):
                raise RuntimeError("actual normal CUDA preparation controls NOT_RED")
            with closing(sqlite3.connect(f"file:{out/'case-0/template.sqlite'}?mode=ro",uri=True)) as db:
                actual = db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall()
                if actual != [(row["id"],row["kind"],row["body"]) for row in cls.records]:
                    raise RuntimeError("actual copied input/SQL controls NOT_RED")
        return out



class NativeGAdmissionTests(_NativeGFixture):
    def test_each_expired_native_bound_prevents_default_bge_database_work(self):
        for bound in ["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"]:
            with self.subTest(bound=bound):
                config = dict(self.config)
                config[bound]=0
                out = self.invoke(config)
                self.assertFalse((out/"case-0/template.sqlite").exists(),"expired native G entry still opened/projected a real BGE store")
                report = json.loads((out/"budget.json").read_text())
                self.assertEqual(report["disposition"],"deadline_refused")
                self.assertEqual(report["phase"],"entry")

    def check_post_open(self,work_ns):
        out = self.invoke(dict(self.config),relative_work_ns=work_ns,allow_partial=True)
        store = out/"case-0/template.sqlite"
        if not store.exists():
            raise RuntimeError("post-open source phase not reached NOT_RED")
        proof = out/"case-0/api-proof.json"
        if proof.exists() and json.loads(proof.read_text())["open_ns"] <= work_ns:
            raise RuntimeError("actual Default open did not span the chosen source work window NOT_RED")
        with closing(sqlite3.connect(f"file:{store}?mode=ro",uri=True)) as db:
            rows = db.execute("SELECT count(*) FROM canonical_nodes").fetchone()[0]
        print(f"actual post-Default-open rows={rows} work_ns={work_ns} store={store}",flush=True)
        self.assertEqual(rows,0,"native G continued configuration/writes after its real Default open exhausted work time")
        report = json.loads((out/"budget.json").read_text())
        self.assertEqual(report["phase"],"configure")
        self.assertEqual(report["disposition"],"deadline_refused")
        self.assertNotIn("sqlite_created",report)
        config = json.loads((out.parent/"cell.json").read_text())
        self.assertGreaterEqual(report["observed_at_ns"],config["work_deadline_ns"])
        self.assertFalse(proof.exists())

    def test_post_default_open_expiry_prevents_following_configuration_and_writes(self):
        self.check_post_open(250_000_000)

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.integers(min_value=150_000_000,max_value=250_000_000))
    def test_tight_source_windows_preserve_empty_store_after_open_expiry(self,work_ns):
        self.check_post_open(work_ns)

    def check_pass_open(self,mode,warmups):
        store = self.root/f"pass-store-{self.ordinal}.sqlite"
        wal = Path(str(self.template)+"-wal")
        if wal.exists() and wal.stat().st_size:
            raise RuntimeError("source closed template WAL nonempty NOT_RED")
        shutil.copyfile(self.template,store)
        if hashlib.sha256(store.read_bytes()).digest() != hashlib.sha256(self.template.read_bytes()).digest():
            raise RuntimeError("fresh closed template copy invalid NOT_RED")
        questions = self.root/"pass-queries.jsonl"
        inputs = [{"id":i,"query":"budget"} for i in range(128)]
        questions.write_text("".join(json.dumps(row)+"\n" for row in inputs))
        config = dict(self.config,family="G_PASS",mode=mode,store=str(store),queries=str(questions),warmups=warmups)
        out = self.invoke(config,relative_work_ns=250_000_000,allow_partial=True)
        proof_path = out/"api-proof.json"
        calls = []
        for name in ["warmups.jsonl","samples.jsonl"]:
            path = out/name
            if path.exists():
                calls.extend(json.loads(line) for line in path.read_text().splitlines())
        if proof_path.exists():
            proof = json.loads(proof_path.read_text())
            if proof["open_ns"] <= 250_000_000:
                raise RuntimeError("actual pass open did not span source work window NOT_RED")
            self.assertEqual(proof["selected_cuda_uuid"],GPU)
            self.assertEqual((proof["warmup_api_calls"],proof["timed_api_calls"]),(warmups,128))
            self.assertEqual([row["query_id"] for row in calls],list(range(warmups))+list(range(128)))
            self.assertTrue(all(row["ok"] and row["elapsed_ns"]>0 and row["query_fingerprint"]==hashlib.sha256(b"budget").hexdigest() for row in calls))
            if mode.endswith("_mean"):
                self.assertTrue(all(row["dimension"]==384 and row["finite"] and row["norm_squared"]>0 for row in calls))
            else:
                self.assertTrue(all(row["returned_hits"]==10 and len(set(row["ids"]))==10 for row in calls))
            if not mode.endswith("_text") and not any(row["own_MiB"]>0 and row["util"]>0 for row in self.last_gpu_samples):
                raise RuntimeError("actual inference series GPU engagement absent NOT_RED")
        print(f"actual post-pass-open mode={mode} warmups={warmups} calls={len(calls)} root={out}",flush=True)
        self.assertEqual(len(calls),0,"generic G pass invoked queries after Default open exhausted its work endpoint")
        self.assertFalse(proof_path.exists())
        self.assertEqual(json.loads((out/"budget.json").read_text())["phase"],"pass")

    def test_all_six_modes_stop_after_expired_real_pass_open(self):
        for mode in MODES:
            with self.subTest(mode=mode):
                self.check_pass_open(mode,0)

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.sampled_from(MODES),st.integers(min_value=1,max_value=3))
    def test_expired_pass_cannot_start_positive_warmups(self,mode,warmups):
        self.check_pass_open(mode,warmups)


    def check_running_phase(self, mode, phase):
        store = self.root/f"running-store-{self.ordinal}.sqlite"
        shutil.copyfile(self.template,store)
        if hashlib.sha256(store.read_bytes()).digest() != hashlib.sha256(self.template.read_bytes()).digest():
            raise RuntimeError("actual closed-template copy failed NOT_RED")
        queries = self.root/f"running-queries-{self.ordinal}.jsonl"
        inputs = [{"id":i,"query":"budget"} for i in range(2048)]
        queries.write_text("".join(json.dumps(row)+"\n" for row in inputs))
        config = dict(self.config,family="G_PASS",store=str(store),queries=str(queries),mode=mode,
                      warmups=len(inputs) if phase=="warmup" else 0)
        out = self.invoke(config,relative_work_ns=2_000_000_000)
        path = out/("warmups.jsonl" if phase=="warmup" else "samples.jsonl")
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        if not rows or not any(row["own_MiB"]>0 and row["util"]>0 for row in self.last_gpu_samples):
            raise RuntimeError("actual inference prefix/own CUDA engagement absent NOT_RED")
        self.assertEqual([row["query_id"] for row in rows],list(range(len(rows))))
        self.assertTrue(all(row["query_fingerprint"]==hashlib.sha256(b"budget").hexdigest() and
                            row["ok"] and row["dimension"]==384 and row["finite"] and
                            row["norm_squared"]>0 and row["elapsed_ns"]>0 for row in rows))
        print(f"actual running {phase} {mode} prefix={len(rows)}/{len(inputs)} root={out}",flush=True)
        self.assertLess(len(rows),len(inputs),"native G continued starting inference calls after its original work endpoint")
        self.assertFalse((out/"api-proof.json").exists(),"expired partial pass reported completion")
        self.assertEqual(json.loads((out/"budget.json").read_text())["phase"],phase)
        if phase=="warmup":
            self.assertEqual((out/"samples.jsonl").read_text(),"")

    def test_running_warmup_and_timed_phases_stop_with_actual_prefix(self):
        for phase in ["warmup","timed"]:
            with self.subTest(phase=phase):
                self.check_running_phase("engine_mean",phase)

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.sampled_from(["engine_mean","sdk_mean"]),st.sampled_from(["warmup","timed"]))
    def test_running_mean_prefix_preserves_input_order(self,mode,phase):
        self.check_running_phase(mode,phase)


    def check_scheduled_drain_budgets(self,mode):
        store=self.root/f"scheduled-store-{self.ordinal}.sqlite"
        shutil.copyfile(self.template,store)
        self.assertEqual(hashlib.sha256(store.read_bytes()).digest(),hashlib.sha256(self.template.read_bytes()).digest())
        queries=self.root/f"scheduled-queries-{self.ordinal}.jsonl"
        inputs=[{"id":i,"query":"budget"} for i in range(20)]
        queries.write_text("".join(json.dumps(row)+"\n" for row in inputs))
        config=dict(self.config,family="G_PASS",store=str(store),queries=str(queries),mode=mode,
                    population="G2",schedule="write",warmups=2)
        out=self.invoke(config,relative_work_ns=10_000_000_000)
        actual=json.loads((out.parent/"cell.json").read_text())
        deadline=min(actual[key] for key in ["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])
        proof=json.loads((out/"api-proof.json").read_text())
        timed=[json.loads(line) for line in (out/"samples.jsonl").read_text().splitlines()]
        phases=[json.loads(line) for line in (out/"batches.jsonl").read_text().splitlines()]
        probes=[json.loads(line) for line in (out/"marker-probes.jsonl").read_text().splitlines()]
        self.assertEqual((proof["api_calls"],proof["warmup_api_calls"],len(timed),len(probes)),(22,2,20,20))
        self.assertEqual([row["query_id"] for row in timed],list(range(20)))
        self.assertTrue(all(row["ok"] and row["elapsed_ns"]>0 for row in timed))
        self.assertTrue(all(row["ok"] and row["marker_current"] for row in probes))
        drains=[row for row in phases if row["event"]=="projection_drained"]
        self.assertEqual([row["after_natural_calls"] for row in drains],[10,20])
        self.assertEqual(proof["g2_reset"]["coverage"]["current_by_kind"],{"doc":65,"note":10})
        for phase,report in [("reset",proof["g2_reset"].get("drain_budget",{})),
                             *[(f"cohort-{index}",row.get("drain_budget",{})) for index,row in enumerate(drains)]]:
            with self.subTest(mode=mode,phase=phase):
                self.assertEqual(report.get("drain_calls",0),1,"G2 reset/scheduled drain still lacks actual remaining-budget forwarding")
                expected=(deadline-report["start_ns"])//1_000_000
                self.assertEqual(report["timeout_ms"],expected)
                self.assertGreater(report["timeout_ms"],5000)

    def test_all_sdk_schedules_forward_original_drain_budget(self):
        for mode in ["sdk_text","sdk_hybrid","sdk_mean"]:
            with self.subTest(mode=mode):
                self.check_scheduled_drain_budgets(mode)

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.sampled_from(["sdk_text","sdk_hybrid","sdk_mean"]))
    def test_scheduled_budget_never_renews_between_cohorts(self,mode):
        self.check_scheduled_drain_budgets(mode)




class OwnedGTTests(_NativeGFixture):
    def check_owned_preparation(self,suffix):
        root=Path(tempfile.mkdtemp(prefix="owned-preparation-",dir=self.root))
        records=[f"budget {suffix} {i}" for i in range(65)]
        corpus,questions=root/"corpus.jsonl",root/"questions.jsonl"
        corpus.write_text("".join(json.dumps(body)+"\n" for body in records))
        questions.write_text("".join(json.dumps(query)+"\n" for query in ["budget","été","missingnomatch"]))
        cell=dict(self.config,id="gt-source",family="G_PREPARE",artifact="gn",gpu_uuid=GPU,
                  n=65,corpus=str(corpus),questions=str(questions),query_count=3)
        digest=hashlib.sha256(self.binary.read_bytes()).hexdigest()
        if digest!=self.features["binary_sha256"]:
            raise RuntimeError("actual normal artifact unavailable NOT_RED")
        print(f"owned GT {root} normal_binary={digest}",flush=True)
        manifest={"schema":"slice30-gd/source-fixture-v1","cell_seconds":150,"campaign_seconds":14400,
                  "artifacts":{"gn":{"path":str(self.binary),"sha256":digest}},"cells":[cell]}
        snapshots=[]
        def monitor(pid):
            snapshots.append(_accounting(pid,min(self.work_end,time.monotonic()+5)))
            return None
        _sample_resources(self.resources,self.work_end,start=True)
        with (self.task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=_accounting(None,min(self.work_end,time.monotonic()+5))
            if before["util"] or before["total_MiB"]-before["used_MiB"]<8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            outcome=run_manifest(manifest,root/"results",monitor=monitor,resources=self.resources)[0]
        if outcome["exit"]!=0:
            raise RuntimeError(f"owned preparation setup/accounting failure NOT_RED {outcome}")
        attempt=Path(outcome["attempt"])
        store=attempt/"case-0/template.sqlite"
        count=0
        if store.exists():
            if not any(row["pid"]==outcome["pid"] and row["own_MiB"]>0 and row["util"]>0 for row in snapshots):
                raise RuntimeError("actual owned preparation CUDA engagement missing NOT_RED")
            with closing(sqlite3.connect(f"file:{store}?mode=ro",uri=True)) as db:
                actual=db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall()
                count=len(actual)
                self.assertEqual(actual,[(f"s30doc-{i}","doc",body) for i,body in enumerate(records)])
                self.assertEqual(db.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0],65)
                self.assertEqual(db.execute("SELECT count(*) FROM _fathomdb_vector_spans").fetchone()[0],65)
            report=json.loads((attempt/"case-0/template.json").read_text())["preparation"]
            self.assertEqual((report["api_complete"],report["write_rows"],report["write_calls"],report["pending"]),(True,65,2,0))
            self.assertEqual(hashlib.sha256(store.read_bytes()).hexdigest(),report["closed_main_sha256"])
            self.assertTrue(not Path(str(store)+"-wal").exists() or Path(str(store)+"-wal").stat().st_size==0)
            encoded=[json.loads(line) for line in (attempt/"questions.jsonl").read_text().splitlines()]
            self.assertEqual(encoded,[{"id":0,"query":"budget"},{"id":1,"query":"été"},{"id":2,"query":"missingnomatch"}])
            self.assertTrue(outcome["cleanup"]["complete"])
        print(f"actual owned GT {attempt} produced={count}/65 pid={outcome['pid']}",flush=True)
        self.assertEqual(count,65,"owned GT controller child did not encode and execute actual normal BGE preparation")
        return outcome

    def test_owned_gt_child_executes_real_closed_template_preparation(self):
        self.check_owned_preparation("épreuve β")

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.text(alphabet="abcéβ常",min_size=1,max_size=8))
    def test_owned_preparation_preserves_selected_unicode_inputs(self,suffix):
        self.check_owned_preparation(suffix)


    def test_actual_gt_preparation_is_recognized_complete(self):
        outcome=self.check_owned_preparation("completion é")
        self.assertEqual(outcome["status"],"complete","actual closed GT output is still rejected by the controller completion adapter")

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.text(alphabet="abcéβ常",min_size=1,max_size=8))
    def test_actual_prepared_unicode_population_completes_controller(self,suffix):
        outcome=self.check_owned_preparation(suffix)
        self.assertEqual(outcome["status"],"complete","actual closed GT output is still rejected by the controller completion adapter")


class CopiedGCellTests(_NativeGFixture):
    check_owned_preparation=OwnedGTTests.check_owned_preparation

    def check_copy(self,suffix):
        producer=self.check_owned_preparation(suffix)
        self.assertEqual(producer["status"],"complete","real GT completion control NOT_RED")
        results=Path(producer["attempt"]).parents[1]
        source=Path(producer["attempt"])/"case-0/template.sqlite"
        source_hash=hashlib.sha256(source.read_bytes()).hexdigest()
        cell=dict(self.config,id="g1-source",family="G1_CELL",artifact="gn",n=65,
                  template_from="gt-source",queries_from="gt-source",passes=MODES,warmups=2,query_count=3)
        artifact={"path":str(self.binary),"sha256":self.features["binary_sha256"]}
        manifest={"schema":"slice30-gd/source-fixture-v1","cell_seconds":150,"campaign_seconds":14400,
                  "artifacts":{"gn":artifact},"cells":[cell]}
        snapshots=[]
        def monitor(pid):
            snapshots.append(_accounting(pid,min(self.work_end,time.monotonic()+5)))
            return None
        _sample_resources(self.resources,self.work_end,start=True)
        with (self.task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=_accounting(None,min(self.work_end,time.monotonic()+5))
            if before["util"] or before["total_MiB"]-before["used_MiB"]<8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            outcome=run_manifest(manifest,results,monitor=monitor,resources=self.resources)[0]
        if outcome["exit"]!=0:
            raise RuntimeError(f"copied source cell setup/accounting failure NOT_RED {outcome}")
        attempt=Path(outcome["attempt"])
        calls=[]
        for ordinal,mode in enumerate(MODES):
            directory=attempt/f"pass-{ordinal}"
            path=directory/"samples.jsonl"
            if not path.exists():
                continue
            rows=[json.loads(line) for line in path.read_text().splitlines()]
            warm=[json.loads(line) for line in (directory/"warmups.jsonl").read_text().splitlines()]
            self.assertEqual([row["query_id"] for row in rows],[0,1,2])
            self.assertEqual([row["query_id"] for row in warm],[0,1])
            self.assertTrue(all(row["ok"] and row["elapsed_ns"]>0 for row in [*warm,*rows]))
            self.assertEqual([row["query_fingerprint"] for row in rows],[hashlib.sha256(text.encode()).hexdigest() for text in ["budget","été","missingnomatch"]])
            if mode.endswith("_mean"):
                self.assertTrue(all(row["dimension"]==384 and row["finite"] and row["norm_squared"]>0 for row in rows))
            elif mode.endswith("_text"):
                self.assertEqual([row["returned_hits"] for row in rows],[10,0,0])
            else:
                self.assertTrue(all(row["returned_hits"]==10 and row["soft_fallback"]=="None" and row["degradations"]=="[]" for row in rows))
            calls.extend(rows)
        store=attempt/"store.sqlite"
        if store.exists():
            with closing(sqlite3.connect(f"file:{store}?mode=ro",uri=True)) as db:
                self.assertEqual(db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall(),
                                 [(f"s30doc-{i}","doc",f"budget {suffix} {i}") for i in range(65)])
                self.assertEqual(db.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0],65)
                self.assertEqual(db.execute("SELECT count(*) FROM _fathomdb_vector_spans").fetchone()[0],65)
        print(f"actual dependent G1 {attempt} timed={len(calls)}/18",flush=True)
        self.assertEqual(len(calls),18,"completed GT template was not copied into an actual six-mode dependent cell")
        self.assertEqual(outcome["status"],"complete")
        copy=json.loads((attempt/"copy.json").read_text())
        self.assertEqual((copy["source_main_sha256"],copy["initial_copy_sha256"]),(source_hash,source_hash))
        self.assertGreater(copy["copy_ns"],0)
        self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(),source_hash)
        self.assertTrue(any(row["pid"]==outcome["pid"] and row["own_MiB"]>0 and row["util"]>0 for row in snapshots))
        self.assertTrue(outcome["cleanup"]["complete"])
        manifest["cells"]=[producer["cell"],cell]
        # The stored producer already includes its selected native identity.
        resumed=run_manifest(manifest,results,resources=self.resources)
        self.assertEqual([row["attempt"] for row in resumed],[producer["attempt"],outcome["attempt"]])
        return manifest,results,outcome

    def test_closed_gt_copy_drives_real_six_mode_cell_and_resume(self):
        self.check_copy("copied é")

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.text(alphabet="abcéβ常",min_size=1,max_size=8))
    def test_copied_unicode_source_keeps_template_and_query_population(self,suffix):
        self.check_copy(suffix)


class ConditionalGTests(_NativeGFixture):
    check_owned_preparation=OwnedGTTests.check_owned_preparation

    def check_admission(self,feasible,tighter=None):
        lower=self.check_owned_preparation("conditional é")
        self.assertEqual(lower["status"],"complete","actual completed lower population NOT_RED")
        results=Path(lower["attempt"]).parents[1]
        source=results.parent/"target-corpus.jsonl"
        source.write_text("".join(json.dumps(f"budget target {i}")+"\n" for i in range(66)))
        projected=(lower["cell_elapsed_ns"]*66*3+65*2-1)//(65*2)
        seconds=150 if feasible or tighter else 30+projected*0.75/1_000_000_000
        cell=dict(self.config,id="g/gt-target",family="G_PREPARE",artifact="gn",group="GT",n=66,
                  lower="gt-source",corpus=str(source),questions=lower["cell"]["questions"],query_count=3)
        manifest={"schema":"slice30-gd/source-fixture-v1","cell_seconds":seconds,"campaign_seconds":14400,
                  "artifacts":{"gn":{"path":str(self.binary),"sha256":self.features["binary_sha256"]}},"cells":[cell]}
        snapshots=[]
        def monitor(pid):
            snapshots.append(_accounting(pid,min(self.work_end,time.monotonic()+5)))
            return None
        _sample_resources(self.resources,self.work_end,start=True)
        with (self.task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=_accounting(None,min(self.work_end,time.monotonic()+5))
            if before["util"] or before["total_MiB"]-before["used_MiB"]<8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            if tighter:
                endpoint=time.monotonic_ns()+projected*9//10
                cell[tighter]=endpoint+(0 if tighter=="work_deadline_ns" else 30_000_000_000)
            target=run_manifest(manifest,results,monitor=monitor,resources=self.resources)[0]
        print(f"actual conditional GT {results} lower_ns={lower['cell_elapsed_ns']} projected={projected} feasible={feasible} pid={target['pid']}",flush=True)
        if target["pid"] is not None:
            if target["exit"]!=0 or target["status"]!="complete":
                raise RuntimeError(f"actual target66 native/runtime setup NOT_RED {target}")
            if not any(row["pid"]==target["pid"] and row["own_MiB"]>0 and row["util"]>0 for row in snapshots):
                raise RuntimeError("actual target CUDA engagement absent NOT_RED")
            with closing(sqlite3.connect(f"file:{Path(target['attempt'])/'case-0/template.sqlite'}?mode=ro",uri=True)) as db:
                self.assertEqual(db.execute("SELECT count(*) FROM canonical_nodes").fetchone()[0],66)
        elif target.get("reason")!="conditional_not_feasible":
            raise RuntimeError(f"external admission/resource failure NOT_RED {target}")
        if not feasible:
            self.assertIsNone(target["pid"],"lower-scale projection exceeded original remaining work but target native child was launched")
            self.assertEqual(target["status"],"not_run_resource")
        else:
            self.assertEqual(target["status"],"complete")
        report=(target.get("admission") or (target.get("environment_before") or {}).get("admission"))
        assert report is not None,"actual conditional decision did not retain its observed lower-scale forecast"
        self.assertEqual(report["time"]["projected_ns"],projected)
        self.assertGreater(report["storage"]["additional_bytes"],0)

    def test_tighter_cell_supplied_endpoints_constrain_conditional_launch(self):
        for bound in ["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"]:
            with self.subTest(bound=bound):
                self.check_admission(False,tighter=bound)

    def test_actual_lower_scale_refuses_target_before_remaining_work_is_exceeded(self):
        self.check_admission(False)

    def test_fitting_actual_source_target_retains_admission_observations(self):
        self.check_admission(True)

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.booleans())
    def test_actual_lower_observation_admits_only_fitting_source_targets(self,feasible):
        self.check_admission(feasible)



class GpuAccountingTests(_NativeGFixture):
    def check_accounting(self, suffix):
        root=Path(tempfile.mkdtemp(prefix="owned-gpu-accounting-",dir=self.root))
        corpus, questions=root/"corpus.jsonl",root/"questions.jsonl"
        bodies=[f"actual accounted budget {suffix} {i}" for i in range(65)]
        corpus.write_text("".join(json.dumps(body)+"\n" for body in bodies))
        questions.write_text(json.dumps("budget")+"\n")
        cell=dict(self.config,id="accounted-gt",family="G_PREPARE",artifact="gn",n=65,
                  gpu_reservation=True,corpus=str(corpus),questions=str(questions),query_count=1)
        manifest={"schema":"slice30-gd/source-fixture-v1","cell_seconds":150,"campaign_seconds":14400,
                  "artifacts":{"gn":{"path":str(self.binary),"sha256":self.features["binary_sha256"]}},"cells":[cell]}
        independent=[]
        def monitor(pid):
            independent.append(_accounting(pid,min(self.work_end,time.monotonic()+5)))
            return None
        outcome=run_manifest(manifest,root/"results",monitor=monitor,resources=self.resources,gpu_interval=0.1)[0]
        if outcome["exit"]!=0 or outcome["status"]!="complete":
            raise RuntimeError(f"actual normal CUDA preparation setup NOT_RED {outcome}")
        attempt=Path(outcome["attempt"])
        store=attempt/"case-0/template.sqlite"
        with closing(sqlite3.connect(f"file:{store}?mode=ro",uri=True)) as db:
            self.assertEqual(db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall(),
                             [(f"s30doc-{i}","doc",body) for i,body in enumerate(bodies)])
            self.assertEqual(db.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0],65)
            self.assertEqual(db.execute("SELECT count(*) FROM _fathomdb_vector_spans").fetchone()[0],65)
        preparation=json.loads((attempt/"case-0/template.json").read_text())["preparation"]
        self.assertEqual((preparation["selected_cuda_uuid"],preparation["pending"],preparation["write_rows"]),(GPU,0,65))
        self.assertEqual(hashlib.sha256(store.read_bytes()).hexdigest(),preparation["closed_main_sha256"])
        if not any(row["pid"]==outcome["pid"] and row["own_MiB"]>0 and row["util"]>0 for row in independent):
            raise RuntimeError("actual independent PID/UUID positive CUDA engagement missing NOT_RED")
        print(f"actual controller GPU series {attempt} pid={outcome['pid']} independent_positive=True",flush=True)
        samples=outcome.get("gpu_samples") or []
        self.assertTrue(samples,"controller omitted its own required GPU accounting records")
        self.assertEqual(samples[0]["phase"],"before")
        self.assertEqual(samples[0]["own_MiB"],0)
        self.assertEqual(samples[-1]["phase"],"final")
        self.assertTrue(all(row["uuid"]==GPU and 0<=row["own_MiB"]<=8192 for row in samples))
        self.assertTrue(any(row["phase"]=="running" and row["pid"]==outcome["pid"]
                            and row["own_MiB"]>0 and row["util"]>0 for row in samples))
        self.assertTrue(outcome["cleanup"]["complete"])
        resumed=run_manifest(manifest,root/"results",resources=self.resources,gpu_interval=0.1)
        self.assertEqual(resumed[0]["attempt"],outcome["attempt"])

    def test_controller_retains_real_reserved_cuda_series_accounting(self):
        self.check_accounting("évidence")

    @settings(max_examples=2,deadline=None,database=None,phases=[Phase.generate])
    @given(st.text(alphabet="abcéβ常",min_size=1,max_size=4))
    def test_accounted_preparation_preserves_selected_unicode_rows(self,suffix):
        self.check_accounting(suffix)

if __name__ == "__main__":
    unittest.main()
