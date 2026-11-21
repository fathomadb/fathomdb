#!/usr/bin/env python3
"""Complete only the 23 unmeasured frozen queries under direct native custody."""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from experiments import tc5_gpu_v2 as tc5  # noqa: E402

OWNED = Path("/tmp/fathomdb-0830-slice70")


def bound_result(path, manifest, spec):
    """Reject a duplicate, reordered or drifted query result before aggregate reduction."""
    result = json.loads(path.read_text())
    expected = {key: manifest[key] for key in (
        "query_digest", "selection_digest", "fixture_digest", "index_digest",
        "build_identity", "runtime_identity", "seed_digest", "model_asset_digest",
    )}
    expected.update({key: spec[key] for key in ("workload", "algorithm", "rerank", "candidate_k", "top_k")})
    expected["resolved_settings_digest"] = spec["settings_digest"]
    expected["status"] = "measurement_complete"
    if any(result.get(key) != value for key, value in expected.items()):
        raise tc5.Tc5GpuV2Error(f"query result binding differs: {path.name}")
    return result


def run_guard(command, deadline):
    return subprocess.run(command, check=False).returncode


def main():
    """Preserve the failed cohort and assemble only independently bound real measurements."""
    deadline = time.monotonic() + 1800
    config_path = OWNED / "tc5-device-fix-full-config.json"
    config = tc5.load_config(config_path)
    binary = config.benchmark_binary
    tc5._validate_candidate_artifacts(config, binary=binary)
    inputs = tc5._load_arm_inputs(config, "bridge")
    assert len(inputs.documents) == 7667 and len(inputs.queries) == 100
    fresh = OWNED / "tc5-native-completion"
    fresh.mkdir(mode=0o700)
    snapshot = tc5._copy_prior_database(
        config, "bridge", inputs, OWNED / "tc5-final/bridge", fresh, binary,
        prior_binary=OWNED / "artifacts/fathomdb-tc5-benchmark",
    )
    index_digest, binary_digest = tc5._sha_file(snapshot), tc5._sha_file(binary)
    jobs = fresh / "query-jobs"
    jobs.mkdir(mode=0o700)
    results = []
    for ordinal, query in enumerate(inputs.queries, 1):
        stem = f"query-{ordinal:03d}"
        if ordinal <= 77:
            old = OWNED / "tc5-isolated-query-recovery/bridge/query-jobs"
            manifest_path = old / f"{stem}.manifest.json"
            database = old / stem / "fathomdb.sqlite"
        else:
            manifest_path = jobs / f"{stem}.manifest.json"
            directory = jobs / stem
            directory.mkdir(mode=0o700)
            database = directory / "fathomdb.sqlite"
            shutil.copyfile(snapshot, database)
            if tc5._sha_file(database) != index_digest:
                raise tc5.Tc5GpuV2Error("fresh query input differs from the immutable ingest")
        manifest, spec = tc5.build_query_job(
            config, query=query, database_path=database, document_count=7667,
            fixture_digest=inputs.fixture_digest, index_digest=index_digest,
            binary_digest=binary_digest, manifest_path=manifest_path,
        )
        spec_path = manifest_path.with_name(f"{stem}.spec.json")
        result_path = manifest_path.with_name(f"{stem}.result.json")
        if ordinal <= 77:
            if json.loads(manifest_path.read_text()) != manifest or json.loads(spec_path.read_text()) != spec:
                raise tc5.Tc5GpuV2Error("retained query inputs differ from the frozen ordinal")
        else:
            if time.monotonic() >= deadline:
                raise tc5.Tc5GpuV2Error("23-query recovery batch deadline reached")
            tc5._write_new_json(manifest_path, manifest)
            tc5._write_new_json(spec_path, spec)
            command = [
                sys.executable, str(ROOT / "dev/experiments/slice70-qualification/run_orin_tc5.py"),
                str(config_path), "--output", str(fresh / f"cell-{ordinal:03d}"),
                "--query-spec", str(spec_path), "--query-result", str(result_path),
                "--native-deadline", str(deadline),
            ]
            print(f"EXECUTE {stem} under direct native leader custody", flush=True)
            if run_guard(command, deadline) != 0:
                raise tc5.Tc5GpuV2Error(f"direct query guard failed: {stem}")
        results.append(bound_result(result_path, manifest, spec))
    receipt = tc5.aggregate_results(
        config, arm="bridge", results=results, fixture_digest=inputs.fixture_digest,
        index_digest=index_digest, binary_digest=binary_digest,
    )
    tc5._write_new_json(fresh / "tc5-gpu-arm-result.v2.json", receipt)
    print(json.dumps({"assembled_queries": len(results), "new_queries": 23, "metrics": receipt["metrics"]}), flush=True)
    if receipt["metrics"]["ci_95"][1] < 0.90:
        raise tc5.Tc5GpuV2Error("unchanged AC075 CI-high floor failed")


if __name__ == "__main__":
    main()
