"""Freeze exact G input/build/cache identities without fetching or opening stores."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid

from g_campaign import make_g_plan

SOURCE = Path(__file__).resolve().parents[3]
PINS = {"corpus": "cc5930e525a2af2954353f92dacf720cc4b79f40dbb7965c44bb5c985b5cb481",
        "queries": "0eeb5582fc93bee7e2a494054a55d26bb0940d771083b726f999a9682e74dc16",
        "license": "d3c4b9aa54759df6ded337978a6f3b55b75615e5e4525c3b82d7e2627d4b9732"}
MODEL_FILES = {"config.json": (743, "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750"),
               "tokenizer.json": (711396, "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66"),
               "model.safetensors": (133466304, "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad")}
FEATURES = {"ordinary": ["embed-cuda"], "diagnostic": ["embed-cuda", "slice30-observer"]}


def git(*args):
    """Read source identities from the owned checkout only."""
    return subprocess.check_output(["git", *args], cwd=SOURCE, text=True).strip()


def compiler_inputs(sha="HEAD"):
    """Identify Rust/native source inputs independently of metadata-only commit changes."""
    records = []
    for line in git("ls-tree", "-r", sha).splitlines():
        metadata, path = line.split("\t", 1)
        if path.startswith(("src/rust/", ".cargo/")) or path in (
                "Cargo.toml", "Cargo.lock", "src/python/pyproject.toml", "rust-toolchain", "rust-toolchain.toml"):
            records.append([path, metadata.split()[2]])
    return hashlib.sha256(json.dumps(records, separators=(",", ":")).encode()).hexdigest()


def identify(path):
    """Identify actual local file bytes using bounded reads."""
    path = Path(path).resolve(strict=True)
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest.hexdigest()}


def build_tools(output):
    """Build/stash exact ordinary and diagnostic tools outside any measurement interval."""
    if git("status", "--porcelain"):
        raise ValueError("G build requires a clean owned source checkout")
    if str(Path("/usr/local/cuda").resolve()) != "/usr/local/cuda-12.6":
        raise ValueError("G build requires unchanged primaryCUDA12.6")
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ, CUDA_COMPUTE_CAP="86", CUDA_HOME="/usr/local/cuda-12.6")
    receipt = {"schema": "slice30-g-build/v1", "source_sha": git("rev-parse", "HEAD"),
               "compiler_inputs": compiler_inputs(), "lock": identify(SOURCE / "Cargo.lock"),
               "primary_cuda": "/usr/local/cuda-12.6", "cuda_compute_cap": "86", "binaries": {}}
    for variant, features in FEATURES.items():
        command = ["cargo", "build", "--locked", "--release", "-p", "fathomdb-sdk", "--features",
                   ",".join(features), "--example", "slice30_measure"]
        subprocess.run(command, cwd=SOURCE, env=environment, check=True)
        built = Path(environment.get("CARGO_TARGET_DIR", str(SOURCE / "target"))) / "release/examples/slice30_measure"
        selected = output / f"slice30-measure-{variant}"
        shutil.copy2(built, selected)
        selected.chmod(0o555)
        receipt["binaries"][variant] = dict(identify(selected), features=features, build_command=command)
    path = output / "build-receipt.json"
    path.write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def freeze(options):
    """Refuse nonregistered bytes, then freeze exact local inputs/build/device prerequisites."""
    spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
    inputs = {}
    for key, expected in PINS.items():
        inputs[key] = identify(options[key])
        if inputs[key]["sha256"] != expected:
            raise ValueError(f"registered {key} mismatch: expected {expected}")
    counts = {}
    for key in ("corpus", "queries"):
        with Path(inputs[key]["path"]).open() as stream:
            counts[key] = sum(1 for _ in stream)
    if counts != {"corpus": 100000, "queries": 500}:
        raise ValueError("registered input counts differ from100k/500")
    if type(options["gpu_ordinal"]) is not int or options["gpu_ordinal"] < 0:
        raise ValueError("G requires a nonnegative selected CUDA ordinal")
    cache = {name: identify(Path(options["model_cache"]) / name) for name in MODEL_FILES}
    for name, (size, digest) in MODEL_FILES.items():
        if cache[name]["bytes"] != size or cache[name]["sha256"] != digest:
            raise ValueError(f"registered BGE cache mismatch: {name}")
    expected_cache = Path(os.environ["HF_HOME"]) / "hub/models--BAAI--bge-small-en-v1.5/snapshots" / spec["model"]["revision"]
    if expected_cache.resolve() != Path(options["model_cache"]).resolve():
        raise ValueError("selected BGE snapshot is not the active owned HF_HOME path")
    if git("status", "--porcelain"):
        raise ValueError("G freeze requires clean tested source")
    build = json.loads(Path(options["build_receipt"]).read_text())
    if (build.get("schema") != "slice30-g-build/v1" or build.get("compiler_inputs") != compiler_inputs()
            or build.get("primary_cuda") != "/usr/local/cuda-12.6"):
        raise ValueError("G tool build/source identity mismatch")
    binaries = {}
    for variant in FEATURES:
        actual = identify(options[variant])
        recorded = build["binaries"][variant]
        if actual != {key: recorded[key] for key in actual} or recorded["features"] != FEATURES[variant]:
            raise ValueError(f"G {variant} binary/features mismatch")
        binaries[variant] = actual
    python_receipt = SOURCE / ".cache/agent/python-native.json"
    native = json.loads(python_receipt.read_text())
    module = identify(native["native"]["module"])
    if (compiler_inputs(native["source_sha"]) != compiler_inputs()
            or native["source_delta_sha256"] != hashlib.sha256(b"").hexdigest()
            or module["sha256"] != native["native"]["sha256"]
            or not Path(module["path"]).is_relative_to(SOURCE / ".venv")):
        raise ValueError("owned noneditable Python/native source identity mismatch")
    from g_gpu import inspect_gpu
    gpu = inspect_gpu(options["gpu_uuid"])
    if gpu["applications"]:
        raise ValueError("reserved3090 has existing compute work")
    gpu["ordinal"] = options["gpu_ordinal"]
    allocation = Path(options["allocation"]).resolve()
    output = Path(options["out"]).resolve()
    if not output.is_relative_to(allocation):
        raise ValueError("G frozen root must retain artifacts inside its shared allocation")
    manifest = {"schema": "slice30-g-campaign/v1", "campaign_uuid": str(uuid.uuid4()),
                "source_sha": git("rev-parse", "HEAD"), "compiler_inputs": compiler_inputs(),
                "lock": identify(SOURCE / "Cargo.lock"), "spec": spec, "plan": make_g_plan(spec),
                "inputs": inputs, "model_files": cache, "binaries": binaries,
                "build_receipt": identify(options["build_receipt"]),
                "python_receipt": identify(python_receipt), "python_native": module,
                "allocation": str(allocation), "root": str(output), "gpu": gpu,
                "environment": {key: os.environ.get(key) for key in
                                ("HF_HOME", "XDG_CACHE_HOME", "CUDA_CACHE_PATH", "CUDA_VISIBLE_DEVICES")},
                "device_policy": f"cuda:{options['gpu_ordinal']}", "format": 37,
                "measurement_scope": "G only; no D/M/F/whole30 or release clearance"}
    output.mkdir(parents=True, exist_ok=False)
    for name, value in (("manifest.json", manifest), ("plan.json", manifest["plan"])):
        path = output / name
        path.write_text(json.dumps(value, indent=2) + "\n")
        path.chmod(0o444)
    return manifest


def validate_frozen(root):
    """Recheck actual frozen inputs/artifacts and source before any production job."""
    root = Path(root).resolve()
    manifest = json.loads((root / "manifest.json").read_text())
    if (manifest["schema"] != "slice30-g-campaign/v1" or manifest["source_sha"] != git("rev-parse", "HEAD")
            or git("status", "--porcelain") or manifest["compiler_inputs"] != compiler_inputs()
            or manifest["plan"] != make_g_plan(manifest["spec"])):
        raise ValueError("frozen G source/plan differs from the selected clean candidate")
    records = [*manifest["inputs"].values(), *manifest["model_files"].values(), *manifest["binaries"].values(),
               manifest["lock"], manifest["build_receipt"], manifest["python_receipt"], manifest["python_native"]]
    if any(identify(record["path"]) != record for record in records):
        raise ValueError("frozen G input/native/model bytes changed")
    if any(os.environ.get(key) != value for key, value in manifest["environment"].items()):
        raise ValueError("frozen G owned cache/device visibility environment changed")
    return manifest
