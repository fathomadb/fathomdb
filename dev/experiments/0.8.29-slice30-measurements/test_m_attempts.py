"""Owned tiny stdlib-SQLite copies prove attempt isolation, not product behavior."""

from contextlib import closing
import json
from pathlib import Path
import shutil
import sqlite3
import sys
import tempfile
import unittest

from m_controller import load_plan, run_budgeted_table


class AttemptIsolationTests(unittest.TestCase):
    """Failed attempts remain retained and completed cells are not replayed."""

    def test_prepare_fresh_closed_copy_charge_and_resume_without_replaying_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "closed.sqlite"
            with closing(sqlite3.connect(source)) as db:
                db.execute("CREATE TABLE notes(body TEXT)")
                db.execute("INSERT INTO notes VALUES ('alpha')")
                db.commit()
            allocation = root / "allocation"
            allocation.mkdir()
            ledger_path = allocation / "g-budget.json"
            ledger_path.write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": 123456789, "inflight": None, "cell_used_ns": {}}))
            profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
                "ledger": str(ledger_path), "total_shared_ns": 10 * 10**9,
                "total_shared_retained_bytes": 1048576, "whole_cell_ns": 900 * 10**9}
            plan = load_plan()[:2]
            forecasts = {cell["id"]: {"components_ns": {
                "copy": 1000000, "packaging": 0, "open": 0, "first": 0,
                "warmups": 0, "timed": 10000000, "logging": 0, "persistence": 0,
                "polling": 0, "idle": 0, "close": 0, "diagnostics": 0, "failed_attempts": 0},
                "new_retained_bytes": 65536} for cell in plan}
            calls, failed = root / "calls.jsonl", root / "failed-once"
            child = "\n".join([
                "import json,pathlib,sqlite3,sys",
                "cell=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                "out=pathlib.Path(sys.argv[2]); subject=out/'subject.sqlite'",
                "before=None",
                "if subject.exists():",
                "    db=sqlite3.connect(subject)",
                "    before=[r[0] for r in db.execute('SELECT body FROM notes')]",
                "    db.execute(\"INSERT INTO notes VALUES ('attempt')\");db.commit();db.close()",
                f"with open({str(calls)!r},'a') as stream:",
                "    stream.write(json.dumps({'id':cell['id'],'before':before,'attempt':str(out)})+'\\n')",
                f"flag=pathlib.Path({str(failed)!r})",
                "if cell['id']=='M0-structured' and not flag.exists():",
                "    flag.write_text('retained failure');sys.exit(7)",
                "(out/'result.json').write_text(json.dumps({'complete':before==['alpha']}))",
            ])

            def prepare(cell, attempt, deadline_ns):
                self.assertEqual(cell["family"], "M0")
                shutil.copyfile(source, attempt / "subject.sqlite")

            arguments = dict(profile=profile, allocation=allocation, forecasts=forecasts,
                             plan=plan, prepare=prepare)
            first = run_budgeted_table(allocation / "m-results", [sys.executable, "-c", child],
                                       **arguments)
            observed = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual([row["before"] for row in observed], [["alpha"], ["alpha"]],
                             "actual children did not receive fresh closed owned copies")
            charged = json.loads(ledger_path.read_text())["used_ns"]
            self.assertGreater(charged, 123456789)
            self.assertEqual([row["status"] for row in first], ["complete", "failed"])
            self.assertEqual(first[1]["exit"], 7)
            second = run_budgeted_table(allocation / "m-results", [sys.executable, "-c", child],
                                        **arguments)
            observed = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual([row["id"] for row in observed],
                             ["M0-text", "M0-structured", "M0-structured"])
            self.assertEqual([row["before"] for row in observed], [["alpha"], ["alpha"], ["alpha"]])
            self.assertEqual(len({row["attempt"] for row in observed}), 3)
            self.assertEqual([row["status"] for row in second], ["complete", "complete"])
            final = json.loads(ledger_path.read_text())
            self.assertGreater(final["used_ns"], charged)
            self.assertIsNone(final["inflight"])
            self.assertTrue(all(value > 0 for value in final["cell_used_ns"].values()))
            self.assertTrue((Path(first[1]["attempt"]) / "child.log").exists())
            with closing(sqlite3.connect(source)) as db:
                self.assertEqual(db.execute("SELECT body FROM notes").fetchall(), [("alpha",)])


if __name__ == "__main__":
    unittest.main()
