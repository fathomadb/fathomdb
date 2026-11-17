"""Natural-write comparison controls; these fragments never qualify native work."""

import copy
import unittest
from unittest.mock import patch

from g_pair import compare_results
from g_pair_report import comparisons


def captured_observation_pair(purpose="ordinary"):
    """Embed observed sample9/batch29 values in independently authored unit payloads."""
    cells = [{"id": mode, "origin_id": "unit-natural-write", "purpose": purpose,
              "family": "G-W", "kind": "write", "provider": "default", "n": 100000,
              "adapters": ["engine"], "routes": ["text"], "samples": 1, "warmup": 1,
              "searches_per_batch": 1, "batch_size": 1, "repeat": 0,
              "slice50_pair_mode": mode, "observe": purpose == "diagnostic"}
             for mode in ("winner", "eager")]
    hit = {"write_cursor": 100010, "source_id": "source", "stored_body": "authored body",
           "score_bits": 7, "branch": "TextNode", "kind": "doc"}
    call = {"ok": True, "adapter": "engine", "route": "text", "query_index": 0,
            "sample_index": 0, "elapsed_ns": 10, "projection_cursor": 100010,
            "ids": ["l:s30doc-0"], "bodies": ["authored body"], "hits": [hit],
            "returned_hits": 1, "soft_fallback": "None", "degradations": "[]",
            "snippets": "None", "explanation": "None", "snapshot_observation": "unavailable"}
    readiness = {"available": True, "elapsed_ns": 1, "usable_embedder": True,
                 "state": "processing", "pending_count": 1,
                 "observation_scope": "adjacent public readiness; not query snapshot"}
    fixture = {"format": 37, "natural_records": 100000, "generation_id": "same-generation",
               "generation_failed": 0, "generation_pending": 0, "pending_count": 0,
               "readiness": "ready", "usable_embedder": True}
    left = {"native_member_qualified": True, "samples": [call],
            "warmups": [dict(call, projection_cursor=100000)], "first_calls": [],
            "batches": [{"before_sample": 0, "cohort_index": 0, "rows": 1,
                         "after_write_cursor": 100010, "affected_ids": ["s30doc-0"],
                         "revision_tokens": ["revision"], "adjacent_readiness": readiness}],
            "probes": [{"before_sample": 0, "cohort_index": 0, "logical_id": "s30doc-0",
                        "revision_token": "revision", "marker_current": True,
                        "point_probe": {"current": True}, "text_probe": copy.deepcopy(call)}],
            "before_workload": copy.deepcopy(fixture), "after_workload": copy.deepcopy(fixture)}
    right = copy.deepcopy(left)
    # Actual frozen TEXT-pair differences: samples/probes row9, readiness batch29.
    right["samples"][0]["projection_cursor"] = 100009
    right["probes"][0]["text_probe"]["projection_cursor"] = 100009
    right["batches"][0]["adjacent_readiness"].update(state="ready", pending_count=0)
    return cells, [left, right]


def compare(cells, values):
    rows = {cell["id"]: {"id": cell["id"], "status": "complete", "complete": True,
                         "cleanup_uncertain": False, "live_owned_pids": []} for cell in cells}
    with patch("g_pair_report._native", side_effect=lambda row: values[0 if row["id"] == "winner" else 1]):
        return comparisons(cells, rows)[0]


