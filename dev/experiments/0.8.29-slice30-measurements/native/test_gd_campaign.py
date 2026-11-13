"""Tiny real manifest/controller/native D integration; not campaign qualification."""

import hashlib
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest

from hypothesis import Phase, given, settings, strategies as st

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from gd_campaign import run_manifest  # noqa: E402


def verify(rows, supervised=False):
    root = Path(tempfile.mkdtemp(prefix="s30-gd-controller-"))
    binary = Path(os.environ["CARGO_TARGET_DIR"])/"release/examples/slice30_gd_diagnostic"
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    print(f"source controller {root} binary={digest}", flush=True)
    records = [{"id":f"manifest-{i}", "kind":"doc" if i%2==0 else "note",
                "body":f"campaignwitness évidence {i}"} for i in range(rows)]
    base = {"family":"D_QUERY_FIXTURE", "records":[], "empty_start":True,
            "write_records":records, "cohort":2, "warmups":2, "samples":3,
            "queries":[{"id":"actual-query", "query":"campaignwitness"}],
            "readiness":True, "artifact":"do"}
    cells = [dict(base, id="d-off", observer=False), dict(base, id="d-on", observer=True)]
    artifact = {"path":str(binary), "sha256":digest,
                "features":["test-hooks","slice30-measurement"], "profile":"release"}
    manifest = {"schema":"slice30-gd/source-fixture-v1", "cell_seconds":180,
                "campaign_seconds":14400, "artifacts":{"do":artifact}, "cells":cells}
    path = root/"manifest.json"
    path.write_text(json.dumps(manifest))
    control = root/"direct-control"
    config = root/"direct.json"
    config.write_text(json.dumps(cells[0]))
    direct = subprocess.run([str(binary), str(config), str(control)], capture_output=True, text=True, timeout=150)
    assert direct.returncode == 0, f"native fixture setup NOT_RED {direct.stderr}"
    assert json.loads((control/"complete.json").read_text())["complete"] is True
    observations = []
    def observe(pid):
        observations.append({"pid":pid, "alive":Path(f"/proc/{pid}").exists()})
        return None
    outcomes = run_manifest(json.loads(path.read_text()), root/"results",
                            monitor=observe if supervised else None)
    assert [outcome["id"] for outcome in outcomes] == ["d-off", "d-on"]
    attempts = []
    for outcome, cell in zip(outcomes, cells, strict=True):
        assert outcome["exit"] == 0, f"native controller setup NOT_RED {outcome}"
        attempt = Path(outcome["attempt"])
        attempts.append(str(attempt))
        assert json.loads((attempt/"complete.json").read_text())["complete"] is True
        with closing(sqlite3.connect(f"file:{attempt/'query.sqlite'}?mode=ro", uri=True)) as db:
            actual = db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall()
            assert actual == [(record["id"],record["kind"],record["body"]) for record in records]
            for table in ["_fathomdb_vector_rows","_fathomdb_vector_spans"]:
                assert db.execute(f"SELECT count(*) FROM {table}").fetchone()[0] == rows
        calls = [json.loads(line) for line in (attempt/"queries.jsonl").read_text().splitlines()]
        assert [call["phase"] for call in calls] == ["warmup"]*2+["timed"]*3
        assert all(call["ok"] and call["query_id"] == "actual-query" for call in calls)
        writes = json.loads((attempt/"writes.json").read_text())
        assert len(writes) == (rows+1)//2 and sum(write["rows"] for write in writes) == rows
        observer = json.loads((attempt/"worker-observer.json").read_text())
        assert observer["available"] is cell["observer"]
        if cell["observer"]:
            assert sum(worker["job_count"] for worker in observer["records"]) == rows
        else:
            assert observer["records"] is None
    print("actual controller child calls/counts/SQL/feature controls passed", flush=True)
    assert [outcome["status"] for outcome in outcomes] == ["complete", "complete"], "missing D completion protocol integration in real controller"
    state = json.loads((root/"results/campaign-state.json").read_text())
    assert [row["id"] for row in state] == ["d-off", "d-on"]
    resumed = run_manifest(manifest, root/"results")
    assert [outcome["attempt"] for outcome in resumed] == attempts
    assert len(list((root/"results").glob("*/attempt-*"))) == 2
    if supervised:
        live = {row["pid"] for row in observations if row["alive"]}
        print(f"actual native child monitor observations {observations}", flush=True)
        assert all(outcome["pid"] in live for outcome in outcomes), "missing live native child supervision before completion"



