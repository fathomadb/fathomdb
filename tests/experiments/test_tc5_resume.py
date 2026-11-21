"""A guarded recovery reuses only the exact completed ingest fixture."""

from dataclasses import replace
import json
from pathlib import Path
import sqlite3

import pytest

from experiments import tc5_gpu_v2 as tc5


def test_run_cli_passes_the_explicit_prior_root_to_the_runner(monkeypatch, tmp_path):
    seen = {}

    def run(config, arm, **kwargs):
        seen.update(config=config, arm=arm, **kwargs)
        return tmp_path / "result.json"

    monkeypatch.setattr(tc5, "run_arm", run)
    assert tc5.main([
        "run", "--config", "chosen.json", "--arm", "bridge", "--output-root",
        str(tmp_path / "new"), "--binary", "chosen-binary", "--resume-from", str(tmp_path / "old"),
    ]) == 0
    assert seen["resume_from"] == tmp_path / "old"


def prior_fixture(tmp_path):
    config = replace(tc5.load_config("experiments/configs/scale-01/tc5-gpu-v2.json"),
                     release="0.8.30", arms={"bridge": 2, "primary": 17272})
    inputs = tc5.ArmInputs(({}, {}), ({"query": "fixture query", "exclude_document_id": "source"},), "a" * 64)
    prior = tmp_path / "prior"
    (prior / "database").mkdir(parents=True)
    (prior / "query-jobs").mkdir()
    database = prior / "database/fathomdb.sqlite"
    connection = sqlite3.connect(database)
    connection.executescript("CREATE TABLE canonical_nodes(kind TEXT); CREATE TABLE _fathomdb_vector_rows(kind TEXT);")
    for table in ["canonical_nodes", "_fathomdb_vector_rows"]:
        connection.executemany(f"INSERT INTO {table} VALUES(?)", [("doc",), ("doc",)])
    connection.commit()
    connection.close()
    binary = tmp_path / "executor"
    binary.write_bytes(b"fixed executor")
    manifest, _ = tc5.build_query_job(
        config, query=inputs.queries[0], database_path=database, document_count=2,
        fixture_digest=inputs.fixture_digest, index_digest=tc5._sha_file(database),
        binary_digest=tc5._sha_file(binary), manifest_path=prior / "query-jobs/query-001.manifest.json",
    )
    (prior / "query-jobs/query-001.manifest.json").write_text(json.dumps(manifest))
    return config, inputs, prior, database, binary


def test_reuse_copies_the_pinned_database_without_mutating_it(tmp_path):
    config, inputs, prior, database, binary = prior_fixture(tmp_path)
    before = database.read_bytes()
    output = tmp_path / "new"
    output.mkdir()
    copied = tc5._copy_prior_database(config, "bridge", inputs, prior, output, binary)
    assert copied != database
    assert copied.read_bytes() == before == database.read_bytes()


@pytest.mark.parametrize("damage", ["bytes", "manifest", "wal"])
def test_reuse_refuses_changed_bytes_inputs_or_uncheckpointed_wal(tmp_path, damage):
    config, inputs, prior, database, binary = prior_fixture(tmp_path)
    if damage == "bytes":
        database.write_bytes(database.read_bytes() + b"changed")
    elif damage == "manifest":
        path = prior / "query-jobs/query-001.manifest.json"
        value = json.loads(path.read_text())
        value["query"] = "different query"
        path.write_text(json.dumps(value))
    else:
        Path(str(database) + "-wal").write_bytes(b"x" * 40)
    output = tmp_path / "new"
    output.mkdir()
    with pytest.raises(tc5.Tc5GpuV2Error):
        tc5._copy_prior_database(config, "bridge", inputs, prior, output, binary)


def test_nonmeasurement_stops_before_launching_more_queries(monkeypatch, tmp_path):
    from types import SimpleNamespace

    config, inputs, prior, _database, binary = prior_fixture(tmp_path)
    inputs = replace(inputs, queries=inputs.queries * 100)
    monkeypatch.setattr(tc5, "load_config", lambda _: config)
    monkeypatch.setattr(tc5, "_validate_candidate_artifacts", lambda *a, **k: None)
    monkeypatch.setattr(tc5, "_validate_candidate_runtime", lambda *a, **k: None)
    monkeypatch.setattr(tc5, "_load_arm_inputs", lambda *a, **k: inputs)
    calls = []

    def unavailable(command, **_kwargs):
        calls.append(command)
        Path(command[-1]).write_text(json.dumps({"status": "device_unavailable", "error_code": "device_preflight_failed"}))
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(tc5.subprocess, "run", unavailable)
    with pytest.raises(tc5.Tc5GpuV2Error):
        tc5.run_arm(tmp_path / "config", "bridge", output_root=tmp_path / "run", binary=binary, resume_from=prior)
    assert len(calls) == 1


def test_resume_can_bind_the_preserved_executor_after_an_inventory_only_fix(tmp_path):
    config, inputs, prior, _database, old_binary = prior_fixture(tmp_path)
    replacement = tmp_path / "new-executor"
    replacement.write_bytes(b"inventory formatting fix")
    output = tmp_path / "new"
    output.mkdir()
    with pytest.raises(tc5.Tc5GpuV2Error):
        tc5._copy_prior_database(config, "bridge", inputs, prior, output, replacement)
    copied = tc5._copy_prior_database(
        config, "bridge", inputs, prior, output, replacement, prior_binary=old_binary,
    )
    assert copied.is_file()


def test_run_cli_preserves_the_previous_executor_binding(monkeypatch, tmp_path):
    seen = {}

    def run(*_args, **kwargs):
        seen.update(kwargs)
        return tmp_path / "result.json"

    monkeypatch.setattr(tc5, "run_arm", run)
    assert tc5.main([
        "run", "--config", "chosen.json", "--arm", "bridge", "--output-root", str(tmp_path / "new"),
        "--binary", "new-binary", "--resume-from", str(tmp_path / "old"), "--prior-executor", "old-binary",
    ]) == 0
    assert seen["prior_executor"] == Path("old-binary")