class NaturalObservationTests(unittest.TestCase):
    def test_captured_async_fragments_keep_full_inequality_and_report_result_parity(self):
        for purpose in ("ordinary", "diagnostic"):
            with self.subTest(purpose=purpose):
                cells, values = captured_observation_pair(purpose)
                self.assertFalse(compare_results(*values))
                actual = compare(cells, values)
                self.assertFalse(actual["equal"])
                self.assertTrue(actual["result_equal"])
                self.assertTrue(actual["complete"])
                self.assertEqual(actual["comparison_scope"], "natural_gw_results")
                self.assertEqual(actual["observation_issues"], {"winner": [], "eager": []})
                self.assertEqual(actual["observation_difference_counts"], {
                    "/samples/*/projection_cursor": 1,
                    "/probes/*/text_probe/projection_cursor": 1,
                    "/batches/*/adjacent_readiness/pending_count": 1,
                    "/batches/*/adjacent_readiness/state": 1})
                self.assertIn({"path": "/samples/0/projection_cursor", "winner": 100010,
                               "eager": 100009}, actual["observation_differences"])

    def test_strict_read_provider_none_and_unrelated_diagnostics_keep_cursor_equality(self):
        for family, kind, provider in (("G-R", "read", "default"), ("P2", "write", "none"),
                                       ("G-O", "read", "default")):
            cells, values = captured_observation_pair("diagnostic")
            for cell in cells:
                cell.update(family=family, kind=kind, provider=provider)
            actual = compare(cells, values)
            self.assertFalse(actual["equal"])
            self.assertFalse(actual["complete"])
            self.assertNotEqual(actual["comparison_scope"], "natural_gw_results")

    def test_other_results_effects_flags_and_unknown_fields_remain_exact(self):
        mutations = [
            lambda v: v["samples"][0]["hits"][0].update(write_cursor=99999),
            lambda v: v["samples"][0]["hits"][0].update(score_bits=8),
            lambda v: v["samples"][0]["hits"][0].update(source_id="other"),
            lambda v: v["samples"][0]["ids"].append("l:other"),
            lambda v: v["samples"][0]["bodies"].append("different"),
            lambda v: v["samples"][0].update(soft_fallback="fallback"),
            lambda v: v["samples"][0].update(degradations="[changed]"),
            lambda v: v["samples"][0].update(snippets="changed"),
            lambda v: v["samples"][0].update(route="hybrid"),
            lambda v: v["samples"][0].update(ok=False),
            lambda v: v["samples"][0].update(future_field=1),
            lambda v: v["probes"][0].update(marker_current=False),
            lambda v: v["batches"][0].update(after_write_cursor=100011),
            lambda v: v["after_workload"].update(natural_records=99999),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                cells, values = captured_observation_pair()
                mutate(values[1])
                self.assertFalse(compare(cells, values)["complete"])

    def test_missing_or_invalid_observations_cannot_be_ignored_even_if_equal(self):
        mutations = [
            lambda v: v["samples"][0].pop("projection_cursor"),
            lambda v: v["samples"][0].update(projection_cursor=True),
            lambda v: v["samples"][0].update(projection_cursor=100011),
            lambda v: v["samples"][0].update(projection_cursor=99999),
            lambda v: v["batches"][0].pop("adjacent_readiness"),
            lambda v: v["batches"][0]["adjacent_readiness"].update(available=False),
            lambda v: v["batches"][0]["adjacent_readiness"].update(pending_count=True),
            lambda v: v["batches"][0]["adjacent_readiness"].update(pending_count=-1),
            lambda v: v["batches"][0]["adjacent_readiness"].update(state="unknown"),
            lambda v: v["batches"][0]["adjacent_readiness"].update(state="ready", pending_count=1),
            lambda v: v["after_workload"].update(generation_id="changed"),
            lambda v: v["probes"][0].update(marker_current=False),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                cells, values = captured_observation_pair()
                mutate(values[0]); values[1] = copy.deepcopy(values[0])
                actual = compare(cells, values)
                self.assertFalse(actual["complete"])
                self.assertTrue(any(actual["observation_issues"].values()))

    def test_no_observed_pending_ceiling_or_adjacent_pending_cursor_linkage(self):
        cells, values = captured_observation_pair()
        values[0]["batches"][0]["adjacent_readiness"].update(state="processing", pending_count=100)
        values[1]["batches"][0]["adjacent_readiness"].update(state="processing", pending_count=900)
        actual = compare(cells, values)
        self.assertTrue(actual["complete"])
        self.assertFalse(actual["equal"])

    def test_declared_active_window_stops_after_a_semantic_pair_mismatch(self):
        import tempfile
        from pathlib import Path
        from g_pair import drive_members
        cells, values = captured_observation_pair()
        following = [dict(cell, id=cell["id"]+"-next", origin_id="next") for cell in cells]
        values[1]["samples"][0]["hits"][0]["score_bits"] = 8
        calls = []
        def execute(cell, index):
            calls.append(cell["id"])
            return {"id": cell["id"], "status": "complete", "complete": True,
                    "cleanup_uncertain": False, "live_owned_pids": []}
        with tempfile.TemporaryDirectory() as temporary, patch(
                "g_pair_report._native", side_effect=lambda row: values[0 if row["id"] == "winner" else 1]):
            drive_members({"scope": "primary", "_active_members": cells+following}, Path(temporary), execute)
        self.assertEqual(calls, ["winner", "eager"])

    def test_natural_parity_advances_credit_window_next_head_and_existing_retirement(self):
        import json
        import tempfile
        from pathlib import Path
        from g_pair import required_prefix
        from g_pair_report import published_report
        from g_pair_runtime import finish, retire_qualified
        from test_pair_public import PairIncrementalRetirementTests
        cells, values = captured_observation_pair()
        pair = compare(cells, values)
        fixture = PairIncrementalRetirementTests()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, all_rows, report = fixture.fixture(root)
            rows = all_rows[:2]
            for row, cell in zip(rows, cells):
                row.update(id=cell["id"], cell=cell, credited=pair["complete"])
            report.update(primary=rows, diagnostics=[], calibration=[],
                          comparisons={"primary": [pair], "diagnostics": [], "calibration": []})
            following = [dict(cell, id=cell["id"]+"-next", origin_id="next") for cell in cells]
            manifest.update(_active_members=cells, plan={"primary": cells+following, "diagnostics": []})
            manifest["primary_limits"]["pairs"] = 1
            # Retain the original EXIT1; its quiescence remains independently certified.
            original = fixture.final_guardian(root, rows[0]["guardian_owner"], exit=1)
            fixture.final_guardian(root, {"started_monotonic_ns": 3, "controller_pid": 4})
            with patch("g_pair.report", return_value=report):
                finish(manifest, rows)
            current = published_report(root)
            self.assertFalse(current["comparisons"]["primary"][0]["equal"])
            self.assertTrue(current["window_complete"])
            self.assertTrue(all(row["credited"] for row in current["primary"]))
            self.assertEqual([row["id"] for row in required_prefix(manifest, root)],
                             [row["id"] for row in following])
            budget = json.loads((root/"g-budget.json").read_text())
            budget["s50_guardian"] = None
            (root/"g-budget.json").write_text(json.dumps(budget))
            with patch("g_pair_report.published_report", return_value=current):
                retire_qualified(root, manifest=manifest)
            self.assertTrue(all(not Path(row["store"]).exists() for row in rows))
            retained = json.loads((root/"guardians/2/guardian.json").read_text())
            self.assertEqual(retained, original)

    def test_malformed_observation_containers_are_unavailable(self):
        for key, value in (("adjacent_readiness", "invalid"), ("text_probe", "invalid")):
            with self.subTest(key=key):
                cells, values = captured_observation_pair()
                stream = "batches" if key == "adjacent_readiness" else "probes"
                values[0][stream][0][key] = value
                values[1] = copy.deepcopy(values[0])
                self.assertFalse(compare(cells, values)["complete"])
