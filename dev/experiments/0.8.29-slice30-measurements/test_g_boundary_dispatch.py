"""Normal-entry containment refusal precedes actual controlled child dispatch."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from g_campaign import make_g_plan
from g_freeze import git
from g_runner import run_frozen
from test_g_boundary_entry import PHASE, unknown_boundary


class GUnknownBoundaryDispatchTests(unittest.TestCase):
    def test_unknown_cleanup_prevents_actual_downstream_child_from_normal_entry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with unknown_boundary(root) as original:
                frozen = root / "allocation/frozen"
                frozen.mkdir()
                marker = root / "actual-unsafe-entry-child"
                spec = json.loads(PHASE.with_name("g-cells.json").read_text())
                manifest = dict(schema="slice30-g-campaign/v1", source_sha=git("rev-parse", "HEAD"),
                                allocation=str(root / "allocation"), plan=make_g_plan(spec),
                                inputs={name: {"path": name} for name in ("corpus", "queries")},
                                binaries={}, gpu={"uuid": "GPU-protocol-control"}, device_policy="cuda:0")
                (frozen / "manifest.json").write_text(json.dumps(manifest))
                def downstream(_manifest, _root):
                    # An actual harmless child witnesses entry dispatch; no GPU prerequisite is qualified.
                    subprocess.run([sys.executable, "-c", f"import pathlib; pathlib.Path({str(marker)!r}).touch()"],
                                   check=True, timeout=3)
                    return []
                refusal = None
                with patch.dict(os.environ, S30_TASK=str(root)), \
                        patch("g_freeze.validate_frozen", return_value=manifest), \
                        patch("g_gpu.inspect_gpu", return_value={"applications": []}), \
                        patch("g_runner.run_prepared", side_effect=downstream):
                    try:
                        run_frozen(frozen)
                    except RuntimeError as error:
                        refusal = str(error)
                self.assertFalse(marker.exists(), "normal entry dispatched an actual child despite retained unknown cleanup")
                self.assertIn("containment", refusal or "")
                self.assertEqual((root / "g-containment-state.json").read_bytes(), original)
                self.assertEqual(json.loads((root / "allocation/g-budget.json").read_text())["used_ns"], 17)
