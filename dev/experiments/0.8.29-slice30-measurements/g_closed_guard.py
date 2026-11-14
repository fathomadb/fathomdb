"""Closed-profile guardian discovery; phase accounting integration is pending."""

import json
from pathlib import Path


def discover(command):
    """Recognize only normal G runner commands with the closed-copy profile."""
    runner = Path(__file__).with_name("g_runner.py").resolve()
    if len(command) < 3 or Path(command[1]).resolve() != runner:
        return None
    action = command[2]
    if action == "freeze":
        values = dict(zip(command[3::2], command[4::2], strict=True))
        if values.get("--preparation") != "closed-copy-v1":
            return None
        return {"allocation": values["--allocation"], "mode": values["--stage"],
                "root": values["--out"]}
    if action in ("run", "report"):
        root = Path(command[3]).resolve()
        manifest = json.loads((root / "manifest.json").read_text())
        if manifest.get("preparation") == "closed-copy-v1":
            return {"allocation": manifest["allocation"], "mode": manifest["stage"], "root": str(root)}
    return None
