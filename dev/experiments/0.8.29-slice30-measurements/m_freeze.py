"""M-local immutable source/build/input/config qualification; no implicit allocation."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid

from g_freeze import MODEL_FILES, PINS, SOURCE, compiler_inputs, git, identify
from m_controller import COMPONENTS, _assessment, load_plan

VARIANTS = {"providerfree": [], "helper": ["test-seams"], "ordinary": ["embed-cuda"]}


def approved(profile, allocation):
    """Require an explicitly selected approved TOTAL profile on the original shared ledger."""
    if (profile.get("schema") != "slice30-m-budget/v1" or profile.get("status") != "APPROVED"
            or profile.get("whole_cell_ns") != 900 * 10**9
            or Path(profile["ledger"]).resolve() != Path(allocation).resolve() / "g-budget.json"
            or any(type(profile.get(key)) is not int or profile[key] <= 0
                   for key in ("total_shared_ns", "total_shared_retained_bytes"))):
        raise ValueError("selected approved shared profile required; M candidates remain inactive")
    if (any(type(profile.get(key)) is not int or profile[key] < 0
            for key in ("reserved_other_work_ns", "reserved_other_work_bytes"))
            or profile.get("other_work_cost_status") not in ("BOUNDED", "HELD_UNAVAILABLE")
            or not isinstance(profile.get("reservation_disposition"), str)
            or not profile["reservation_disposition"].strip()):
        raise ValueError("explicit reviewed other-work reservation/disposition required; unknown cost is not zero")


def build(output, variant):
    """Build one exact owned release artifact in a separate retained target, jobs2/locked."""
    from g_boundary import require_known_containment
    task = Path(os.environ["S30_TASK"]).resolve()
    require_known_containment(task)
    if git("status", "--porcelain"):
        raise ValueError("M build requires clean owned source")
    output = Path(output).resolve()
    if not output.is_relative_to(task) or not Path(os.environ["CARGO_HOME"]).resolve().is_relative_to(task):
        raise ValueError("M build output/Cargo cache must be owned by S30_TASK")
    output.mkdir(parents=True, exist_ok=False)
    target = output / "target"
    features = VARIANTS[variant]
    command = ["cargo", "build", "--locked", "--release", "--jobs", "2", "-p", "fathomdb-mcp",
               "--no-default-features", "--target-dir", str(target)]
    if features:
        command += ["--features", ",".join(features)]
    command += ["--example", "slice30_m_helper"] if variant == "helper" else ["--bin", "fathomdb-mcp"]
    environment = dict(os.environ, CARGO_BUILD_JOBS="2")
    if variant == "ordinary":
        if str(Path("/usr/local/cuda").resolve()) != "/usr/local/cuda-12.6":
            raise ValueError("unchanged primaryCUDA12.6 required")
        environment.update(CUDA_HOME="/usr/local/cuda-12.6", CUDA_COMPUTE_CAP="86")
    receipt = {"schema": "slice30-m-build/v1", "variant": variant, "features": features,
               "source_sha": git("rev-parse", "HEAD"), "compiler_inputs": compiler_inputs(),
               "lock": identify(SOURCE / "Cargo.lock"), "command": command}
    subprocess.run(command, cwd=SOURCE, env=environment, check=True)
    binary = target / ("release/examples/slice30_m_helper" if variant == "helper" else "release/fathomdb-mcp")
    selected = output / ("slice30_m_helper" if variant == "helper" else "fathomdb-mcp")
    shutil.copy2(binary, selected)
    selected.chmod(0o555)
    receipt["binary"] = identify(selected)
    (output / "build.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def freeze(options, output):
    """Pin selected closed packaging and complete inputs without opening or changing SQLite."""
    allocation = Path(options["allocation"]).resolve()
    profile = json.loads(Path(options["profile"]).read_text())
    approved(profile, allocation)
    if git("status", "--porcelain"):
        raise ValueError("M freeze requires clean reviewed source")
    output = Path(output).resolve()
    if not output.is_relative_to(allocation):
        raise ValueError("M freeze/results must belong to selected retained allocation")
    forecasts = json.loads(Path(options["forecasts"]).read_text())
    for cell in load_plan():
        value = forecasts["cells"][cell["id"]]
        if (set(value["components_ns"]) != set(COMPONENTS)
                or any(type(v) is not int or v < 0 for v in value["components_ns"].values())
                or type(value["new_retained_bytes"]) is not int or value["new_retained_bytes"] < 0):
            raise ValueError("complete bounded cell component forecasts required")
    common = forecasts["common_components_ns"]
    if set(common) != set(COMPONENTS) or any(type(v) is not int or v < 0 for v in common.values()):
        raise ValueError("complete bounded common component forecasts required")
    if sum(common.values()) <= 0:
        raise ValueError("common controller/qualification/polling/finalization forecast unavailable")
    common_bytes = forecasts["common_new_retained_bytes"]
    if type(common_bytes) is not int or common_bytes <= 0:
        raise ValueError("bounded remaining common freeze/report retained bytes required")
    ledger = json.loads((allocation / "g-budget.json").read_text())
    effective = dict(profile,
        total_shared_ns=profile["total_shared_ns"]-profile["reserved_other_work_ns"]-2*sum(common.values()),
        total_shared_retained_bytes=profile["total_shared_retained_bytes"]-profile["reserved_other_work_bytes"]-2*common_bytes)
    _assessment(load_plan(), effective, ledger, forecasts["cells"], allocation)
    return _bind_inputs(options, output, profile, forecasts)


def freeze_calibration(options, output, cell_id):
    """Bind only the selected calibration; primary46 admission remains unchanged."""
    from m_calibration import selected_cell
    allocation = Path(options["allocation"]).resolve()
    profile = json.loads(Path(options["profile"]).read_text())
    approved(profile, allocation)
    if git("status", "--porcelain"):
        raise ValueError("M calibration requires clean reviewed source")
    if not Path(output).resolve().is_relative_to(allocation):
        raise ValueError("M calibration belongs to the original retained allocation")
    return _bind_inputs(options, Path(output).resolve(), profile, None,
                        calibration=selected_cell(cell_id))


def _bind_inputs(options, output, profile, forecasts, *, calibration=None):
    allocation = Path(options["allocation"]).resolve()
    selected = json.loads((SOURCE / "dev/plans/release-state-0.8.29.json").read_text())
    # The release state owns this metadata's location; recursive lookup avoids another state view.
    def find(value):
        if isinstance(value, dict):
            if "g_selected_input_for_m" in value:
                return value["g_selected_input_for_m"]
            for nested in value.values():
                if found := find(nested):
                    return found
        if isinstance(value, list):
            for nested in value:
                if found := find(nested):
                    return found
        return None
    selected = find(selected)
    custody = options["closed_subject"]
    if (not selected or custody["owner_released_closed"] is not True
            or custody["source_sha"] != selected["source_sha"]
            or custody["campaign_uuid"] != selected["campaign_uuid"]
            or Path(custody["main"]).resolve() != Path(selected["store"]).resolve()
            or custody["schema_version"] != 37 or custody["readiness"] != "Ready"
            or custody["pending"] != 0
            or custody["counts"] != {"natural": 100000, "vectors": 100000,
                                     "physical": 100000, "spans": 100000}
            or custody["stored_coverage"] != {"full": 83644, "partial": 16356, "unknown": 0}):
        raise ValueError("released qualified closed100k subject/provenance facts required")
    main = Path(custody["main"]).resolve()
    custody = dict(custody, main=str(main))
    expected = {main, *(Path(str(main) + suffix) for suffix in ("-wal", "-shm")
                        if Path(str(main) + suffix).exists())}
    package = [identify(path) for path in custody["package"]]
    if {Path(value["path"]) for value in package} != expected:
        raise ValueError("closed-file packaging must include exact existing main/WAL/SHM companions")
    inputs = {key: identify(options[key]) for key in PINS}
    if any(inputs[key]["sha256"] != digest for key, digest in PINS.items()):
        raise ValueError("unchanged registered corpus/query/license bytes required")
    if sum(1 for _ in Path(inputs["queries"]["path"]).open()) != 500:
        raise ValueError("exact500 registered queries required")
    cache = Path(options["model_cache"]).resolve()
    task = Path(os.environ["S30_TASK"]).resolve()
    revision = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
    if (not Path(os.environ["HF_HOME"]).resolve().is_relative_to(task)
            or cache != (Path(os.environ["HF_HOME"]) / "hub/models--BAAI--bge-small-en-v1.5/snapshots" / revision).resolve()):
        raise ValueError("owned pinned local BGE snapshot required; no fetch")
    model = {name: identify(cache / name) for name in MODEL_FILES}
    if any((model[name]["bytes"], model[name]["sha256"]) != value for name, value in MODEL_FILES.items()):
        raise ValueError("registered model cache bytes required")
    binaries = {}
    for variant in (VARIANTS if calibration is None else ("ordinary",)):
        receipt = json.loads(Path(options["builds"][variant]).read_text())
        binary = identify(receipt["binary"]["path"])
        if (receipt.get("schema") != "slice30-m-build/v1" or receipt["variant"] != variant
                or receipt["features"] != VARIANTS[variant] or receipt["compiler_inputs"] != compiler_inputs()
                or binary != receipt["binary"] or not Path(binary["path"]).is_relative_to(task)):
            raise ValueError("owned separate locked build identity mismatch")
        binaries[variant] = receipt
    gpu = options["gpu"]
    if (type(gpu["ordinal"]) is not int or gpu["ordinal"] < 0 or not gpu["uuid"]
            or gpu["driver_version"] != "580.178.04" or gpu["primary_cuda"] != "/usr/local/cuda-12.6"):
        raise ValueError("explicit reserved3090/R580/CUDA12.6 identity required")
    manifest = {"schema": "slice30-m-freeze/v1", "uuid": str(uuid.uuid4()),
        "source_sha": git("rev-parse", "HEAD"), "compiler_inputs": compiler_inputs(),
        "allocation": str(allocation), "root": str(output), "task": str(task),
        "profile": profile, "forecasts": forecasts, "closed_subject": custody, "package": package,
        "inputs": inputs, "model": model, "model_cache": str(cache), "hf_home": os.environ["HF_HOME"],
        "gpu": gpu, "builds": binaries, "plan": load_plan(), "runtime_status": "HELD_UNTIL_SELECTED_RUN"}
    manifest["combined_feasibility"] = ("UNKNOWN" if profile["other_work_cost_status"] == "HELD_UNAVAILABLE"
                                         else "BOUNDED_BY_SELECTED_PROFILE")
    if calibration is not None:
        manifest.update(schema="slice30-m-calibration/v1", scope="bounded-natural-calibration",
                        plan=[calibration], calibration=options["calibration"],
                        qualifies_primary=False, runtime_status="BOUNDED_CALIBRATION_ONLY")
    output.mkdir(parents=True, exist_ok=True)
    path = output / "manifest.json"
    with path.open("x") as stream:
        stream.write(json.dumps(manifest, indent=2) + "\n")
    path.chmod(0o444)
    return manifest


def validate(root):
    """Require the frozen reviewed source and exact local inputs before any campaign child."""
    root = Path(root).resolve()
    value = json.loads((root / "manifest.json").read_text())
    approved(value["profile"], value["allocation"])
    if (value["schema"] != "slice30-m-freeze/v1" or value["root"] != str(root)
            or git("status", "--porcelain") or value["source_sha"] != git("rev-parse", "HEAD")
            or value["compiler_inputs"] != compiler_inputs() or value["plan"] != load_plan()):
        raise ValueError("frozen clean reviewed M source/inventory required")
    for original in [*value["package"], *value["inputs"].values(), *value["model"].values(),
                     *(receipt["binary"] for receipt in value["builds"].values())]:
        if identify(original["path"]) != original:
            raise ValueError("frozen local input/artifact identity differs")
    return value


def validate_calibration(root):
    """Require exact selected source/input/build identities without admitting primary rows."""
    from m_calibration import CELL_ID, selected_cell
    root = Path(root).resolve()
    value = json.loads((root / "manifest.json").read_text())
    approved(value["profile"], value["allocation"])
    if (value["schema"] != "slice30-m-calibration/v1" or value["scope"] != "bounded-natural-calibration"
            or value["root"] != str(root) or git("status", "--porcelain")
            or value["source_sha"] != git("rev-parse", "HEAD")
            or value["compiler_inputs"] != compiler_inputs() or value["plan"] != [selected_cell(CELL_ID)]):
        raise ValueError("exact frozen clean selected calibration required")
    for original in [*value["package"], *value["inputs"].values(), *value["model"].values(),
                     *(receipt["binary"] for receipt in value["builds"].values())]:
        if identify(original["path"]) != original:
            raise ValueError("frozen local calibration input/artifact identity differs")
    return value
