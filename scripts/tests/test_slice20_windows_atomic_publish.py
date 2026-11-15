"""Exercise replacement of an existing owned JSON file on actual Windows."""
import base64
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "dev/experiments/0.8.29-slice20-windows"
spec = importlib.util.spec_from_file_location("a6_publish_operator", PACKAGE / "a6_operator.py")
op = importlib.util.module_from_spec(spec)
spec.loader.exec_module(op)


@unittest.skipUnless(os.environ.get("FATHOM_WINDOWS_TESTS") == "1", "actual Windows opt-in")
class AtomicPublishTests(unittest.TestCase):
    def setUp(self):
        candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        cfg = op.venue_config(Path(os.environ["FATHOM_WINDOWS_VENUE"]), candidate, uuid.uuid4().hex)
        self.control = op.GUEST + "\\control\\" + cfg["runId"]
        encoded = base64.b64encode(json.dumps(cfg).encode()).decode()
        script = (PACKAGE / "common.ps1").read_text()
        script += "\n$c=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('" + encoded + "'))|ConvertFrom-Json;Assert-A6Start $c|Out-Null;"
        script += "[IO.Directory]::CreateDirectory(" + op.quote(self.control) + ")|Out-Null"
        op.command(script, timeout=20)

    def test_host_lease_replaces_existing_json_without_backup(self):
        session = op.Session.__new__(op.Session)
        session.control = self.control
        session.publish("lease.json", {"generation": 1})
        session.publish("lease.json", {"generation": 2})
        actual = json.loads(op.command("Get-Content -Raw -LiteralPath " + op.quote(self.control + "\\lease.json")))
        self.assertEqual(actual, {"generation": 2})

    def test_guest_json_replaces_existing_json_without_backup(self):
        path = op.quote(self.control + "\\guest.json")
        script = (PACKAGE / "common.ps1").read_text()
        script += "\nWrite-A6Json " + path + " @{generation=1};Write-A6Json " + path + " @{generation=2};Get-Content -Raw -LiteralPath " + path
        self.assertEqual(json.loads(op.command(script, timeout=20)), {"generation": 2})


if __name__ == "__main__":
    unittest.main()
