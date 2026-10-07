#!/usr/bin/env python3
"""Independently audit the installed TypeScript capability receipt and archives."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import tarfile
from typing import Any


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def recount(operations: list[str], rows: dict[str, Any]) -> dict[str, int]:
    """Recount disjoint outcomes against the canonical live-operation IDs."""
    if len(operations) != len(set(operations)) or set(rows) != set(operations):
        raise ValueError("row set differs from canonical operations")
    counts = {"supported": len(operations), "executed": 0, "failed": 0, "gap": 0, "unavailable": 0}
    for row in rows.values():
        status = row.get("status")
        if status not in ("executed", "failed", "gap", "unavailable"):
            raise ValueError("unknown operation status")
        if status == "executed" and not (
            row.get("positive", {}).get("case")
            and row.get("negative", {}).get("case")
            and row.get("reopen", {}).get("databases", 0) > 0
        ):
            raise ValueError("executed row lacks independently asserted evidence")
        if status == "failed" and not row.get("error"):
            raise ValueError("failed row lacks error")
        if status in ("gap", "unavailable") and not (row.get("reason") and row.get("owner")):
            raise ValueError("unexplained gap")
        counts[status] += 1
    return counts


def archive_member(path: Path, name: str) -> bytes:
    with tarfile.open(path, "r:gz") as archive:
        member = archive.extractfile(name)
        if member is None:
            raise ValueError(f"archive member missing: {name}")
        return member.read()


def audit(raw: dict[str, Any], canonical_path: Path,
          receipt_dir: Path | None = None, repo: Path | None = None) -> dict[str, Any]:
    """Check accounting, selected real runs, source hashes and installed archive bytes."""
    canonical = json.loads(canonical_path.read_text())
    live = [entry for entry in canonical["operations"] if entry["state"] == "live"]
    operations = [entry["id"] for entry in live]
    spellings = {entry["id"]: entry["typescript"]["spelling"] for entry in live}
    if raw["canonical"]["sha256"] != digest(canonical_path):
        raise ValueError("canonical map hash mismatch")
    counts = recount(operations, raw["operations"])
    if counts != raw["counts"]:
        raise ValueError("operation counts mismatch")
    if len(raw["archives"]) != 2:
        raise ValueError("expected main and native npm archives")
    archives = [receipt_dir / Path(item["path"]).name if receipt_dir else Path(item["path"])
                for item in raw["archives"]]
    for item, path in zip(raw["archives"], archives):
        if digest(path) != item["sha256"]:
            raise ValueError("archive hash mismatch")
    main_archive, native_archive = archives
    installed = raw["artifact"]
    archived_only = receipt_dir is not None
    for name, member, archive in [
        ("module", "package/dist/index.js", main_archive),
        ("main_package", "package/package.json", main_archive),
        ("native", "package/fathomdb.linux-x64-gnu.node", native_archive),
        ("native_package", "package/package.json", native_archive),
    ]:
        file_path = Path(installed[name]["path"])
        archived = archive_member(archive, member)
        if hashlib.sha256(archived).hexdigest() != installed[name]["sha256"]:
            raise ValueError(f"installed {name} differs from archived member")
        if not archived_only and (digest(file_path) != installed[name]["sha256"]
                                  or file_path.read_bytes() != archived):
            raise ValueError(f"installed {name} hash mismatch")
    for fixture in raw["fixtures"]:
        source = (repo / "src/python/tests" / Path(fixture["path"]).name) if repo else Path(fixture["path"])
        if digest(source) != fixture["sha256"]:
            raise ValueError("fixture hash mismatch")
    for name, case in raw["cases"].items():
        source = (repo / "src/ts/tests" / Path(case["testSource"]).name) if repo else Path(case["testSource"])
        if name != case["case"] or digest(source) != case["testSourceSha256"]:
            raise ValueError("test source hash mismatch")
        adapted = (archive_member(receipt_dir / "adapted-tests.tgz",
                                  "tests/" + Path(case["target"]).name)
                   if archived_only else Path(case["target"]).read_bytes())
        if hashlib.sha256(adapted).hexdigest() != case["installedTestSha256"]:
            raise ValueError("installed test adaptation hash mismatch")
        if case["status"] == "passed" and not (
            case["exit"] == 0
            and "# pass 1\n" in case["stdout"]
            and "# fail 0\n" in case["stdout"]
            and case["reopen"]
            and not case["stderr"]
        ):
            raise ValueError("case pass lacks one real test and reopen")
    for operation, row in raw["operations"].items():
        if row["status"] == "gap" and row.get("partial_case"):
            if raw["cases"][row["partial_case"]]["status"] != "passed":
                raise ValueError(f"partial case failed: {operation}")
        if row["status"] != "executed":
            continue
        positive = raw["cases"][row["positive"]["case"]]
        if positive["status"] != "passed" or len(positive["reopen"]) != row["reopen"]["databases"]:
            raise ValueError(f"positive/reopen mismatch: {operation}")
        source = (repo / "src/ts/tests" / Path(positive["testSource"]).name) if repo else Path(positive["testSource"])
        code = source.read_text()
        code = re.sub(r"/\*.*?\*/|//[^\n]*", "", code, flags=re.DOTALL)
        if not re.search(r"\b" + re.escape(spellings[operation]) + r"\s*\(", code):
            raise ValueError(f"positive test source does not invoke {operation}")
        negative = row["negative"]["case"]
        if negative == "direct":
            if raw["direct"]["observed"][operation]["status"] != "passed":
                raise ValueError(f"direct refusal missing: {operation}")
        elif raw["cases"][negative]["status"] != "passed":
            raise ValueError(f"typed refusal case failed: {operation}")
    return {"status": "PASS", "counts": counts, "cases": len(raw["cases"]),
            "archived_only": archived_only,
            "native_sha256": installed["native"]["sha256"]}


def negative_controls(raw: dict[str, Any], canonical_path: Path,
                      receipt_dir: Path | None = None, repo: Path | None = None) -> list[str]:
    """Prove the audit rejects missing IDs, false counts and archive tampering."""
    rejected = []
    mutations = {
        "missing_operation": lambda value: value["operations"].pop(next(iter(value["operations"]))),
        "false_count": lambda value: value["counts"].__setitem__("executed", 999),
        "archive_digest": lambda value: value["archives"][0].__setitem__("sha256", "0" * 64),
        "wrong_positive_route": lambda value: value["operations"]["engine.search_projected_text"]["positive"].__setitem__("case", "admin"),
    }
    for name, mutate in mutations.items():
        changed = copy.deepcopy(raw)
        mutate(changed)
        try:
            audit(changed, canonical_path, receipt_dir, repo)
        except ValueError:
            rejected.append(name)
    if len(rejected) != len(mutations):
        raise ValueError("audit accepted a negative control")
    return rejected


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", required=True, type=Path)
    parser.add_argument("--canonical", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--receipt-dir", type=Path)
    parser.add_argument("--repo", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("audit output already exists")
    raw = json.loads(args.raw.read_text())
    result = audit(raw, args.canonical, args.receipt_dir, args.repo)
    result["negative_controls_rejected"] = negative_controls(raw, args.canonical, args.receipt_dir, args.repo)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
