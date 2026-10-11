#!/usr/bin/env python3
"""Recompute exact-candidate provider and close target results from raw logs."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


EXPECTED = {
    "foreground": {
        "close_cancels_started_direct_embed_waiter",
        "direct_embed_saturation_and_queued_expiry_are_overloaded",
        "direct_embed_started_error_and_panic_preserve_error_contract_and_worker_reuse",
        "direct_embed_started_timeout_uses_engine_dispatch_deadline",
        "eight_reader_burst_keeps_vector_only_search_results",
        "frozen_search_provider_panic_unwinds_reader_owner_boundary",
        "frozen_sparse_fallback_keeps_reader_snapshot_across_provider_wait",
        "ordinary_and_frozen_search_keep_sparse_fallback_on_provider_error",
        "ordinary_search_provider_panic_reaches_caller_boundary",
        "ordinary_search_uses_sparse_fallback_after_started_provider_timeout",
    },
    "projection": {
        "close_interrupts_long_provider_retry_wait_and_preserves_pending_row",
        "projection_wait_cancels_on_close_and_reopen_recovers_pending_row",
        "provider_retry_delay_uses_one_absolute_deadline_during_wake_storm",
        "timed_out_provider_keeps_slot_and_later_work_durable_until_return",
    },
}


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
    require(len(manifest["runs"]) == 10, "expected ten target runs")
    summaries = []
    for target, expected_names in EXPECTED.items():
        binary = manifest["binaries"][target]
        if Path(binary["path"]).is_file():
            require(
                digest(Path(binary["path"])) == binary["sha256"], "binary hash mismatch"
            )
        target_runs = [run for run in manifest["runs"] if run["target"] == target]
        require(len(target_runs) == 5, f"{target}: expected five runs")
        for ordinal, run in enumerate(target_runs, start=1):
            stem = f"{target}-{ordinal:02d}"
            require(run["ordinal"] == ordinal, f"{stem}: ordinal mismatch")
            require(run["returncode"] == 0, f"{stem}: nonzero exit")
            command = run["command"]
            require(
                command[:3] == ["/usr/bin/time", "-v", "-o"]
                and command[4:] == [binary["path"], "--nocapture", "--test-threads=1"],
                f"{stem}: command mismatch",
            )
            paths = {
                "stdout": root / f"{stem}.stdout",
                "stderr": root / f"{stem}.stderr",
                "resource": root / f"{stem}.resource.txt",
            }
            for kind, path in paths.items():
                require(
                    digest(path) == run[f"{kind}_sha256"],
                    f"{stem}: {kind} hash mismatch",
                )
            stdout = paths["stdout"].read_text()
            stderr = paths["stderr"].read_text()
            resource = paths["resource"].read_text()
            names = {
                match.group(1)
                for match in re.finditer(
                    r"^test (\w+) \.\.\. ok$", stdout, re.MULTILINE
                )
            }
            require(names == expected_names, f"{stem}: executed case set mismatch")
            require(
                f"test result: ok. {len(expected_names)} passed; 0 failed" in stdout,
                f"{stem}: summary mismatch",
            )
            expected_panics = 3 if target == "foreground" else 0
            require(
                stderr.count("injected provider panic") == expected_panics,
                f"{stem}: injected panic count mismatch",
            )
            swaps = re.search(r"^\s*Swaps:\s*(\d+)\s*$", resource, re.MULTILINE)
            rss = re.search(
                r"^\s*Maximum resident set size \(kbytes\):\s*(\d+)\s*$",
                resource,
                re.MULTILINE,
            )
            require(
                swaps is not None and int(swaps.group(1)) == 0, f"{stem}: child swaps"
            )
            require(rss is not None and int(rss.group(1)) > 0, f"{stem}: RSS missing")
            summaries.append(
                {
                    "target": target,
                    "ordinal": ordinal,
                    "passed_cases": len(names),
                    "expected_provider_panics": expected_panics,
                    "peak_rss_kib": int(rss.group(1)),
                    "swaps": int(swaps.group(1)),
                }
            )
    return {"source_sha": source_sha, "target_runs": summaries}


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
