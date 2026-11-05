"""Run paired real-model open measurements and preserve every sample."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[3]
PROBE = ROOT / "src/rust/crates/fathomdb-engine/tests/slice50_fresh_open_cost.rs"


def capture(command):
    """Capture a read-only provenance command without raising on unavailable tools."""
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    return {"exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}


def describe(values):
    """Describe samples without interpolating beyond the observed distribution."""
    ordered = sorted(values)
    return {"n": len(values), "min": min(values), "median": statistics.median(values),
            "p95_nearest_rank": ordered[math.ceil(0.95 * len(ordered)) - 1], "max": max(values)}


def check_assets():
    """Require the existing default-loader cache to match the source's pinned files."""
    source = (ROOT / "src/rust/crates/fathomdb-embedder/src/loader.rs").read_text()
    def pin(name):
        match = re.search(r"const " + name + r': &str =\s*"([^\"]+)"', source)
        if match is None:
            raise ValueError(f"missing loader pin: {name}")
        return match.group(1)
    prefix = hashlib.sha256(f"{pin('HF_REPO')}@{pin('HF_REVISION')}".encode()).hexdigest()[:12]
    cache = Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "fathomdb/embedders" / prefix
    assets = []
    for name, constant in [("config.json", "CONFIG_JSON_SHA256"),
                           ("tokenizer.json", "TOKENIZER_JSON_SHA256"),
                           ("model.safetensors", "MODEL_SAFETENSORS_SHA256")]:
        path = cache / name
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != pin(constant):
            raise ValueError(f"cached asset differs from loader pin: {path}")
        assets.append({"file": name, "sha256": digest, "bytes": path.stat().st_size})
    return {"directory": str(cache), "files": assets}


def main():
    """Run strict devices separately, validate all cells, and save raw and parsed data."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--pairs", type=int, default=10)
    parser.add_argument("--devices", nargs="+", default=["cuda:0", "cpu"])
    args = parser.parse_args()
    if args.pairs < 2 or any(d != "cpu" and not d.startswith("cuda:") for d in args.devices):
        parser.error("at least two pairs and explicit cpu/cuda:N devices required")
    assets = check_assets()
    args.out.mkdir(parents=True, exist_ok=False)
    command = ["cargo", "test", "--offline", "--release", "-p", "fathomdb-engine",
               "--features", "test-hooks,embed-cuda", "--test", "slice50_fresh_open_cost",
               "--", "--ignored", "--exact", "fresh_open_cost", "--nocapture", "--test-threads=1"]
    result = {"started_utc": datetime.now(timezone.utc).isoformat(), "command": command,
              "git": capture(["git", "rev-parse", "HEAD"]),
              "status": capture(["git", "status", "--short"]), "host": platform.platform(),
              "cpu": next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                           if line.startswith("model name")), "unknown"),
              "rustc": capture(["rustc", "--version"]),
              "probe_sha256": hashlib.sha256(PROBE.read_bytes()).hexdigest(),
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "env": {k: os.environ.get(k) for k in ["CUDA_PATH", "CUDA_COMPUTE_CAP", "RAYON_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS", "OPENBLAS_NUM_THREADS", "CUDA_VISIBLE_DEVICES"]},
              "assets_before": assets, "devices": []}
    gpu_command = ["nvidia-smi", "--query-gpu=index,name,driver_version,memory.used,memory.free,utilization.gpu", "--format=csv,noheader"]
    for device in args.devices:
        env = os.environ.copy()
        env.pop("HF_TOKEN", None)
        env["FATHOMDB_EMBED_DEVICE"] = device
        env["FRESH_OPEN_PAIRS"] = str(args.pairs)
        env["FRESH_OPEN_ROWS"] = "300"
        label = device.replace(":", "-")
        entry = {"requested": device, "gpu_before": capture(gpu_command),
                 "load_before": Path("/proc/loadavg").read_text().strip()}
        raw_path = args.out / f"{label}.log"
        with raw_path.open("w") as raw:
            child = subprocess.run(command, cwd=ROOT, env=env, stdout=raw,
                                   stderr=subprocess.STDOUT, check=False)
        entry.update({"exit_code": child.returncode, "gpu_after": capture(gpu_command),
                      "load_after": Path("/proc/loadavg").read_text().strip(), "samples": []})
        for line in raw_path.read_text().splitlines():
            if "FRESH_OPEN_META " in line:
                entry["metadata"] = json.loads(line.split("FRESH_OPEN_META ", 1)[1])
            if "FRESH_OPEN_COST " in line:
                entry["samples"].append(json.loads(line.split("FRESH_OPEN_COST ", 1)[1]))
        result["devices"].append(entry)
        expected = {(mode, cell, pair) for mode in ["reuse", "reload"]
                    for cell in ["fresh", "cached"] for pair in range(args.pairs)}
        actual = {(r["lifecycle"], r["verification"], r["pair"]) for r in entry["samples"]}
        complete = child.returncode == 0 and len(entry["samples"]) == len(expected) and actual == expected
        entry["complete"] = complete
        if complete:
            assert entry["metadata"]["effective"] == device
            for sample in entry["samples"]:
                assert sample["probe_embeds"] == (45 if sample["verification"] == "fresh" else 0)
            entry["summary"] = []
            for mode in ["reuse", "reload"]:
                cells = {cell: {r["pair"]: r for r in entry["samples"]
                                if r["lifecycle"] == mode and r["verification"] == cell}
                         for cell in ["fresh", "cached"]}
                for metric in ["open_ms", "total_ms"]:
                    delta = [cells["fresh"][pair][metric] - cells["cached"][pair][metric]
                             for pair in range(args.pairs)]
                    entry["summary"].append({"lifecycle": mode, "metric": metric,
                        "fresh": describe([r[metric] for r in cells["fresh"].values()]),
                        "cached": describe([r[metric] for r in cells["cached"].values()]),
                        "paired_delta": describe(delta)})
        result["assets_after"] = check_assets()
        assert result["assets_after"] == assets
        result["ended_utc"] = datetime.now(timezone.utc).isoformat()
        (args.out / "results.json").write_text(json.dumps(result, indent=2) + "\n")
        print(f"{device}: complete={complete}, samples={len(entry['samples'])}", flush=True)
        if not complete:
            raise SystemExit(child.returncode or 1)
    for entry in result["devices"]:
        for row in entry["summary"]:
            print(entry["requested"], row["lifecycle"], row["metric"], row["paired_delta"], flush=True)


if __name__ == "__main__":
    main()
