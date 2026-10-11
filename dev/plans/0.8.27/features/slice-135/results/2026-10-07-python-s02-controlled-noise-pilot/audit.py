"""Recompute the controlled Python S02 baseline pilot from retained raw blocks."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import runpy
import statistics
import sys
import zipfile


ROOT = Path(__file__).resolve().parent
INDEPENDENT_CHECK = runpy.run_path(
    str(ROOT.parent / "2026-10-07-python-s02-baseline-noise-pilot/audit.py")
)["check_raw"]
STABLE = ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler")
RESOURCE_KEYS = {
    "user_s": "user_cpu_s",
    "system_s": "system_cpu_s",
    "peak_rss_kib": "peak_rss_kib",
    "fs_inputs": "fs_inputs",
    "fs_outputs": "fs_outputs",
    "major_faults": "major_faults",
    "swap_events": "swap_events",
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_environment(environment: dict, resources: list[dict]) -> list[str]:
    """Reject drift and retain host paging without calling it child swap."""
    start, end = environment["start"], environment["end"]
    if any(not start.get(key) or start[key] != end.get(key) for key in STABLE):
        raise ValueError("host identity changed")
    if (
        start["governor"] == "unavailable"
        or start["competing_jobs"]
        or end["competing_jobs"]
    ):
        raise ValueError("governor or competing-job control failed")
    if min(start["disk_free_bytes"], end["disk_free_bytes"]) < 1_073_741_824:
        raise ValueError("disk pressure")
    if end["swap_pages"] < start["swap_pages"]:
        raise ValueError("host swap counter decreased")
    if any(resource["swap_events"] != 0 for resource in resources):
        raise ValueError("measured child swapped")
    if environment["invalidators"] != []:
        raise ValueError("producer environment invalidator retained")
    expected_warnings = []
    delta = end["swap_pages"] - start["swap_pages"]
    if delta:
        expected_warnings.append(
            f"host swap drift: {delta} pages; child swap events: 0"
        )
    for resource in resources:
        if resource["major_faults"]:
            expected_warnings.append(f"child major faults: {resource['major_faults']}")
    if sorted(set(expected_warnings)) != sorted(environment["warnings"]):
        raise ValueError("paging warning differs from counters")
    return expected_warnings


def check_resource(path: Path, recorded: dict) -> None:
    fields = dict(
        line.split("=", 1) for line in path.read_text().splitlines() if "=" in line
    )
    for field, key in RESOURCE_KEYS.items():
        value = (
            float(fields[field])
            if field in ("user_s", "system_s")
            else int(fields[field])
        )
        if recorded[key] != value:
            raise ValueError(f"resource {key} differs from GNU Time")
    if recorded["method"] != "gnu-time" or recorded["unsupported"] != []:
        raise ValueError("child resources incomplete")


def main() -> None:
    if len(sys.argv) != 6:
        raise SystemExit(
            "usage: audit.py WHEEL BLOCK_RUNNER TIMED_RUNNER S01_HELPER S02_HELPER"
        )
    wheel, block_runner, timed_runner, s01_helper, s02_helper = map(Path, sys.argv[1:])
    pilot = json.loads((ROOT / "pilot-manifest.json").read_text())
    if pilot["source_sha"] != "f99e002f0d2e4002f3694c9f8d4986b56089edaa":
        raise ValueError("baseline source changed")
    if sha(block_runner) != pilot["block_runner_sha256"]:
        raise ValueError("block runner changed")
    with zipfile.ZipFile(wheel) as archive:
        native_sha = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    blocks = []
    raw_hashes = {}
    first_raw = None
    for index in range(1, 6):
        directory = ROOT / f"block-{index:02d}"
        manifest = json.loads((directory / "manifest.json").read_text())
        if manifest["source_sha"] != pilot["source_sha"] or (
            manifest["wheel_sha256"],
            manifest["native_sha256"],
            manifest["runner_sha256"],
            manifest["s01_helper_sha256"],
            manifest["s02_helper_sha256"],
        ) != (
            sha(wheel),
            native_sha,
            sha(timed_runner),
            sha(s01_helper),
            sha(s02_helper),
        ):
            raise ValueError(f"block {index} artifact identity changed")
        for source in (timed_runner, s01_helper, s02_helper):
            if sha(directory / "bundle" / source.name) != sha(source):
                raise ValueError(f"block {index} bundle changed")
        environment_path = directory / "environment.json"
        attempts_path = directory / "attempts.json"
        summary = json.loads((directory / "summary.json").read_text())
        if (
            summary["manifest_sha256"] != sha(directory / "manifest.json")
            or summary["environment_sha256"] != sha(environment_path)
            or summary["attempts_sha256"] != sha(attempts_path)
            or summary["status"] != "VALID_BASELINE_S02_PILOT_BLOCK"
            or summary["invalidators"] != []
        ):
            raise ValueError(f"block {index} summary binding changed")
        attempts = json.loads(attempts_path.read_text())
        if len(attempts) != 4 or [item["label"] for item in attempts] != [
            "warmup",
            "sample-001",
            "sample-002",
            "sample-003",
        ]:
            raise ValueError(f"block {index} sample order changed")
        samples = []
        resources = []
        for attempt in attempts:
            label = attempt["label"]
            path = directory / f"{label}.json"
            raw = json.loads(path.read_text())
            INDEPENDENT_CHECK(raw, manifest)
            if (
                not attempt["valid"]
                or attempt["invalidators"]
                or (
                    attempt["raw_sha256"] != sha(path)
                    or attempt["whole_product_ns"] != raw["whole_product_ns"]
                )
            ):
                raise ValueError(f"block {index} {label} attempt changed")
            check_resource(directory / f"{label}.resource.txt", attempt["resource"])
            if (directory / f"{label}.stdout").stat().st_size or (
                directory / f"{label}.stderr"
            ).stat().st_size:
                raise ValueError(f"block {index} {label} emitted output")
            resources.append(attempt["resource"])
            raw_hashes[str(path.relative_to(ROOT))] = sha(path)
            if label != "warmup":
                samples.append(raw["whole_product_ns"])
                if first_raw is None:
                    first_raw = raw
        environment = json.loads(environment_path.read_text())
        check_environment(environment, resources)
        median = sorted(samples)[1]
        if (
            summary["valid_samples"] != 3
            or summary["median_product_ns"] != median
            or summary["min_product_ns"] != min(samples)
            or summary["max_product_ns"] != max(samples)
        ):
            raise ValueError(f"block {index} summary statistic changed")
        blocks.append(
            {
                "block": index,
                "median_product_ns": median,
                "samples_ns": samples,
                "host_swap_delta_pages": environment["end"]["swap_pages"]
                - environment["start"]["swap_pages"],
                "warnings": environment["warnings"],
            }
        )
    assert first_raw is not None
    negative = deepcopy(first_raw)
    negative["observed"]["after_reopen"]["source_absent"] = False
    try:
        INDEPENDENT_CHECK(negative, manifest)
    except ValueError as error:
        negative_message = str(error)
    else:
        raise AssertionError("surviving source was accepted")
    if negative_message != "after_reopen source_absent failed":
        raise ValueError("negative control rejected for wrong reason")
    changed_environment = deepcopy(
        json.loads((ROOT / "block-01/environment.json").read_text())
    )
    changed_environment["end"]["governor"] = "powersave"
    try:
        check_environment(changed_environment, resources)
    except ValueError as error:
        environment_negative = str(error)
    else:
        raise AssertionError("governor-drift negative control was accepted")
    if environment_negative != "host identity changed":
        raise ValueError("environment negative control rejected for wrong reason")
    medians = [item["median_product_ns"] for item in blocks]
    warning_free = [
        item["median_product_ns"] for item in blocks if not item["warnings"]
    ]
    output = {
        "schema_version": 1,
        "status": "CONTROLLED_BASELINE_ONLY_NO_PAIR",
        "pilot_manifest_sha256": sha(ROOT / "pilot-manifest.json"),
        "raw_sha256": raw_hashes,
        "blocks": blocks,
        "valid_samples": 15,
        "median_spread_ns": max(medians) - min(medians),
        "median_spread_fraction": (max(medians) - min(medians))
        / statistics.median(medians),
        "warning_free_median_spread_ns": max(warning_free) - min(warning_free),
        "warning_free_blocks": len(warning_free),
        "negative_control_rejected": negative_message,
        "environment_negative_control_rejected": environment_negative,
    }
    (ROOT / "audit.json").write_text(json.dumps(output, indent=2) + "\n")
    print(
        json.dumps(
            {
                "valid": output["valid_samples"],
                "median_spread_ns": output["median_spread_ns"],
            }
        )
    )


if __name__ == "__main__":
    main()