class ActualCampaignTests(unittest.TestCase):
    def test_real_two_cell_manifest_uses_actual_native_completion_and_resume(self):
        verify(3)

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=1, max_value=5))
    def test_real_manifest_preserves_population_tail_and_cell_order(self, rows):
        verify(rows)


class SupervisedCampaignTests(unittest.TestCase):
    def test_actual_children_are_observed_while_alive(self):
        verify(3, supervised=True)

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=1, max_value=5))
    def test_live_supervision_preserves_real_native_population_and_resume(self, rows):
        verify(rows, supervised=True)


def clock_fixture():
    root = Path(tempfile.mkdtemp(prefix="s30-shared-clock-"))
    binary = Path(os.environ["CARGO_TARGET_DIR"])/"release/examples/slice30_gd_diagnostic"
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    print(f"shared clock fixture {root} binary={digest}", flush=True)
    records = [{"id":"clock-row", "kind":"doc", "body":"clockwitness évidence"}]
    base = {"family":"D_QUERY_FIXTURE", "records":[], "empty_start":True, "write_records":records,
            "cohort":1, "warmups":1, "samples":2, "queries":[{"id":0,"query":"clockwitness"}],
            "readiness":True, "observer":False, "artifact":"do"}
    manifest = {"schema":"slice30-gd/source-fixture-v1", "cell_seconds":150, "campaign_seconds":14400,
                "artifacts":{"do":{"path":str(binary),"sha256":digest}},
                "cells":[dict(base,id="clock-a"),dict(base,id="clock-b")]}
    return root, binary, manifest


def prepare_actual_database(root, binary, name):
    config = root/f"{name}.json"
    config.write_text(json.dumps({"family":"NATIVE_BUDGET_CONTROL","work_deadline_ns":2**64-1,"hard_deadline_ns":2**64-1,"shared_deadline_ns":2**64-1}))
    out = root/name
    result = subprocess.run([str(binary),str(config),str(out)],capture_output=True,text=True,timeout=150)
    assert result.returncode == 0, f"real preparation setup NOT_RED {result.stderr}"
    assert json.loads((out/"budget.json").read_text())["open_calls"] == 1
    assert (out/"store.sqlite").exists()


