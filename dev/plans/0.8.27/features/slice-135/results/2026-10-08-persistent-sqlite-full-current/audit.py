#!/usr/bin/env python3
"""Independently check repeated persistent SQLite-full state receipts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


EXPECTED_BEFORE = ["slice135robust body base", None, None, None, None]
EXPECTED_REOPENED = [
    "slice135robust body base",
    None,
    None,
    None,
    "slice135robust body recovery",
]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition: bool, reason: str) -> None:
    if not condition:
        raise ValueError(reason)


def audit(root: Path, source_sha: str) -> dict[str, object]:
    manifest = json.loads((root / "manifest.json").read_text())
    require(manifest["source_sha"] == source_sha, "source SHA mismatch")
    require(re.fullmatch(r"[0-9a-f]{40}", source_sha) is not None, "invalid source SHA")
    require(
        manifest["features"] == ["test-hooks", "default-embedder"], "feature mismatch"
    )
    require(len(manifest["runs"]) == 5, "expected five runs")
    binary = Path(manifest["binary"])
    if binary.is_file():
        require(digest(binary) == manifest["binary_sha256"], "binary hash mismatch")
    summary = []
    for ordinal, run in enumerate(manifest["runs"], start=1):
        require(run["ordinal"] == ordinal, f"run {ordinal}: ordinal mismatch")
        require(run["returncode"] == 0, f"run {ordinal}: nonzero exit")
        command = run["command"]
        require(command[0] == "/usr/bin/time", f"run {ordinal}: timer mismatch")
        require(command[4] == str(binary), f"run {ordinal}: binary mismatch")
        require(
            command[5:]
            == [
                "--exact",
                "slice135_persistent_sqlite_full_preserves_state_until_cap_lifts",
                "--nocapture",
                "--test-threads=1",
            ],
            f"run {ordinal}: test selector mismatch",
        )
        stem = f"run-{ordinal:02d}"
        paths = {
            "stdout": root / f"{stem}.stdout",
            "stderr": root / f"{stem}.stderr",
            "resource": root / f"{stem}.resource.txt",
        }
        for kind, path in paths.items():
            require(
                digest(path) == run[f"{kind}_sha256"],
                f"run {ordinal}: {kind} hash mismatch",
            )
        stdout = paths["stdout"].read_text()
        stderr = paths["stderr"].read_text()
        resource = paths["resource"].read_text()
        require(
            "test result: ok. 1 passed; 0 failed" in stdout,
            f"run {ordinal}: test result mismatch",
        )
        records = [
            json.loads(line.partition("SLICE135_ROBUSTNESS ")[2])
            for line in stderr.splitlines()
            if "SLICE135_ROBUSTNESS " in line
        ]
        require(len(records) == 1, f"run {ordinal}: expected one state record")
        record = records[0]
        require(
            record["case"] == "persistent_sqlite_full_write",
            f"run {ordinal}: case mismatch",
        )
        require(
            record["before"] == EXPECTED_BEFORE, f"run {ordinal}: seed state mismatch"
        )
        require(
            record["attempt_errors"] == ["Storage"] * 3,
            f"run {ordinal}: errors mismatch",
        )
        require(
            record["states_after_each_failure"] == [EXPECTED_BEFORE] * 3,
            f"run {ordinal}: partial failed write",
        )
        require(
            record["reopened"] == EXPECTED_REOPENED,
            f"run {ordinal}: reopened state mismatch",
        )
        require(
            record["expected_reopened"] == EXPECTED_REOPENED,
            f"run {ordinal}: test oracle mismatch",
        )
        require(
            record["max_page_count"] == record["page_count"] + 1,
            f"run {ordinal}: cap mismatch",
        )
        require(
            record["physical_canonical_rows"] == 2,
            f"run {ordinal}: physical count mismatch",
        )
        require(record["integrity_check"] == "ok", f"run {ordinal}: integrity failure")
        require(record["fd_before"] == record["fd_after"], f"run {ordinal}: FD growth")
        swaps = re.search(r"^\s*Swaps:\s*(\d+)\s*$", resource, re.MULTILINE)
        rss = re.search(
            r"^\s*Maximum resident set size \(kbytes\):\s*(\d+)\s*$",
            resource,
            re.MULTILINE,
        )
        require(
            swaps is not None and int(swaps.group(1)) == 0,
            f"run {ordinal}: child swaps",
        )
        require(
            rss is not None and int(rss.group(1)) > 0, f"run {ordinal}: RSS missing"
        )
        summary.append(
            {
                "ordinal": ordinal,
                "page_count": record["page_count"],
                "max_page_count": record["max_page_count"],
                "peak_rss_kib": int(rss.group(1)),
                "swaps": int(swaps.group(1)),
                "physical_canonical_rows": record["physical_canonical_rows"],
            }
        )
    return {
        "source_sha": source_sha,
        "binary_sha256": manifest["binary_sha256"],
        "runs": summary,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(audit(args.root, args.source_sha), indent=2, sort_keys=True) + "\n"
    )


if __name__ == "__main__":
    main()
