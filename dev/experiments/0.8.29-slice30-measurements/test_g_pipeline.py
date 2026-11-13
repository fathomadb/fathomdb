"""Actual prepared-job controls; no qualified frozen model/native inputs are simulated."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_runner import run_prepared


def controlled_manifest(root):
    """Prepare only harmless commands which production qualification must reject."""
    calls = root / "native-control-calls.jsonl"
    code = "\n".join([
        "import json,pathlib,sys",
        "config=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
        f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(config['family']+'\\n')",
        "assert not out.exists(); out.mkdir(parents=True)",
        "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
    ])
    command = [sys.executable, "-c", code]
    return {"commands": {"ordinary": command, "diagnostic": command},
            "corpus": "registered-corpus.jsonl", "queries": "registered-queries.jsonl",
            "allocation": str(root)}, calls


class GPreparedPipelineTests(unittest.TestCase):
    def test_unqualified_g0_retains_all_obligations_without_starting_later_jobs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, calls = controlled_manifest(root)
            outcomes = run_prepared(manifest, root / "g-run")
            self.assertEqual(calls.read_text().splitlines(), ["G-seed", "G-R"])
            self.assertEqual(outcomes[0]["status"], "failed")
            self.assertEqual(len(list((root / "g-run" / "results").glob("*/attempt-*"))), 1,
                             "later jobs started after G0 lacked positive Default qualification")
            self.assertEqual(len(outcomes), 84)
            self.assertTrue(all(row["status"] == "not_run_prerequisite" and row["complete"] is False
                                and row["reason"] == "g0_not_qualified" for row in outcomes[1:]))

    def test_prepared_tranche_cannot_renew_exhausted_shared_allocation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, calls = controlled_manifest(root)
            (root / "g-budget.json").write_text(json.dumps({"schema":"slice30-gd-budget/v1",
                                                           "used_ns":14400 * 10**9,"inflight":None}))
            outcomes = run_prepared(manifest, root / "fresh-g-tranche")
            self.assertFalse(calls.exists(), "new G result directory renewed the exhausted shared allocation")
            self.assertEqual(len(outcomes), 84)
            self.assertTrue(all(row["status"] == "not_run_cap" and row["reason"] == "shared_time_exhausted"
                                for row in outcomes))


if __name__ == "__main__":
    unittest.main()
