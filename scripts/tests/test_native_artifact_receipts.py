#!/usr/bin/env python3
"""Hermetic contract for five-target native artifact receipts."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/release/native-artifact-receipts.py"
MANIFEST_SCRIPT = ROOT / "scripts/release/slice50-candidate-manifest.py"
WORKFLOW = ROOT / ".github/workflows/ci.yml"
CANDIDATE = subprocess.run(
    ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
    check=True,
    capture_output=True,
    text=True,
).stdout.strip()
TARGETS = (
    ("x86_64-unknown-linux-gnu", "linux-x64-gnu"),
    ("aarch64-unknown-linux-gnu", "linux-arm64-gnu"),
    ("x86_64-apple-darwin", "darwin-x64"),
    ("aarch64-apple-darwin", "darwin-arm64"),
    ("x86_64-pc-windows-msvc", "win32-x64-msvc"),
)


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        text=True,
        capture_output=True,
        check=False,
    )


def run_manifest(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(MANIFEST_SCRIPT), *args],
        text=True,
        capture_output=True,
        check=False,
    )


def main() -> None:
    assert SCRIPT.is_file(), f"missing native receipt helper: {SCRIPT}"
    assert MANIFEST_SCRIPT.is_file(), f"missing candidate manifest helper: {MANIFEST_SCRIPT}"
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        receipts = root / "receipts"
        receipts.mkdir()
        for target, label in TARGETS:
            wheel = root / f"fathomdb-{label}.whl"
            napi = root / f"fathomdb.{label}.node"
            wheel.write_bytes(f"wheel:{label}".encode())
            napi.write_bytes(f"napi:{label}".encode())
            output = receipts / f"native-artifact-receipt-{label}.json"
            result = run(
                "write",
                "--candidate-sha", CANDIDATE,
                "--target", target,
                "--label", label,
                "--runner", f"runner-{label}",
                "--rust", "1.95.0",
                "--python", "3.11",
                "--node", "25.9.0",
                "--wheel", str(wheel),
                "--napi", str(napi),
                "--command", "native-artifact-runtime-validation",
                "--output", str(output),
            )
            assert result.returncode == 0, result.stderr
            payload = json.loads(output.read_text(encoding="utf-8"))
            assert payload["schema_version"] == "fathomdb.native-artifact-receipt/v1"
            assert payload["candidate_sha"] == CANDIDATE
            assert payload["target"] == target
            assert payload["outcome"] == "pass"
            by_kind = {row["kind"]: row for row in payload["artifacts"]}
            assert set(by_kind) == {"wheel", "napi"}
            assert by_kind["wheel"]["size"] == wheel.stat().st_size
            assert by_kind["wheel"]["sha256"] == hashlib.sha256(
                wheel.read_bytes()
            ).hexdigest()

        matrix = root / "native-artifact-matrix.json"
        collected = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert collected.returncode == 0, collected.stderr
        payload = json.loads(matrix.read_text(encoding="utf-8"))
        assert payload["schema_version"] == "fathomdb.native-artifact-matrix/v1"
        assert payload["candidate_sha"] == CANDIDATE
        assert {row["target"] for row in payload["platforms"]} == {
            target for target, _label in TARGETS
        }

        receipt_path = receipts / "native-artifact-receipt-linux-x64-gnu.json"
        original = receipt_path.read_text(encoding="utf-8")
        malformed = json.loads(original)
        del malformed["command"]
        receipt_path.write_text(json.dumps(malformed), encoding="utf-8")
        rejected = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert rejected.returncode == 1 and "invalid receipt keys" in rejected.stderr
        receipt_path.write_text(original, encoding="utf-8")

        mislabeled = json.loads(original)
        mislabeled["label"] = "darwin-x64"
        receipt_path.write_text(json.dumps(mislabeled), encoding="utf-8")
        rejected = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert rejected.returncode == 1 and "target-label mismatch" in rejected.stderr
        receipt_path.write_text(original, encoding="utf-8")

        malformed = json.loads(original)
        malformed["artifacts"][0]["sha256"] = "bad"
        receipt_path.write_text(json.dumps(malformed), encoding="utf-8")
        rejected = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert rejected.returncode == 1 and "invalid artifact digest" in rejected.stderr
        receipt_path.write_text(original, encoding="utf-8")

        (receipts / "native-artifact-receipt-linux-arm64-gnu.json").unlink()
        missing = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert missing.returncode == 1
        assert "missing target" in missing.stderr, missing.stderr

        contract = root / "python-test-hooks-v1.json"
        contract.write_text('{"schema_version":"test"}\n', encoding="utf-8")
        wal_receipt = root / "windows-wal-receipt.json"
        wal = run(
            "write-wal",
            "--candidate-sha", CANDIDATE,
            "--wheel", str(wheel),
            "--hook-contract", str(contract),
            "--command", "windows-wal-attribution",
            "--output", str(wal_receipt),
        )
        assert wal.returncode == 0, wal.stderr
        wal_payload = json.loads(wal_receipt.read_text(encoding="utf-8"))
        assert wal_payload["schema_version"] == "fathomdb.windows-wal-receipt/v1"
        assert wal_payload["candidate_sha"] == CANDIDATE
        assert wal_payload["outcome"] == "pass"
        assert {item["kind"] for item in wal_payload["artifacts"]} == {
            "test-hooks-wheel",
            "hook-contract",
        }

        npm = root / "fathomdb-0.8.26.tgz"
        cli = root / "fathom"
        graph = root / "slice50-graph-evidence.json"
        generated_scan = root / "gitleaks-generated.txt"
        tracked_scan = root / "gitleaks-tracked.txt"
        npm.write_bytes(b"npm package")
        cli.write_bytes(b"cli binary")
        graph_rows = []
        for seed in ("explicit", "query"):
            for direction in ("outgoing", "incoming", "both"):
                for depth in (1, 2):
                    graph_rows.append(
                        {
                            "seed": seed,
                            "direction": direction,
                            "depth": depth,
                            "hop_count": depth,
                            "target_index": 0,
                            "target_ref": "target",
                            "terminal_ref": "edge",
                            "resolved_target_revision": "target-r1",
                            "resolved_edge_revision": "edge-r1",
                            "edge_source": "actuated" if not graph_rows else "ordinary",
                            "route_provenance": ["edge"],
                            "intrinsic_evidence": ["target-r1", "edge-r1"],
                        }
                    )
        snapshot = [{"name": "schema-33.sqlite", "size": 1, "sha256": "a" * 64}]
        graph.write_text(
            json.dumps(
                {
                    "schema_version": "fathomdb.slice50-graph-evidence/v1",
                    "rows": graph_rows,
                    "schema_33_refusal": {
                        "database": "schema-33.sqlite",
                        "expected_schema": 33,
                        "supported_schema": 34,
                        "outcome": "typed_refusal",
                        "before": snapshot,
                        "after": snapshot,
                    },
                }
            ),
            encoding="utf-8",
        )
        generated_scan.write_text("PASS generated scan\n", encoding="utf-8")
        tracked_scan.write_text("PASS tracked scan\n", encoding="utf-8")
        manifest = root / "candidate-manifest.json"
        assemble_args = [
            "assemble",
            "--repo-root", str(ROOT),
            "--candidate-sha", CANDIDATE,
            "--rust", "1.95.0",
            "--python", "3.12",
            "--node", "25.9.0",
            "--artifact", f"wheel={wheel}",
            "--artifact", f"npm={npm}",
            "--artifact", f"cli={cli}",
            "--evidence", f"graph-evidence={graph}",
            "--evidence", f"gitleaks-generated={generated_scan}",
            "--evidence", f"gitleaks-tracked={tracked_scan}",
            "--native-matrix", str(matrix),
            "--windows-wal-receipt", str(wal_receipt),
            "--command", "agent-verify=pass",
            "--command", "installed-wheel-profile=pass",
            "--command", "installed-npm-profile=pass",
            "--command", "cli-integrity=pass",
            "--command", "gitleaks-generated=pass",
            "--command", "gitleaks-tracked=pass",
            "--output", str(manifest),
        ]
        wrong_candidate_args = list(assemble_args)
        candidate_index = wrong_candidate_args.index("--candidate-sha") + 1
        wrong_candidate_args[candidate_index] = "b" * 40
        rejected = run_manifest(*wrong_candidate_args)
        assert rejected.returncode == 1
        assert "candidate SHA does not match repo HEAD" in rejected.stderr

        assembled = run_manifest(*assemble_args)
        assert assembled.returncode == 0, assembled.stderr
        validated = run_manifest("validate", "--manifest", str(manifest))
        assert validated.returncode == 0, validated.stderr
        candidate = json.loads(manifest.read_text(encoding="utf-8"))
        assert candidate["schema_version"] == "fathomdb.slice50-candidate/v1"
        assert candidate["axes"] == {"workspace": "0.8.26", "embedder_api": "0.6.1"}
        assert {item["kind"] for item in candidate["artifacts"]} == {"wheel", "npm", "cli"}
        assert candidate["external"]["native_matrix"]["state"] == "passed"
        assert candidate["external"]["windows_wal"]["state"] == "passed"
        assert len(candidate["external"]["native_matrix"]["receipt"]["platforms"]) == 5
        assert len(candidate["profiles"]["graph_evidence"]["payload"]["rows"]) == 12

        matrix_source = matrix.read_text(encoding="utf-8")
        forged_matrix = {
            "schema_version": "fathomdb.native-artifact-matrix/v1",
            "candidate_sha": CANDIDATE,
            "platforms": [{}, {}, {}, {}, {}],
        }
        matrix.write_text(json.dumps(forged_matrix), encoding="utf-8")
        forged = run_manifest(*assemble_args)
        assert forged.returncode == 1 and "invalid receipt keys" in forged.stderr
        matrix.write_text(matrix_source, encoding="utf-8")

        wal_source = wal_receipt.read_text(encoding="utf-8")
        wal_receipt.write_text(
            json.dumps(
                {
                    "schema_version": "fathomdb.windows-wal-receipt/v1",
                    "candidate_sha": CANDIDATE,
                    "outcome": "pass",
                }
            ),
            encoding="utf-8",
        )
        forged = run_manifest(*assemble_args)
        assert forged.returncode == 1 and "invalid Windows WAL receipt keys" in forged.stderr
        wal_receipt.write_text(wal_source, encoding="utf-8")

        graph_source = graph.read_text(encoding="utf-8")
        forged_graph = json.loads(graph_source)
        forged_graph["rows"] = []
        graph.write_text(json.dumps(forged_graph), encoding="utf-8")
        forged = run_manifest(*assemble_args)
        assert forged.returncode == 1 and "missing graph evidence case" in forged.stderr
        graph.write_text(graph_source, encoding="utf-8")

        unresolved = json.loads(json.dumps(candidate))
        unresolved["external"] = {
            "native_matrix": {"state": "unresolved", "reason": "executor unavailable"},
            "windows_wal": {"state": "unresolved", "reason": "executor unavailable"},
        }
        manifest.write_text(json.dumps(unresolved), encoding="utf-8")
        validated = run_manifest("validate", "--manifest", str(manifest))
        assert validated.returncode == 0, validated.stderr

        tampered = json.loads(json.dumps(candidate))
        tampered["external"]["native_matrix"]["receipt"]["platforms"][0][
            "runner"
        ] = "different-runner"
        manifest.write_text(json.dumps(tampered), encoding="utf-8")
        rejected = run_manifest("validate", "--manifest", str(manifest))
        assert rejected.returncode == 1 and "payload digest mismatch" in rejected.stderr

        tampered = json.loads(json.dumps(candidate))
        tampered["profiles"]["graph_evidence"]["payload"]["rows"][0][
            "resolved_target_revision"
        ] = "different-valid-revision"
        manifest.write_text(json.dumps(tampered), encoding="utf-8")
        rejected = run_manifest("validate", "--manifest", str(manifest))
        assert rejected.returncode == 1 and "graph evidence payload digest mismatch" in rejected.stderr

        candidate["commands"]["agent-verify"] = "not-run"
        manifest.write_text(json.dumps(candidate), encoding="utf-8")
        rejected = run_manifest("validate", "--manifest", str(manifest))
        assert rejected.returncode == 1 and "invalid command outcome" in rejected.stderr

    workflow = WORKFLOW.read_text(encoding="utf-8")
    assert "native-artifact-receipts.py write" in workflow
    assert "native-artifact-receipt-${{ matrix.label }}" in workflow
    assert "collect-native-artifact-receipts:" in workflow
    assert "native-artifact-receipts.py collect" in workflow
    assert "native-artifact-receipts.py write-wal" in workflow
    assert "slice50-windows-wal-receipt.json" in workflow
    print("PASS test-native-artifact-receipts")


if __name__ == "__main__":
    main()
