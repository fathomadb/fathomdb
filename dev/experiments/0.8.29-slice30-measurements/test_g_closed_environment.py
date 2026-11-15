"""Closed-copy payload children inherit the validated device and offline policy."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
import g_runner

POLICY = {"FATHOMDB_EMBED_DEVICE": "cuda:0", "HF_HUB_OFFLINE": "1",
          "TRANSFORMERS_OFFLINE": "1"}


def _child_environment(marker):
    code = "\n".join([
        "import json,os,pathlib",
        f"pathlib.Path({str(marker)!r}).touch()",
        f"print(json.dumps({{key:os.environ.get(key) for key in {tuple(POLICY)!r}}}))",
    ])
    return json.loads(subprocess.check_output([sys.executable, "-c", code], text=True, timeout=5))


class GClosedEnvironmentTests(unittest.TestCase):
    def test_public_run_child_uses_prepared_policy_over_absent_or_conflicting_environment(self):
        for inherited in ({}, {"FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "0",
                               "TRANSFORMERS_OFFLINE": "0"}):
            with self.subTest(inherited=inherited), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                # Bootstrap is routing data; the supervised preflight supplies the policy.
                (root / "manifest.json").write_text(json.dumps({"preparation": "closed-copy-v1",
                    "allocation": str(root), "stage": "closed-copy-calibration", "device_policy": "cpu"}))
                prepared = {"device_policy": "cuda:0"}
                marker = root / "payload-child"
                events = []

                def common(_allocation, _mode, action, _options):
                    events.append(action)
                    return prepared if action == "preflight" else None

                def controller(manifest, frozen):
                    events.append("controller")
                    self.assertIs(manifest, prepared)
                    self.assertEqual(frozen, root)
                    return _child_environment(marker)

                with patch.dict(os.environ, dict(inherited, S30_TASK=str(root))):
                    for key in POLICY:
                        if key not in inherited:
                            os.environ.pop(key, None)
                    with patch("g_boundary.require_known_containment"), \
                            patch.object(g_closed_entry, "_call", common), \
                            patch.object(g_runner, "run_prepared", controller):
                        observed = g_runner.run_frozen(root)
                self.assertTrue(marker.exists())
                self.assertEqual(events, ["preflight", "controller", "finish"])
                self.assertEqual(observed, POLICY)

    def test_failed_preflight_never_launches_payload_child_or_overrides_environment(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "manifest.json").write_text(json.dumps({"preparation": "closed-copy-v1",
                "allocation": str(root), "stage": "closed-copy-calibration", "device_policy": "cuda:0"}))
            marker = root / "forbidden-payload-child"
            inherited = {"FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "0",
                         "TRANSFORMERS_OFFLINE": "0"}
            with patch.dict(os.environ, dict(inherited, S30_TASK=str(root))), \
                    patch("g_boundary.require_known_containment"), \
                    patch.object(g_closed_entry, "_call", side_effect=RuntimeError("preflight refused")) as common, \
                    patch.object(g_runner, "run_prepared", side_effect=lambda *_: _child_environment(marker)) as controller:
                with self.assertRaisesRegex(RuntimeError, "preflight refused"):
                    g_runner.run_frozen(root)
                self.assertEqual({key: os.environ.get(key) for key in POLICY}, inherited)
            common.assert_called_once_with(root, "closed-copy-calibration", "preflight", {"root": str(root)})
            controller.assert_not_called()
            self.assertFalse(marker.exists())
