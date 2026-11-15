"""Private Slice50 scope inside the existing P/G measurement entry."""

import json
import os
import shutil
import subprocess
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
CALIBRATION_ID = "S50-CAL:g-r-n10000-r0"
FEATURES = {"ordinary":["embed-cuda","slice50-measurement"],
            "diagnostic":["embed-cuda","slice50-measurement","slice30-observer"]}


def plan():
    """Derive exact search obligations without changing original P/G inventories."""
    from campaign import make_plan
    from g_campaign import make_g_plan
    bases = []
    for cell in make_plan(json.loads((TOOLS / "cells.json").read_text()), 2):
        bases.append(dict(cell, routes=["text"], provider="none", record_first_call=False,
                          kind="write" if cell["family"] == "P2" else "read"))
    for cell in make_g_plan(json.loads((TOOLS / "g-cells.json").read_text())):
        if cell["stage"] not in ("G-R", "G-W") or cell.get("routes") == ["embed"]:
            continue
        bases.append(dict(cell, family=cell["stage"],
                          routes=[route for route in cell["routes"] if route != "embed"]))
    diagnostic_ids = {"p1-n100000-r0", "p2-b10-s1-r0", "p2-b1-s100-r0", "g-r-n100000-r0"}
    diagnostic_ids.update(f"g-w-b{batch}-s{ratio}-{adapter}-{route}-r0"
                          for batch, ratio in ((10, 1), (1, 100))
                          for adapter in ("engine", "sdk") for route in ("text", "hybrid"))

    def members(cell, purpose, diagnostic=False):
        modes = ["winner", "eager"] if cell["repeat"] == 0 else ["eager", "winner"]
        return [dict(cell, id=f"s50-{cell['id']}-{purpose}-{mode}", origin_id=cell["id"],
                     purpose=purpose, slice50_pair_mode=mode, observe=diagnostic,
                     artifact="diagnostic" if diagnostic else "ordinary",
                     fresh_copy=True, fresh_seed=False, preparation="closed-copy-v1")
                for mode in modes]
    calibration = next(cell for cell in bases if cell["id"] == "g-r-n10000-r0")
    return {"primary": [item for cell in bases for item in members(cell, "ordinary")],
            "diagnostics": [item for cell in bases if cell["id"] in diagnostic_ids
                            for item in members(cell, "diagnostic", True)],
            "calibration": members(calibration, "calibration-ordinary")
                           + members(calibration, "calibration-diagnostic", True)}


def report(root):
    """Report every missing/failed/refused member without cross-crediting scopes."""
    root = Path(root)
    state_path = root / "results/campaign-state.json"
    previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    result = {"scope": "slice50-pair", "complete": False}
    for name, cells in plan().items():
        result[name] = [dict(previous.get(cell["id"], {}), id=cell["id"], cell=cell,
                             status=previous.get(cell["id"], {}).get("status", "not_run"))
                        for cell in cells]
    return result


def build_tools(output, *, _run=None):
    """Build both private artifacts; injected harmless builders remain source-only."""
    from g_freeze import SOURCE, compiler_inputs, git, identify
    if _run is None and git("status", "--porcelain"):
        raise ValueError("pair build requires clean current source")
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    receipt = {"schema":"slice50-pair-build/v1", "scope":"slice50-pair",
               "source_sha":git("rev-parse","HEAD"), "compiler_inputs":compiler_inputs(),
               "lock":identify(SOURCE / "Cargo.lock"), "runner_only":_run is not None,
               "binaries":{}}
    for variant, features in FEATURES.items():
        command = ["cargo","build","--offline","--locked","--release","--jobs","2",
                   "-p","fathomdb-sdk","--features",",".join(features),"--example","slice30_measure"]
        (_run or subprocess.run)(command, cwd=SOURCE, check=True)
        built = Path(os.environ.get("CARGO_TARGET_DIR",str(SOURCE/'target'))) / 'release/examples/slice30_measure'
        target = output / ('slice30-measure-'+variant)
        shutil.copy2(built,target)
        target.chmod(0o555)
        receipt['binaries'][variant] = dict(identify(target),features=list(features),command=command)
    (output/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    return receipt


def assemble_manifest(options, build, inputs, donors):
    """Bind validated current build/data to private canonical obligations."""
    actual_features = {name:list(build['binaries'][name]['features']) for name in FEATURES}
    if actual_features != FEATURES:
        raise ValueError("current private pair build/features required")
    return {"schema":"slice50-paired-campaign/v1", "scope":options.get('scope','primary'),
            "source_sha":build['source_sha'], "compiler_inputs":build['compiler_inputs'],
            "build":build, "features":actual_features, "inputs":inputs, "donors":donors,
            "plan":plan(), "allocation":options['allocation'],
            "profile":options['profile'], "retire_complete_copies":options.get('retire_complete_copies',False)}