class SharedClockCampaignTests(unittest.TestCase):
    def test_clock_begins_before_real_preparation_and_is_shared_by_cells(self):
        root, binary, manifest = clock_fixture()
        clock = root/"results/campaign-clock.json"
        preparations = []
        def before():
            observed = json.loads(clock.read_text()) if clock.exists() else None
            start = time.monotonic_ns()
            prepare_actual_database(root,binary,f"prepare-{len(preparations)}")
            record = {"clock":observed,"preparation_start_ns":start,"preparation_end_ns":time.monotonic_ns()}
            preparations.append(record)
            return record
        outcomes = run_manifest(manifest,root/"results",before=before)
        assert [row["status"] for row in outcomes] == ["complete","complete"]
        assert len(preparations) == 2
        actual = [(row["clock"] or {}).get("started_ns") for row in preparations]
        print(f"clock before actual preparations={actual}", flush=True)
        self.assertTrue(all(isinstance(start,int) and start <= row["preparation_start_ns"] for start,row in zip(actual,preparations,strict=True)),"missing shared clock before actual preparation")
        self.assertEqual(len(set(actual)),1)
        saved = clock.read_bytes()
        resumed = run_manifest(manifest,root/"results",before=before)
        self.assertEqual([row["attempt"] for row in resumed],[row["attempt"] for row in outcomes])
        self.assertEqual(len(preparations),2)
        self.assertEqual(clock.read_bytes(),saved)

    def test_elapsed_actual_preparation_cannot_gain_fresh_budget_on_resume(self):
        root, binary, manifest = clock_fixture()
        results = root/"results"
        results.mkdir()
        clock = results/"campaign-clock.json"
        started = time.monotonic_ns()
        clock.write_text(json.dumps({"started_ns":started,"boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip()}))
        prepare_actual_database(root,binary,"already-prepared")
        manifest["campaign_seconds"] = (time.monotonic_ns()-started)/2_000_000_000
        saved = clock.read_bytes()
        outcomes = run_manifest(manifest,results)
        for row in outcomes:
            if row["status"] == "complete":
                assert json.loads((Path(row["attempt"])/"complete.json").read_text())["complete"] is True
        self.assertEqual([row["status"] for row in outcomes],["not_run_resource","not_run_resource"],"elapsed preparation/shared clock was renewed or ignored")
        self.assertTrue(all(row["reason"] == "campaign_time_cap" and row["pid"] is None for row in outcomes))
        self.assertEqual(list(results.glob("*/attempt-*")),[])
        again = run_manifest(manifest,results)
        self.assertEqual([row["status"] for row in again],["not_run_resource","not_run_resource"])
        self.assertEqual(clock.read_bytes(),saved)


class NativeAdmissionCampaignTests(unittest.TestCase):
    def check_native_admission(self, bound):
        root, binary, manifest = clock_fixture()
        manifest["cells"] = manifest["cells"][:1]
        healthy = run_manifest(manifest, root/"healthy")
        self.assertEqual(healthy[0]["status"], "complete", "real D API/setup control NOT_RED")
        attempt = Path(healthy[0]["attempt"])
        self.assertEqual(len((attempt/"queries.jsonl").read_text().splitlines()), 3)
        self.assertTrue((attempt/"query.sqlite").exists())
        cell = manifest["cells"][0]
        cell.update(work_deadline_ns=2**64-1, hard_deadline_ns=2**64-1, shared_deadline_ns=2**64-1)
        cell[bound] = 0
        config, attempt = root/"expired.json", root/"expired-native"
        config.write_text(json.dumps(cell))
        child = subprocess.run([str(binary),str(config),str(attempt)],capture_output=True,text=True,timeout=150)
        self.assertEqual(child.returncode,0, f"native admission setup NOT_RED {child.stderr}")
        self.assertFalse((attempt/"complete.json").exists(), "expired native phase still completed real database work")
        self.assertFalse((attempt/"query.sqlite").exists())
        self.assertFalse((attempt/"queries.jsonl").exists())
        self.assertEqual(json.loads((attempt/"budget.json").read_text())["disposition"], "deadline_refused")

    def test_each_absolute_native_bound_refuses_actual_d_database_work(self):
        for bound in ["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"]:
            with self.subTest(bound=bound):
                self.check_native_admission(bound)

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.sampled_from(["shared_deadline_ns", "work_deadline_ns", "hard_deadline_ns"]))
    def test_expired_native_admission_keeps_real_store_and_query_outputs_absent(self, bound):
        self.check_native_admission(bound)


class CellBudgetCampaignTests(unittest.TestCase):
    def check_cleanup_reserve(self, seconds):
        root, binary, manifest = clock_fixture()
        prepare_actual_database(root, binary, "real-preparation-control")
        manifest["cells"] = manifest["cells"][:1]
        manifest["cell_seconds"] = seconds
        preparations = []
        result = run_manifest(manifest, root/"results", before=lambda: preparations.append(True))
        self.assertEqual(result[0]["status"], "not_run_resource", "cell cleanup reserve was consumed as work time")
        self.assertEqual(result[0]["reason"], "cell_work_deadline")
        self.assertEqual(preparations, [])
        self.assertEqual(list((root/"results").glob("*/attempt-*")), [])

    def test_cleanup_reserve_prevents_actual_native_launch(self):
        self.check_cleanup_reserve(30)

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=1, max_value=30))
    def test_no_positive_work_allowance_cannot_launch_a_database_child(self, seconds):
        self.check_cleanup_reserve(seconds)

    def test_work_deadline_reaches_actual_native_config_and_owned_child_supervision(self):
        root, _, manifest = clock_fixture()
        manifest["cells"] = manifest["cells"][:1]
        manifest["cells"][0]["samples"] = 3000
        healthy = run_manifest(manifest, root/"healthy")
        self.assertEqual(healthy[0]["status"], "complete", "real D workload control NOT_RED")
        self.assertEqual(len((Path(healthy[0]["attempt"])/"queries.jsonl").read_text().splitlines()), 3001)
        manifest["cell_seconds"] = 30.02
        started = time.monotonic_ns()
        outcomes = run_manifest(manifest, root/"bounded")
        elapsed = time.monotonic_ns()-started
        print(f"actual bounded D attempt elapsed_ns={elapsed}: {outcomes}", flush=True)
        self.assertEqual(outcomes[0]["status"], "failed", "actual native work passed beyond the reserved work endpoint")
        outcome = outcomes[0]
        config = json.loads((Path(outcome["attempt"])/"cell.json").read_text())
        self.assertLessEqual(config["work_deadline_ns"], started+25_000_000)
        self.assertEqual(config["hard_deadline_ns"]-config["work_deadline_ns"],30_000_000_000)
        self.assertLessEqual(config["hard_deadline_ns"],config["shared_deadline_ns"])
        self.assertEqual(outcome["pgid"],outcome["pid"])
        self.assertTrue(outcome["cleanup"]["complete"])
        self.assertFalse(Path(f"/proc/{outcome['pid']}").exists())
        self.assertLess(elapsed,2_000_000_000)


