"""Manual SDK G2 read-only schedule reporting on the accepted closed fixture."""

import json
import unittest

import test_g_text as text


class ReadonlyScheduleTests(text.CoreTextTests):
    adapter="sdk"
    mode="sdk_text"
    samples=20
    extra_config={"schedule":"readonly"}

    def test_literal_text_calls_emit_their_population(self):
        root,proof,rows,issues=self.run_fixture()
        if issues or len(rows)!=20:
            raise RuntimeError("existing filtered natural API records did not complete")
        preparation=proof["marker_preparation"]
        expected_ids=[f"s30marker-{slot}" for slot in range(10)]
        expected_bodies=[f"s30markercurrent{slot} revisionreset" for slot in range(10)]
        if (preparation["reset_write_calls"],preparation["reset_rows"],preparation["doc_vectors"],preparation["note_vectors"])!=(1,10,2,10):
            raise RuntimeError("existing marker preparation/count control failed")
        if preparation["marker_ids"]!=expected_ids or preparation["marker_bodies"]!=expected_bodies:
            raise RuntimeError("literal reset marker revisions failed")
        if preparation["reset_write_ns"]<=0 or preparation["reset_drain_ns"]<=0:
            raise RuntimeError("separate reset timings absent")
        for index,row in enumerate(rows):
            if (row["sample_index"],row["ids"],row["kind_filter"],row["adapter"],row["mode"])!=(index,["l:alpha"],"doc","sdk","direct_text") or not row["ok"] or row["elapsed_ns"]<=0:
                raise RuntimeError("actual SDK natural doc-filter/literal/order control failed")
        for stream in ["batches","marker-probes"]:
            if (root/"out"/(stream+".jsonl")).stat().st_size:
                raise RuntimeError("read-only fixture performed scheduled write/probe work")
        expected={"mode":"readonly","natural_calls":20,"natural_kind_filter":"doc",
            "scheduled_write_calls":0,"scheduled_write_rows":0,"marker_probes":0}
        print(json.dumps({"existing_control_calls":len(rows),"expected_schedule":expected}),flush=True)
        self.assertEqual(proof.get("schedule"),expected,"missing explicit zero-write G control schedule report")


if __name__=="__main__":
    unittest.main()
