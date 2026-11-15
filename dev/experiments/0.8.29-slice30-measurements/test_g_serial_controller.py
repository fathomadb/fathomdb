"""Real harmless children expose complete-table and post-quiescence accounting."""

from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_campaign import make_g_plan, run_g_plan
from test_g_closed_write_gate import setup
from test_g_serial_storage import model, SCHEMA

GiB = 1024**3


def prepared():
    """Build a fully ordered typed cost review, never an actual runtime approval."""
    spec, reused = setup()
    review = spec["_reviewed_costs"]
    review["limits"] = {"campaign_seconds": 43200, "active_bytes": 128*GiB}
    spec.update(_closed_limits=review["limits"], _closed_observation_wait_ms=50)
    storage = model()
    storage["basis"].update(r1=review["r1"], calibration=review["calibration"])
    review["storage_model"] = storage
    plan = make_g_plan(spec)
    review["g_table"] = {}
    for cell in plan:
        if cell["stage"] != "G-W":
            continue
        cohorts = (1000+cell["searches_per_batch"]-1)//cell["searches_per_batch"]
        rows = cohorts*cell["batch_size"]
        records = 1500+2*rows+cohorts
        fixed_output = 715223040+2*52764*records+26292
        closed = fixed_output+2*9831*rows
        live = fixed_output+2*238120*rows
        review["g_table"][cell["id"]] = {"cell": cell, "prediction": {
            "allowed": True, "total_ns": 1, "new_bytes": live,
            "storage": {"schema": SCHEMA, "closed_bytes": closed, "live_bytes": live,
                        "live_excess_bytes": live-closed}}}
    return spec, reused


def child(root, *, shared_growth=0, fail_first=False):
    first = "g-w-b1-s1-engine-text-r0"
    return [sys.executable, "-c", "\n".join([
        "import json,pathlib,sqlite3,sys",
        "cell=json.loads(pathlib.Path(sys.argv[1]).read_text());out=pathlib.Path(sys.argv[2])",
        f"root=pathlib.Path({str(root)!r})",
        "with (root/'calls').open('a') as f:f.write(cell['id']+'\\n')",
        "(out/'subject').mkdir()",
        "db=sqlite3.connect(out/'subject/store.sqlite');db.execute('create table t(x)');db.commit();db.close()",
        f"first=cell['id']=={first!r}",
        f"growth={shared_growth}",
        "if first and growth:\n with (root/'shared-state').open('wb') as f:f.truncate(growth)",
        f"failed=first and {fail_first!r}",
        "(out/'result.json').write_text(json.dumps({'complete':not failed,'runner_only':True}))",
        "sys.exit(int(failed))",
    ])]


class GSerialControllerTests(unittest.TestCase):
    def test_complete_remaining_table_refuses_before_individually_fitting_first_cell(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with (root/"prior-history").open("wb") as f:
                f.truncate(40*GiB)
            spec, reused = prepared()
            rows = run_g_plan(spec, root/"results", child(root), allocation_root=root, reused=reused)
            self.assertFalse((root/"calls").exists(),
                             "controller dispatched an individually fitting cell while its complete remaining table exceeded the cap")
            self.assertEqual(len(rows), 84)
            self.assertTrue(all(r["status"] == "not_run_cap" for r in rows if r["cell"]["stage"] == "G-W"))

    def test_shared_serialization_after_first_closure_is_charged_before_next_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            rows = run_g_plan(spec, root/"results", child(root, shared_growth=40*GiB),
                              allocation_root=root, reused=reused)
            self.assertEqual((root/"calls").read_text().splitlines(), ["g-w-b1-s1-engine-text-r0"],
                             "later whole-allocation growth was omitted from the next complete-table check")
            self.assertEqual(sum(r["status"] == "complete" for r in rows), 1)

    def test_closed_tuple_includes_final_controller_output_and_normal_absent_companions(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            original = deepcopy(reused)
            rows = run_g_plan(spec, root/"results", child(root), allocation_root=root, reused=reused)
            budget = json.loads((root/"g-budget.json").read_text())
            closures = budget.get("serial_closed_cases", {})
            self.assertEqual(len(closures), 72, "quiescent cases have no measured closed byte disposition")
            for row in rows:
                if row["id"] in original:
                    self.assertEqual(row, original[row["id"]])
                    continue
                witness = closures[row["id"]]
                attempt = Path(row["attempt"])
                self.assertEqual(witness["attempt"], str(attempt))
                self.assertEqual(witness["bytes"]["main"], 8192)
                self.assertEqual(witness["bytes"]["wal"], 0)
                self.assertEqual(witness["bytes"]["shm"], 0)
                total = sum(p.stat().st_size for p in attempt.rglob("*") if p.is_file())
                self.assertEqual(witness["bytes"]["total"], total)
                self.assertEqual(witness["bytes"]["output"], total-8192)
                self.assertGreaterEqual(witness["bytes"]["output"], (attempt/"controller-outcome.json").stat().st_size)
            self.assertIsNone(budget["inflight"])

    def test_known_closed_failure_stays_failed_and_is_never_redispatched_on_resume(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            rows = run_g_plan(spec, root/"results", child(root, fail_first=True), allocation_root=root, reused=reused)
            self.assertEqual(rows[12]["status"], "failed")
            run_g_plan(spec, root/"results", child(root, fail_first=True), allocation_root=root, reused=reused)
            self.assertEqual(len((root/"calls").read_text().splitlines()), 72,
                             "a terminal, known-closed failed case was automatically retried")