class NativePhaseStopTests(unittest.TestCase):
    def check_stop(self, samples):
        root, _, manifest = clock_fixture()
        manifest["cells"] = manifest["cells"][:1]
        healthy = run_manifest(manifest, root/"healthy")
        self.assertEqual(healthy[0]["status"],"complete", "ordinary real D calls NOT_RED")
        manifest["cells"][0]["samples"] = samples
        manifest["cell_seconds"] = 30.02
        result = run_manifest(manifest, root/"bounded")[0]
        self.assertEqual(result["status"],"failed", "controller deadline control NOT_RED")
        attempt = Path(result["attempt"])
        print(f"actual native stop fixture {attempt}", flush=True)
        self.assertFalse((attempt/"complete.json").exists(), "native caller continued later work after refused/expired phase")
        report = json.loads((attempt/"budget.json").read_text())
        self.assertEqual(report["disposition"], "deadline_refused")
        self.assertIn(report["phase"],["entry","configure","drain","warmup","timed","write","collect","complete"])
        path = attempt/"queries.jsonl"
        calls = [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []
        self.assertLess(len(calls),samples+1)
        self.assertTrue(all(call["ok"] for call in calls))
        self.assertTrue(result["cleanup"]["complete"])

    def test_real_native_does_not_publish_completion_after_its_phase_budget(self):
        self.check_stop(3000)

    @settings(max_examples=2, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=2000,max_value=3000))
    def test_expired_native_preserves_only_the_completed_call_prefix(self,samples):
        self.check_stop(samples)


