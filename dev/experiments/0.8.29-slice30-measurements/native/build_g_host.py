"""Build the private G host and retain its actual compiler feature/artifact identity."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def build(label, observer=False):
    """Build one locked release identity; refuse test-hook or TC5 feature contamination."""
    task = Path(os.environ["S30_GD_TASK"])
    feature = "slice30-g-observer" if observer else "slice30-g"
    log = task / "logs" / f"gd-g-host-{label}-compiler.jsonl"
    proof_path = task / "proofs" / f"gd-g-host-{label}.json"
    assert not log.exists() and not proof_path.exists(), "preserve prior compiler evidence"
    command = ["cargo", "build", "--locked", "--release", "-p", "fathomdb-tc5-benchmark",
               "--bin", "slice30-g-measurement", "--features", feature, "--message-format=json"]
    environment = dict(os.environ)
    environment.update(CUDA_HOME="/usr/local/cuda-12.6", CUDA_PATH="/usr/local/cuda-12.6")
    environment["PATH"] = "/usr/local/cuda-12.6/bin:" + environment["PATH"]
    identity = {"command": command, "cuda_home": environment["CUDA_HOME"],
                "nvcc": subprocess.check_output(["/usr/local/cuda-12.6/bin/nvcc", "--version"], text=True), "cwd": str(Path.cwd()),
                "source": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                "delta": subprocess.check_output(["git", "status", "--porcelain"], text=True),
                "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
                "cargo": subprocess.check_output(["cargo", "-V"], text=True),
                "lock_sha256": hashlib.sha256(Path("Cargo.lock").read_bytes()).hexdigest(),
                "root_manifest_sha256": hashlib.sha256(Path("Cargo.toml").read_bytes()).hexdigest()}
    with log.open("x") as output:
        result = subprocess.run(command, stdout=output, check=False, env=environment)
    rows = [json.loads(line) for line in log.read_text().splitlines() if line.strip()]
    for row in rows:
        if row.get("reason") == "compiler-message":
            print(row["message"]["rendered"], end="")
    if result.returncode:
        raise SystemExit(result.returncode)
    artifacts = [{key: row[key] for key in ["package_id", "target", "profile", "features", "filenames"]}
                 for row in rows if row.get("reason") == "compiler-artifact"]
    engine = [row for row in artifacts if row["target"]["name"] == "fathomdb_engine"]
    assert len(engine) == 1 and "embed-cuda" in engine[0]["features"]
    assert ("slice30-measurement" in engine[0]["features"]) == observer
    assert all(not any(feature == "test-hooks" or feature.startswith("tc5-benchmark")
                       for feature in row["features"]) for row in artifacts)
    binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/slice30-g-measurement"
    identity.update({"binary": str(binary), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                     "compiler_json_sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
                     "normal_gn": not observer, "observer": observer, "artifacts": artifacts})
    proof_path.write_text(json.dumps(identity, indent=2) + "\n")
    (task / "proofs/gd-g-host-feature-closure.json").write_bytes(proof_path.read_bytes())
    print(json.dumps({"proof": str(proof_path), "binary_sha256": identity["binary_sha256"],
                      "engine_features": engine[0]["features"]}), flush=True)


if __name__ == "__main__":
    build(sys.argv[1] if len(sys.argv) > 1 else str(time.time_ns()), observer=len(sys.argv) == 3 and sys.argv[2] == "observer")
