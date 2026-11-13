"""Freeze exact G input/build/cache identities without fetching or opening stores."""

import hashlib
import json
from pathlib import Path

from g_campaign import make_g_plan


def identify(path):
    """Identify actual local file bytes using bounded reads."""
    path = Path(path).resolve(strict=True)
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest.hexdigest()}


def freeze(options):
    """Freeze the selected current G table and actual local identities."""
    spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
    inputs = {key: identify(options[key]) for key in ("corpus", "queries", "license", "ordinary", "diagnostic", "build_receipt")}
    cache = {name: identify(Path(options["model_cache"]) / name)
             for name in ("config.json", "tokenizer.json", "model.safetensors")}
    manifest = {"schema": "slice30-g-campaign/v1", "spec": spec, "plan": make_g_plan(spec),
                "inputs": inputs, "model_files": cache, "allocation": str(Path(options["allocation"]).resolve())}
    output = Path(options["out"])
    output.mkdir(parents=True, exist_ok=False)
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest
