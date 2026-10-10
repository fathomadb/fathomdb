#!/usr/bin/env python3
"""Focused contract for 0.8.27 Slice 10 repository preparation."""

from __future__ import annotations

import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
UPLOAD_SHA = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def latest_published_release(root: Path = ROOT) -> str:
    checker = runpy.run_path(str(ROOT / "scripts/check-public-doc-truth.py"))
    return checker["current_published_state"](root)["release"]


def check_published_lookup() -> None:
    with tempfile.TemporaryDirectory(prefix="slice10-release-state-") as directory:
        root = Path(directory)
        state_dir = root / "dev/plans"
        state_dir.mkdir(parents=True)
        tracked = state_dir / "release-state-0.8.26.json"
        tracked.write_text(json.dumps({
            "release": "0.8.26",
            "published": {
                "tag": "v0.8.26",
                "tag_commit": "a" * 40,
                "published_on": "2026-09-19",
                "npm_dist_tag": "latest",
            },
        }))
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        subprocess.run(["git", "-C", str(root), "add", "--", "dev/plans/release-state-0.8.26.json"], check=True)
        (state_dir / "release-state-9.9.9.json").write_text(json.dumps({"release": "9.9.9", "published": {}}))
        require(latest_published_release(root) == "0.8.26", "untracked scratch must not change the published release")


def check_dependency() -> None:
    lock = json.loads((ROOT / "package-lock.json").read_text())
    markdownlint = lock["packages"]["node_modules/markdownlint-cli2"]
    require(markdownlint["version"] == "0.23.3", "markdownlint-cli2 must use the patched upstream release")
    version = lock["packages"]["node_modules/smol-toml"]["version"]
    require(version == "1.8.0", "smol-toml must resolve to the patched 1.8.0 dependency")
    require(markdownlint["dependencies"]["smol-toml"] == "1.8.0", "the upstream dependency must be patched")
    expected_cohort = {
        "node_modules/js-yaml": "5.4.1",
        "node_modules/markdown-it": "15.0.1",
        "node_modules/linkify-it": "6.1.0",
    }
    for path, expected in expected_cohort.items():
        require(lock["packages"][path]["version"] == expected, f"{path} must resolve to {expected}")
    package = json.loads((ROOT / "package.json").read_text())
    for expected in ("js-yaml@5.4.1", "markdown-it@15.0.1", "linkify-it@6.1.0"):
        require(expected in package["comment-security"], f"security provenance must name {expected}")


def check_action_comments() -> None:
    workflow = (ROOT / ".github/workflows/ci.yml").read_text()
    pinned = f"actions/upload-artifact@{UPLOAD_SHA} # v7.0.1"
    require(workflow.count(pinned) == 2, "both upload-artifact comments must identify v7.0.1")
    require(workflow.count(f"actions/upload-artifact@{UPLOAD_SHA}") == 7, "all upload SHAs must stay byte-identical")
    require(f"actions/upload-artifact@{UPLOAD_SHA} # v4.6.2" not in workflow, "stale action comments must be gone")


def check_platform_truth() -> None:
    manifest = json.loads((ROOT / "dev/platform-capabilities.json").read_text())
    require(manifest["release"] == latest_published_release(), "platform manifest must identify the published release")
    published = [item["triple"] for item in manifest["platforms"] if item["status"] == "published"]
    require(
        published
        == ["linux-x64-gnu", "linux-arm64-gnu", "darwin-x64", "darwin-arm64", "win32-x64-msvc"],
        "platform manifest must match the five published native targets",
    )


def check_slice30_prerequisites() -> None:
    path = ROOT / "dev/plans/0.8.27/features/slice-10/slice30-prerequisites.json"
    value = json.loads(path.read_text())
    require(value["schema"] == "fathomdb.slice30-prerequisites.v1", "unexpected prerequisite schema")
    require(value["cargo_public_api"] == "0.52.0", "cargo-public-api version must be exact")
    require(value["nightly"] == "nightly-2026-04-24", "nightly must be exact")
    require(value["owned_root"] == ".cache/0.8.27-slice30", "generated build roots must be checkout-owned")
    require(value["scratch_root"] == "/tmp/fathomdb-0.8.27-slice30", "scratch must be owned outside the repo")
    require(value["heavy_route_min_free_bytes"] == 100_000_000_000, "heavy-route disk floor must be 100 GB")
    require(value["heavy_runner_scratch_bytes"] == 17_179_869_184, "pilot scratch limit must be retained")



def check_planning_truth() -> None:
    detail = (ROOT / "dev/doc-index/plans.md").read_text()
    thin = (ROOT / "dev/DOC-INDEX.md").read_text()
    for stale in (
        "**Active 0.8.25 plan.**",
        "**0.8.25 live board.**",
        "**Active data-plane foldback plan v2.**",
    ):
        require(stale not in detail, f"planning detail retains stale current label: {stale}")
    require("through Slice 65" in thin, "thin index must cover the completed 0.8.26 slice range")
    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts/release-current.py")],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    target = latest_published_release()
    if result.stdout.strip():
        release, _, state_path = result.stdout.strip().split("\t")
        state = json.loads((ROOT / state_path).read_text())
        target = state.get("public_docs_target_release", target)
        require(target == release or target == latest_published_release(), "documentation target must be active or published")
    require(
        thin.count(f"current-link banner names {target}") == 3,
        f"historical release-note index rows must name the {target} banner",
    )


def main() -> None:
    check_dependency()
    check_action_comments()
    check_published_lookup()
    check_platform_truth()
    check_slice30_prerequisites()
    check_planning_truth()
    print("ok    slice10-preparation")


if __name__ == "__main__":
    main()
