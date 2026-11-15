"""Literal next-quartet review math and a harmless-process bridge; no D database."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

from d_driver import reviewed_quartet, run_supervised_stage


SECOND, MIB = 10**9, 1024**2
SPEC = json.loads(Path(__file__).with_name("d-cells.json").read_text())


def fixture(overlap_complete=False):
    """State hand-costed qualified-prefix unit data; these are not native records."""
    records = {}
    for cell in SPEC["cells"][:8 if overlap_complete else 4]:
        seconds = {("C", 0): 20, ("C", 1): 22, ("T", 0): 10, ("T", 1): 12}[(cell["layer"], cell["repeat"])]
        mib = {("C", 0): 40, ("C", 1): 42, ("T", 0): 20, ("T", 1): 22}[(cell["layer"], cell["repeat"])]
        records[cell["id"]] = {"status": "complete", "attempt_id": "literal-one",
            "closed": True, "error_count": 0, "physical_count": 10000,
            "observations_complete": True, "dropped_records": 0,
            "actual_overlap": cell["mode"] == "overlap", "elapsed_ns": seconds*SECOND,
            "peak_bytes": mib*MIB, "unit_diagnostic": True}
    budget = {"used_ns": 100*SECOND, "shared_ns": 14400*SECOND,
              "retained_bytes": 2*1024**3, "storage_limit": 16*1024**3,
              "cell_used_ns": {}, "common_used_ns": {}, "inflight": None}
    review = {"verdict": "ACCEPT", "reviewer": "literal-review", "subject": "unit-prefix",
              "n": 100000 if overlap_complete else 10000,
              "mode": "held" if overlap_complete else "overlap",
              "checkpoint": {"used_ns": 100*SECOND, "retained_bytes": 2*1024**3},
              "prefix_attempts": {key: value["attempt_id"] for key, value in records.items()},
              "bounds": {"C": {"time_ns": (440 if overlap_complete else 30)*SECOND,
                                  "active_bytes": (840 if overlap_complete else 60)*MIB},
                         "T": {"time_ns": (240 if overlap_complete else 20)*SECOND,
                                  "active_bytes": (440 if overlap_complete else 30)*MIB}}}
    return records, budget, review


class DContinuationTests(unittest.TestCase):
    def test_accepted_review_names_only_next10k_overlap_quartet_with_manual_bound(self):
        records, budget, review = fixture()
        decision = reviewed_quartet(SPEC, records, budget, review)
        self.assertEqual(decision["status"], "admit", "accepted next-quartet review never reached admission")
        self.assertEqual(decision["cells"], ["d-c-overlap-n10000-r0", "d-t-overlap-n10000-r0",
                                            "d-t-overlap-n10000-r1", "d-c-overlap-n10000-r1"])
        self.assertEqual(decision["quartet_time_ns"], 100*SECOND)
        self.assertEqual(decision["quartet_bytes"], 180*MIB)

    def test_100k_held_uses_existing_same_mode_conservative_costs(self):
        records, budget, review = fixture(overlap_complete=True)
        decision = reviewed_quartet(SPEC, records, budget, review)
        self.assertEqual(decision["status"], "admit", "reviewed100k same-mode continuation remains unwired")
        self.assertEqual(decision["cells"], ["d-c-held-n100000-r0", "d-t-held-n100000-r0",
                                            "d-t-held-n100000-r1", "d-c-held-n100000-r1"])
        self.assertEqual(decision["quartet_time_ns"], 1360*SECOND)
        self.assertEqual(decision["quartet_bytes"], 2560*MIB)

    def test_unknown_unqualified_skipped_or_unaffordable_review_stays_held(self):
        for change in ("absent", "unknown-cost", "skipped-stage", "changed-attempt", "unclosed",
                       "time", "storage", "above900", "smaller-than-lower-bound"):
            with self.subTest(change=change):
                records, budget, review = fixture(overlap_complete=change == "smaller-than-lower-bound")
                if change == "absent":
                    review = None
                elif change == "unknown-cost":
                    review["bounds"]["C"]["time_ns"] = None
                elif change == "skipped-stage":
                    review["n"] = 100000
                elif change == "changed-attempt":
                    records["d-c-held-n10000-r0"]["attempt_id"] = "another-attempt"
                elif change == "unclosed":
                    records["d-t-held-n10000-r0"]["closed"] = False
                elif change == "time":
                    budget["shared_ns"] = 150*SECOND
                elif change == "storage":
                    budget["storage_limit"] = budget["retained_bytes"]+100*MIB
                elif change == "above900":
                    review["bounds"]["C"]["time_ns"] = 900*SECOND
                else:
                    review["bounds"]["C"]["time_ns"] = 439*SECOND
                self.assertNotEqual(reviewed_quartet(SPEC, records, budget, review)["status"], "admit")

    def test_reviewed_one_quartet_reaches_owned_children_and_never_retries_failed_prefix(self):
        records, budget, review = fixture()
        prefix = copy.deepcopy(records)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            markers = root/"dispatches"
            code = "\n".join([
                "import json,pathlib,sys",
                "config=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                f"with pathlib.Path({str(markers)!r}).open('a') as stream: stream.write(config['id']+'\\n')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':False,'unit_diagnostic':True}))",
            ])
            command = [sys.executable, "-c", code]
            rows = run_supervised_stage(SPEC, records, budget, root/"attempts", command,
                                        initial=False, review=review)
            self.assertTrue(markers.exists(), "accepted continuation never connected owned dispatch")
            expected = ["d-c-overlap-n10000-r0", "d-t-overlap-n10000-r0",
                        "d-t-overlap-n10000-r1", "d-c-overlap-n10000-r1"]
            self.assertEqual(markers.read_text().splitlines(), expected)
            self.assertEqual([row["id"] for row in rows], [cell["id"] for cell in SPEC["cells"]])
            self.assertEqual([row["status"] for row in rows[:4]], ["complete"]*4)
            self.assertEqual([row["status"] for row in rows[4:8]], ["failed"]*4)
            self.assertTrue(all(row["status"] == "not_run_stage_review" for row in rows[8:]))
            self.assertEqual({key: records[key] for key in prefix}, prefix)
            first_charges = dict(budget["cell_used_ns"])
            run_supervised_stage(SPEC, records, budget, root/"attempts", command, initial=False, review=review)
            self.assertEqual(markers.read_text().splitlines(), expected)
            self.assertEqual(budget["cell_used_ns"], first_charges)
            self.assertIsNone(budget["inflight"])


if __name__ == "__main__":
    unittest.main()
