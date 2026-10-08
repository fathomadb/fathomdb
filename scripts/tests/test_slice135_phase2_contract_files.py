"""File-level Phase 2 audit binds raw results to artifacts and retained DBs."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import zipfile

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_phase2_contract_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_contract_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, sort_keys=True) + "\n")


def _campaign(tmp_path: Path) -> tuple[dict, dict]:
    fixture_path = tmp_path / "fixture.json"
    fixture = {
        "corpus": [{"kind": "note", "body": "alpha"}],
        "queries": [{"query": "alpha", "expected_bodies": ["alpha"]}],
    }
    _json(fixture_path, fixture)
    runner_path = tmp_path / "runner.py"
    runner_path.write_text("runner bytes\n")
    oracle_path = tmp_path / "oracle.py"
    oracle_path.write_text("authored order\n")
    paths = {
        "fixture_path": fixture_path,
        "runner_path": runner_path,
        "oracle_path": oracle_path,
        "version_dirs": {},
        "wheels": {},
    }
    versions = {}
    for version in ("baseline", "candidate"):
        wheel = tmp_path / f"{version}.whl"
        native = f"{version} native".encode()
        with zipfile.ZipFile(wheel, "w") as archive:
            archive.writestr("fathomdb/_fathomdb.abi3.so", native)
        versions[version] = {
            "source_sha": ("b" if version == "baseline" else "c") * 40,
            "wheel_sha256": _sha(wheel.read_bytes()),
            "native_sha256": _sha(native),
        }
        paths["wheels"][version] = wheel
    protocol = {
        "schema_version": 1,
        "fixture_sha256": _sha(fixture_path.read_bytes()),
        "runner_sha256": _sha(runner_path.read_bytes()),
        "ordered_case_source_sha256": _sha(oracle_path.read_bytes()),
        "source_id": "source",
        "logical_id_prefix": "id-",
        "ordered_case": {"query": "alpha", "expected_bodies": ["alpha"]},
        "versions": versions,
    }
    protocol_path = tmp_path / "protocol.json"
    _json(protocol_path, protocol)
    paths["protocol_path"] = protocol_path
    for version in versions:
        directory = tmp_path / version
        directory.mkdir()
        database = directory / "contract.sqlite"
        with sqlite3.connect(database) as connection:
            connection.execute(
                "CREATE TABLE canonical_nodes (logical_id TEXT, kind TEXT, body TEXT, "
                "source_id TEXT, state TEXT, superseded_at INTEGER)"
            )
            connection.execute(
                "INSERT INTO canonical_nodes VALUES ('id-0','note','alpha','source','active',NULL)"
            )
        hit = {
            "id_space": "logical", "id_value": "id-0", "kind": "note",
            "body": "alpha", "branch": "text", "source_id": "source",
        }
        raw = {
            "schema_version": 1,
            "version": version,
            **versions[version],
            "fixture_sha256": protocol["fixture_sha256"],
            "runner_sha256": protocol["runner_sha256"],
            "protocol_sha256": _sha(protocol_path.read_bytes()),
            "database_sha256": _sha(database.read_bytes()),
            "sqlite_integrity_check": "ok",
            "environment": {"python_executable": "/isolated/python", "python_version": "3.12", "platform": "linux"},
            "identity": {"module_path": "/isolated/fathomdb", "native_path": "/isolated/native"},
            "cases": [{"query": "alpha", "before": [hit], "after": [hit]}],
        }
        _json(directory / "raw.json", raw)
        paths["version_dirs"][version] = directory
    return paths, protocol


def _audit(paths: dict) -> dict:
    return AUDIT.audit_campaign(
        protocol_path=paths["protocol_path"],
        fixture_path=paths["fixture_path"],
        runner_path=paths["runner_path"],
        oracle_source_path=paths["oracle_path"],
        wheels=paths["wheels"],
        version_dirs=paths["version_dirs"],
    )


def test_file_campaign_accepts_two_independent_gold_matches(tmp_path: Path) -> None:
    paths, _ = _campaign(tmp_path)
    result = _audit(paths)
    assert result["versions"]["baseline"]["queries"] == 1
    assert result["versions"]["candidate"]["canonical_rows"] == 1


@pytest.mark.parametrize("tamper", ["raw", "wheel", "oracle", "database"])
def test_file_campaign_rejects_tampering(tmp_path: Path, tamper: str) -> None:
    paths, _ = _campaign(tmp_path)
    if tamper == "raw":
        path = paths["version_dirs"]["candidate"] / "raw.json"
        raw = json.loads(path.read_text())
        raw["cases"][0]["after"][0]["body"] = "wrong"
        _json(path, raw)
    elif tamper == "wheel":
        paths["wheels"]["candidate"].write_bytes(b"wrong")
    elif tamper == "oracle":
        paths["oracle_path"].write_text("changed order\n")
    else:
        path = paths["version_dirs"]["baseline"] / "contract.sqlite"
        with sqlite3.connect(path) as connection:
            connection.execute("UPDATE canonical_nodes SET body='wrong'")
    with pytest.raises(ValueError):
        _audit(paths)