class PrelaunchDeadlineTests(unittest.TestCase):
    def check_prelaunch(self, bound, deadline):
        root, _, manifest = clock_fixture()
        manifest["cells"] = manifest["cells"][:1]
        healthy = run_manifest(manifest,root/"healthy")
        self.assertEqual(healthy[0]["status"],"complete", "ordinary real caller NOT_RED")
        cell = manifest["cells"][0]
        cell.update(work_deadline_ns=2**64-1,hard_deadline_ns=2**64-1,shared_deadline_ns=2**64-1)
        cell[bound] = deadline
        outcome = run_manifest(manifest,root/"expired")[0]
        print(f"actual prelaunch {bound}: pid={outcome['pid']} status={outcome['status']}",flush=True)
        self.assertIsNone(outcome["pid"], "controller launched a native child despite its actual tightened absolute bound")
        self.assertEqual(outcome["status"],"not_run_resource")
        self.assertEqual(outcome["reason"],"native_prelaunch_deadline")
        attempt = Path(outcome["attempt"])
        config = json.loads((attempt/"cell.json").read_text())
        self.assertEqual(config[bound],deadline)
        self.assertFalse((attempt/"query.sqlite").exists())
        self.assertFalse((attempt/"budget.json").exists(), "no fabricated native observation when no child ran")
        self.assertFalse((attempt/"child.log").exists())

    def test_each_tightened_actual_bound_is_rechecked_before_popen(self):
        for bound in ["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"]:
            with self.subTest(bound=bound):
                self.check_prelaunch(bound,0)

    @settings(max_examples=3,deadline=None,database=None,phases=[Phase.generate])
    @given(st.sampled_from(["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"]),st.integers(min_value=0,max_value=1000))
    def test_expired_actual_config_cannot_start_a_child(self,bound,deadline):
        self.check_prelaunch(bound,deadline)


def actual_resources():
    task = Path(os.environ["S30_GD_TASK"])
    proposal = json.loads((task.parent/"parent/slice30-resumption-resource-proposal.json").read_text())
    roots = [row["path"] for row in proposal["extended_roots"]]
    assert len(roots) == 13
    return {"roots":roots,"task":str(task),"baseline":69280403456}


class ResourceAdmissionCampaignTests(unittest.TestCase):
    def test_real_thirteen_root_admission_precedes_actual_preparation_and_native_calls(self):
        root, binary, manifest = clock_fixture()
        manifest["cells"] = manifest["cells"][:1]
        resources = actual_resources()
        preparation = []
        def before():
            preparation.append(time.monotonic_ns())
            prepare_actual_database(root,binary,"resource-admitted-preparation")
            return {"actual_preparation":True}
        outcome = run_manifest(manifest,root/"results",before=before,resources=resources)[0]
        self.assertEqual(outcome["status"],"complete", "ordinary actual D caller NOT_RED")
        self.assertTrue((Path(outcome["attempt"])/"query.sqlite").exists())
        environment = outcome["environment_before"] or {}
        samples = environment.get("resource_samples",[])
        print(f"actual resource admission samples={len(samples)}",flush=True)
        self.assertEqual(len(samples),2,"missing actual resource admission before preparation and before native launch")
        self.assertEqual(environment["preparation"],{"actual_preparation":True})
        self.assertLessEqual(samples[0]["observed_ns"],preparation[0])
        self.assertGreaterEqual(samples[1]["observed_ns"],preparation[0])
        for sample in samples:
            self.assertEqual([row["path"] for row in sample["roots"]],resources["roots"])
            self.assertGreater(sample["owned"],69280403456)
            self.assertGreaterEqual(sample["memory_available"],16*1024**3)
            self.assertEqual(sample["baseline"],69280403456)

    def test_unavailable_required_size_accounting_refuses_real_child_and_preparation(self):
        root, binary, manifest = clock_fixture()
        manifest["cells"] = manifest["cells"][:1]
        prepare_actual_database(root,binary,"ordinary-real-control")
        resources = actual_resources()
        resources["roots"] = [*resources["roots"][:-1],str(root/"unavailable-owned-resource")]
        preparations = []
        outcome = run_manifest(manifest,root/"results",before=lambda:preparations.append(True),resources=resources)[0]
        self.assertEqual(outcome["status"],"not_run_resource","required accounting failure was ignored")
        self.assertTrue(outcome["reason"].startswith("resource_observation_unavailable"))
        self.assertIn("unavailable-owned-resource",outcome["reason"])
        self.assertIsNone(outcome["pid"])
        self.assertEqual(preparations,[])
        self.assertEqual(list((root/"results").glob("*/attempt-*")),[])


if __name__ == "__main__":
    unittest.main()
