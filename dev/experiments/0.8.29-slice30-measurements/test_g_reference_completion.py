"""Completion reporting preserves qualified references without implying new measurements."""

from contextlib import redirect_stdout
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import g_runner
from g_campaign import make_g_plan
from g_report import summarize
from test_g_closed_plan import ORIGINAL, SPEC


def report_protocol(root):
    plan = make_g_plan(dict(SPEC, preparation="closed-copy-v1"))
    (root / "manifest.json").write_text(json.dumps({"plan": plan}))
    actual = root / "original-result"
    actual.mkdir()
    (actual / "result.json").write_text(json.dumps({"complete": True, "qualification_issues": [], "steps": []}))
    reused = ORIGINAL | {"g-r-n100000-r1"}
    rows = [{"id": cell["id"], "cell": cell, "status": "reused_complete" if cell["id"] in reused else "complete",
             "attempt": str(actual), "origin": {"source_sha": "original-protocol-source", "campaign_uuid": "original-protocol-uuid"}}
            for cell in plan]
    (root / "results").mkdir()
    (root / "results/g-outcomes.json").write_text(json.dumps(rows))
    (root / "last-run.json").write_text(json.dumps({"environment_ok": True}))
    return rows


class GReferenceCompletionTests(unittest.TestCase):
    def test_report_counts_validated_original_references_as_completed_obligations(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = report_protocol(root)
            value = summarize(root)
            self.assertTrue(value["complete"], "12 validated references kept a fully completed protocol table incomplete")
            self.assertEqual(len(value["cells"]), 84)
            self.assertEqual(sum(row["status"] == "reused_complete" for row in value["cells"]), 12)
            self.assertEqual([row["outcome"] for row in value["cells"]], original)
            self.assertIn("no D/M/F/whole30", value["scope"])

    def test_normal_cli_completion_uses_reference_semantics_without_retagging(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows = report_protocol(root)
            stream = io.StringIO()
            failure = None
            with patch.object(g_runner, "run_frozen", return_value=rows), \
                    patch.dict("os.environ", AGENT_LONG="1"), \
                    patch("sys.argv", [str(Path(g_runner.__file__)), "run", str(root)]), redirect_stdout(stream):
                try:
                    g_runner.main()
                except SystemExit as error:
                    failure = error
            self.assertIsNone(failure, "normal CLI rejects completed original-reference outcomes")
            self.assertTrue(json.loads(stream.getvalue())["complete"])
            self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 12)
