"""Manual real SDK G2 actuation: post-group atomic writes, drain, literal probes."""

import json
import sqlite3
import unittest

from hypothesis import Phase, given, settings, strategies as st

import test_g_text as text
from test_g_mean import read_jsonl_records


class WriteScheduleTests(text.CoreTextTests):
    adapter="sdk"
    mode="sdk_text"
    samples=20
    extra_config={"schedule":"write"}

    def require_schedule(self, calls):
        self.samples=calls
        root,proof,natural,issues=self.run_fixture()
        if issues or len(natural)!=calls:
            raise RuntimeError("existing natural SDK calls did not complete")
        reset=proof["marker_preparation"]
        if (reset["reset_write_calls"],reset["reset_rows"],reset["doc_vectors"],reset["note_vectors"])!=(1,10,2,10):
            raise RuntimeError("existing reset/drain/per-kind controls failed")
        if reset["marker_ids"]!=[f"s30marker-{slot}" for slot in range(10)] or reset["marker_bodies"]!=[f"s30markercurrent{slot} revisionreset" for slot in range(10)]:
            raise RuntimeError("literal initial marker reset failed")
        for index,row in enumerate(natural):
            if (row["sample_index"],row["ids"],row["kind_filter"],row["adapter"],row["mode"],row["ok"])!=(index,["l:alpha"],"doc","sdk","direct_text",True):
                raise RuntimeError("actual natural IDs/filter/order failed")
            if not 0<row["record_start_ns"]<=row["record_ready_ns"] or row["elapsed_ns"]<=0:
                raise RuntimeError("natural API/record intervals invalid")
        phases=read_jsonl_records(root/"out/batches.jsonl",issues)
        probes=read_jsonl_records(root/"out/marker-probes.jsonl",issues)
        writes=[row for row in phases if row.get("event")=="governed_write"]
        cohorts=calls//10
        print(json.dumps({"root":str(root),"successful_natural_calls":len(natural),"actual_write_cohorts":len(writes),"expected_write_cohorts":cohorts,"actual_probe_rows":len(probes)}),flush=True)
        with sqlite3.connect((root/"out/store.sqlite").as_uri()+"?mode=ro",uri=True) as connection:
            actual=connection.execute("SELECT logical_id,body FROM canonical_nodes WHERE kind='note' AND state='active' AND superseded_at IS NULL ORDER BY logical_id").fetchall()
            counts=connection.execute("SELECT c.kind,count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows v ON v.kind=c.kind AND v.write_cursor=c.write_cursor JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=c.write_cursor WHERE c.state='active' AND c.superseded_at IS NULL GROUP BY c.kind ORDER BY c.kind").fetchall()
            retained_mapping_rows=connection.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0]
            if counts!=[("doc",2),("note",10)] or [row[0] for row in actual]!=[f"s30marker-{slot}" for slot in range(10)]:
                raise RuntimeError("independent current-cursor SQL fixture/control failed before actuation assertion")
        self.assertEqual(len(writes),cohorts,"missing actual governed G cohorts after completed natural groups")
        self.assertEqual(issues,[])
        self.assertEqual(len(phases),cohorts*3)
        self.assertEqual(len(probes),cohorts*10)
        expected_events=["governed_write","projection_drained","active_coverage"]*cohorts
        self.assertEqual([row["event"] for row in phases],expected_events)
        for end in range(len(phases)+1):
            prefix=phases[:end]
            events=[row["event"] for row in prefix]
            self.assertEqual(events,expected_events[:end])
            written=events.count("governed_write")
            drained=events.count("projection_drained")
            covered=events.count("active_coverage")
            self.assertTrue(covered<=drained<=written)
            self.assertLessEqual(written-covered,1)
            self.assertEqual(written==drained==covered==cohorts,end==len(phases),"valid incomplete prefix is not a complete pass")
        for cohort in range(cohorts):
            group_end=(cohort+1)*10
            write,drain,coverage=phases[cohort*3:cohort*3+3]
            self.assertEqual([row["event"] for row in (write,drain,coverage)],["governed_write","projection_drained","active_coverage"])
            self.assertEqual([row["after_natural_calls"] for row in (write,drain,coverage)],[group_end]*3)
            self.assertEqual([row["cohort"] for row in (write,drain,coverage)],[cohort]*3)
            self.assertEqual(write["rows"],10)
            self.assertGreater(write["cursor"],0)
            self.assertLessEqual(natural[group_end-1]["record_ready_ns"],write["start_ns"])
            self.assertLess(write["start_ns"],write["end_ns"])
            self.assertEqual(write["write_ns"],write["end_ns"]-write["start_ns"])
            self.assertLessEqual(write["end_ns"],drain["start_ns"])
            self.assertLess(drain["start_ns"],drain["end_ns"])
            self.assertEqual(drain["drain_ns"],drain["end_ns"]-drain["start_ns"])
            self.assertEqual(drain["pending_after_drain"],0)
            previous=drain["end_ns"]
            for slot,probe in enumerate(probes[cohort*10:cohort*10+10]):
                self.assertEqual((probe["cohort"],probe["after_natural_calls"],probe["slot"]),(cohort,group_end,slot))
                self.assertEqual(probe["ids"],[f"l:s30marker-{slot}"])
                self.assertEqual(probe["bodies"],[f"s30markercurrent{slot} revision{cohort}"])
                self.assertEqual((probe["ok"],probe["marker_current"]),(True,True))
                self.assertLessEqual(previous,probe["start_ns"])
                self.assertLess(probe["start_ns"],probe["end_ns"])
                self.assertEqual(probe["proof_ns"],probe["end_ns"]-probe["start_ns"])
                previous=probe["end_ns"]
            self.assertLessEqual(previous,coverage["start_ns"])
            self.assertLess(coverage["start_ns"],coverage["end_ns"])
            self.assertEqual(coverage["current_by_kind"],{"doc":2,"note":10})
            self.assertEqual(coverage["current_vector_cursors"],12)
            self.assertEqual(coverage["total_physical_vectors"],12+(cohort+1)*10)
            self.assertEqual(coverage["historical_physical_vectors"],(cohort+1)*10)
            if group_end<calls:
                self.assertLessEqual(coverage["end_ns"],natural[group_end]["record_start_ns"])
        self.assertEqual(actual,[(f"s30marker-{slot}",f"s30markercurrent{slot} revision{cohorts-1}") for slot in range(10)])
        self.assertEqual(retained_mapping_rows,12+cohorts*10)
        self.assertEqual(proof["schedule"],{"mode":"write","natural_calls":calls,"natural_kind_filter":"doc","scheduled_write_calls":cohorts,"scheduled_write_rows":cohorts*10,"marker_probes":cohorts*10})
        persistence=proof["raw_persistence"]
        self.assertEqual(persistence["records"],calls+cohorts*13)
        self.assertEqual((persistence["checkpoint_records"],persistence["checkpoints"]),(16,cohorts*5))
        self.assertEqual(persistence["encoded_bytes"],sum((root/"out"/(stream+".jsonl")).stat().st_size for stream in ["samples","warmups","batches","marker-probes","matches"]))
        self.assertGreaterEqual(persistence["persistence_ns"],persistence["checkpoint_ns"])
        self.assertGreater(persistence["checkpoint_ns"],0)
        measured=sum(row["elapsed_ns"] for row in natural)+sum(row["end_ns"]-row["start_ns"] for row in phases)+sum(row["proof_ns"] for row in probes)
        self.assertGreater(proof["loop_ns"],measured)

    def test_literal_text_calls_emit_their_population(self):
        self.require_schedule(20)

    @settings(max_examples=3,deadline=None,derandomize=True,phases=[Phase.generate])
    @given(cohorts=st.integers(min_value=1,max_value=3))
    def test_generated_completed_groups_preserve_count_order_and_current_revisions(self, cohorts):
        self.require_schedule(cohorts*10)


if __name__=="__main__":
    unittest.main()
