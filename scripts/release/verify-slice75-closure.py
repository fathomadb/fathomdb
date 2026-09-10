#!/usr/bin/env python3
"""Validate the sealed Slice 75 manifest without executing its cells."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any


SCHEMA = "fathomdb.slice75-closure-manifest/v1"
TOP_LEVEL_KEYS = {
    "schema_version",
    "release",
    "branch",
    "suite_labels",
    "bindings",
    "global_rules",
    "fixed_inputs",
    "input_sets",
    "cell_input_sets",
    "external_inputs_by_cell",
    "cells",
    "retained",
    "excluded",
}
CELL_KEYS = {"id", "timeout_seconds", "commands", "expected", "unset_environment", "unavailable"}
REQUIRED_CELLS = {
    "default-tree",
    "final-interactions",
    "schema26-upgrade",
    "ac021",
    "ac059b",
    "ac034ab",
    "performance",
    "eu7-real",
    "model-cache",
    "model-ts",
    "model-engine",
    "model-python",
    "model-cli",
    "rust-release-build",
    "rust-leaf-packages",
    "cli-installed-smoke",
    "linux-artifact-build",
    "linux-artifact-current-smoke",
    "linux-runtime-floor-smokes",
    "linux-cuda-package",
    "global-01-native",
    "windows-runner-preflight",
    "hosted-native-validation",
    "jetson-tegra",
    "mkdocs",
    "hosted-ci",
}
COUNT_CONTRACT = {
    "default-tree": ("suite_labels", 109),
    "final-interactions": ("passed", 4),
    "schema26-upgrade": ("passed", 2),
    "ac021": ("passed", 1),
    "ac059b": ("passed", 2),
    "ac034ab": ("passed", 1),
    "performance": ("passed", 3),
    "eu7-real": ("passed", 1),
    "model-ts": ("passed", 33),
    "model-engine": ("passed", 9),
    "model-python": ("passed", 13),
    "model-cli": ("passed", 2),
}
PERFORMANCE_LIMITS = {
    "text_p50_ms_max": 20,
    "text_p99_ms_max": 150,
    "vector_p50_ms_max": 80,
    "vector_p99_ms_max": 300,
    "concurrent_ratio_max": 0.1875,
}
RETAINED_DIGESTS = {
    "slice71-write": ("receipt_sha256", "c356a7f6d900103866811c6174e423fb5c0affe6eb7a80bd6fe301f6111dabd5"),
    "slice71-ac072": ("receipt_sha256", "4291cc2a5c7843190da2422e9ab1b8e568ec3ec4a858ae857d605820fc60b190"),
    "slice72": ("receipt_sha256", "9cee6f677e8aa9ff3dd7ca48f9eeeea93c82a394b9c66132f3f1f5725cd27e14"),
    "slice73": ("receipt_sha256", "4020aad58cfef363aab83f782de215fc4f9ad648454fab40f72408cc80819bbe"),
}
EXCLUDED = {
    "scripts/check.sh",
    "agent-verify-fast",
    "agent-verify-heavy",
    "synthetic-recall-verdict",
    "paid-global",
    "tracked-100k",
    "tracked-1m",
    "tegra-napi",
    "windows-cuda",
    "windows-arm64",
    "windows-ia32",
    "linux-musl",
    "linux-armv7",
}
SEALED_EXECUTION_SHA256 = "f8f72a68b8d3425a64b616bf91a317d8c248c6f197ad9be7cdfe160e9fea0092"
SEALED_EXECUTION_KEYS = (
    "bindings",
    "global_rules",
    "input_sets",
    "cell_input_sets",
    "external_inputs_by_cell",
    "cells",
    "excluded",
)


class InvalidManifest(ValueError):
    """The closure manifest is incomplete or weaker than the sealed contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise InvalidManifest(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate(manifest: dict[str, Any], repo: Path) -> None:
    unknown = set(manifest) - TOP_LEVEL_KEYS
    require(not unknown, f"unknown top-level keys: {sorted(unknown)}")
    require(set(manifest) == TOP_LEVEL_KEYS, "missing required top-level keys")
    require(manifest["schema_version"] == SCHEMA, "wrong schema_version")
    require(manifest["release"] == "0.8.25", "release must be 0.8.25")
    require(manifest["branch"] == "release/0.8.25", "branch must be release/0.8.25")
    require(manifest["suite_labels"] == 109, "suite_labels must remain 109")
    require(set(manifest["excluded"]) == EXCLUDED, "excluded cells changed")

    rules = manifest["global_rules"]
    require(
        rules.get("forbidden_output_patterns") == ["[skip]", "0 tests", "timed out"],
        "forbidden_output_patterns must remain fail-closed",
    )
    require(rules.get("dirty_product_tree") == "fail", "dirty_product_tree must fail")
    require(rules.get("retry_count") == 0, "retry_count must remain zero")

    cells_raw = manifest["cells"]
    require(isinstance(cells_raw, list), "cells must be a list")
    cells: dict[str, dict[str, Any]] = {}
    for cell in cells_raw:
        require(isinstance(cell, dict), "each cell must be an object")
        require(not set(cell) - CELL_KEYS, f"unknown keys in cell {cell.get('id')}")
        cell_id = cell.get("id")
        require(isinstance(cell_id, str) and cell_id not in cells, "cell IDs must be unique strings")
        require(isinstance(cell.get("timeout_seconds"), int) and cell["timeout_seconds"] > 0, f"{cell_id}: invalid timeout")
        commands = cell.get("commands")
        require(isinstance(commands, list) and commands and all(isinstance(v, str) and v for v in commands), f"{cell_id}: commands must be nonempty")
        require(all("--no-verify" not in command and "scripts/check.sh" not in command for command in commands), f"{cell_id}: forbidden command")
        require(isinstance(cell.get("expected"), dict), f"{cell_id}: expected must be an object")
        cells[cell_id] = cell
    require(set(cells) == REQUIRED_CELLS, "required cells do not match the sealed 26-cell matrix")
    require(set(manifest["cell_input_sets"]) == REQUIRED_CELLS, "cell_input_sets must cover every required cell exactly")

    for cell_id, (field, expected) in COUNT_CONTRACT.items():
        require(cells[cell_id]["expected"].get(field) == expected, f"{cell_id}: {field} must equal {expected}")
    require(cells["model-ts"]["expected"].get("per_file") == [2, 1, 5, 2, 11, 6, 6], "model-ts: per_file counts changed")
    for cell_id in ("model-ts", "model-engine", "model-python", "model-cli"):
        require(cells[cell_id]["expected"].get("skipped") == 0, f"{cell_id}: skipped must equal zero")
        require(
            "FATHOMDB_SKIP_NETWORK_TESTS" in cells[cell_id].get("unset_environment", []),
            f"{cell_id}: FATHOMDB_SKIP_NETWORK_TESTS must be explicitly unset",
        )

    performance = cells["performance"]
    for field, expected in PERFORMANCE_LIMITS.items():
        require(performance["expected"].get(field) == expected, f"performance: {field} must equal {expected}")
    require(
        set(performance.get("unset_environment", []))
        == {"AC_FULL_SCALE", "AC013_SCALE_TREATMENT", "AC013_DRAIN_TIMEOUT_MS"},
        "performance environment exclusions changed",
    )
    require(cells["eu7-real"]["expected"].get("ac075_recall_ci_hi_min") == 0.9, "EU7 AC-075 floor changed")
    require("FATHOMDB_SKIP_NETWORK_TESTS" in cells["eu7-real"].get("unset_environment", []), "EU7 must unset FATHOMDB_SKIP_NETWORK_TESTS")
    require(cells["schema26-upgrade"]["expected"].get("steps") == list(range(27, 34)), "schema26-upgrade steps changed")
    require(cells["linux-artifact-current-smoke"]["expected"].get("source_fallback") is False, "source_fallback must remain false")

    global_expected = cells["global-01-native"]["expected"]
    require("answer_quality" not in global_expected, "GLOBAL-01 data-plane cell cannot claim answer_quality")
    require(global_expected.get("spend_usd") == 0, "GLOBAL-01 must remain zero-spend")
    require(global_expected.get("engine_search_calls") == 1, "GLOBAL-01 must call Engine.search exactly once")

    windows_command = " ".join(cells["windows-runner-preflight"]["commands"])
    require("fathomadb/fathomdb" in windows_command and "windchill3-windows-11" in windows_command, "Windows runner identity changed")
    require(
        cells["windows-runner-preflight"]["expected"].get("labels")
        == ["self-hosted", "Windows", "X64", "windchill3-windows-11"],
        "Windows runner labels changed",
    )
    jetson_command = " ".join(cells["jetson-tegra"]["commands"])
    require("--candidate-version 0.8.25" in jetson_command, "Jetson candidate version changed")
    require(cells["jetson-tegra"]["expected"].get("publication_jobs") == 0, "Jetson publication must remain disabled")

    fixed_inputs = manifest["fixed_inputs"]
    for relative, expected in fixed_inputs.items():
        path = repo / relative
        require(path.is_file(), f"fixed input missing: {relative}")
        require(re.fullmatch(r"[0-9a-f]{64}", expected) is not None, f"invalid fixed input digest: {relative}")
        require(sha256(path) == expected, f"fixed input digest mismatch: {relative}")

    retained_by_id = {item.get("id"): item for item in manifest["retained"]}
    require(set(retained_by_id) == set(RETAINED_DIGESTS), "retained evidence IDs changed")
    for retained_id, (field, expected) in RETAINED_DIGESTS.items():
        item = retained_by_id[retained_id]
        require(item.get(field) == expected, f"{retained_id}: {field} changed")
        receipt = repo / item["receipt"]
        require(receipt.is_file(), f"{retained_id}: receipt missing")
        require(sha256(receipt) == expected, f"{retained_id}: receipt_sha256 mismatch")
        require(item.get("invalidation_paths"), f"{retained_id}: invalidation_paths missing")
        for relative in item["invalidation_paths"]:
            require((repo / relative).exists(), f"{retained_id}: invalidation path missing: {relative}")
    binding_item = retained_by_id["slice71-ac072"]
    binding = repo / binding_item["binding"]
    require(binding.is_file(), "slice71-ac072: binding missing")
    require(sha256(binding) == binding_item.get("binding_sha256"), "slice71-ac072: binding_sha256 mismatch")

    sealed_execution = {key: manifest[key] for key in SEALED_EXECUTION_KEYS}
    sealed_bytes = json.dumps(
        sealed_execution, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    require(
        hashlib.sha256(sealed_bytes).hexdigest() == SEALED_EXECUTION_SHA256,
        "sealed execution commands, timeouts, environments, inputs, or expectations changed",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    try:
        value = json.loads(args.manifest.read_text(encoding="utf-8"))
        require(isinstance(value, dict), "manifest root must be an object")
        validate(value, args.repo.resolve())
    except (OSError, json.JSONDecodeError, InvalidManifest) as error:
        print(f"slice75-closure: {error}", file=sys.stderr)
        return 1
    print(f"slice75-closure: valid ({len(value['cells'])} cells)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
