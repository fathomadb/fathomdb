"""Run the fixed Slice 40 cells through the existing native G adapter."""

import argparse
from collections.abc import Iterator
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
from tempfile import TemporaryDirectory
import time

READ_ORDER = ("AB", "BA", "BA", "AB", "BA", "AB", "AB", "BA")
FIXTURE_SHA = "e60fc08720a21748f4cf023a65d5fa55c3fe2df3da045d5e0cba1ee24d6d8f55"
QUERY_SHA = "0eeb5582fc93bee7e2a494054a55d26bb0940d771083b726f999a9682e74dc16"
CORPUS_SHA = "cc5930e525a2af2954353f92dacf720cc4b79f40dbb7965c44bb5c985b5cb481"
MODEL = {
    "config.json": "CONFIG_JSON_SHA256",
    "tokenizer.json": "TOKENIZER_JSON_SHA256",
    "model.safetensors": "MODEL_SAFETENSORS_SHA256",
}
LOADER = Path("src/rust/crates/fathomdb-embedder/src/loader.rs")
CACHE = Path("fathomdb/embedders/0b2926f8a9b1")
CELL_SECONDS = 20 * 60


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def dump(path: Path, data) -> None:
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def load(path: Path):
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def stage_records(result, row):
    batch = next(
        item
        for item in result["observation_batches"]
        if item["operation_id"] == row["operation_id"]
    )
    if batch["issues"] or batch["dropped_records"]:
        raise ValueError("G observer batch is incomplete")
    if any(item["operation_id"] != row["operation_id"] for item in batch["records"]):
        raise ValueError("G observation includes a different request")
    return batch["records"]


def active_phases(result, row) -> set[str]:
    return {
        item["phase"]
        for item in stage_records(result, row)
        if item["status"] == "ok" and item["started_ns"] is not None
    }


def validate_dense(fixture, report, replacements=0):
    loader = LOADER.read_text(encoding="utf-8")
    pins = dict(re.findall(r'(\w+_SHA256):\s*&str\s*=\s*"([0-9a-f]{64})"', loader))
    expected = {
        "natural_records": 100_000,
        "vector_rows": 100_000 + replacements,
        "span_rows": 100_000 + replacements,
        "physical_vector_rows": 100_000 + replacements,
        "format": 37,
        "pending_count": 0,
        "generation_pending": 0,
        "generation_failed": 0,
        "qualified_dense_fixture": True,
    }
    for label, value in expected.items():
        if fixture.get(label) != value:
            raise ValueError(
                f"fixture {label}: {fixture.get(label)!r}, expected {value!r}"
            )
    device = report["embedder_device_resolution"]
    if device["effective_device"] != {"kind": "cpu"} or device["cuda_compiled"]:
        raise ValueError(f"unexpected device: {device}")
    if report["embedder_download_ms"] is not None:
        raise ValueError("model fetched during open")
    identity = report["embedder_identity"]
    if identity != {
        "name": "fathomdb-bge-small-en-v1.5",
        "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
        "dimension": 384,
    }:
        raise ValueError(f"unexpected CPU embedder: {identity}")


def validate_routes(result):
    phases = {"hybrid": set(), "text": set()}
    vector_hits = 0
    for row in result["samples"]:
        phases[row["route"]].update(active_phases(result, row))
        if row["route"] == "hybrid":
            vector_hits += sum(hit["branch"] == "Vector" for hit in row["hits"])
    if (
        not {"query_embedding_dispatch_wall", "vector_retrieval", "node_fts_candidates"}
        <= phases["hybrid"]
    ):
        raise ValueError(f"hybrid failed to witness both arms: {phases['hybrid']}")
    if (
        "query_embedding_dispatch_wall" in phases["text"]
        or "api_text" not in phases["text"]
    ):
        raise ValueError(f"expected direct text without embedding: {phases['text']}")
    if not vector_hits:
        raise ValueError("no vector-bearing returned hits")
    return {
        "vector_hits": vector_hits,
        "phases": {key: sorted(value) for key, value in phases.items()},
    }


@contextmanager
def fresh_copies(seed: Path, count: int) -> Iterator[list[Path]]:
    with TemporaryDirectory(prefix="slice40-subjects-") as directory:
        stores = [Path(directory) / f"{index}.sqlite" for index in range(count)]
        for store in stores:
            subprocess.run(["cp", "--reflink=auto", seed, store], check=True)
        yield stores


def context():
    return {
        "loadavg": os.getloadavg(),
        "affinity": sorted(os.sched_getaffinity(0)),
        "elapsed_epoch": time.time(),
    }


class Study:
    def __init__(self, args):
        self.args = args
        self.start = time.monotonic()
        self.env = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("FATHOMDB_")
        }
        self.env.update(
            FATHOMDB_EMBED_DEVICE="cpu",
            AGENT_LONG="1",
            XDG_CACHE_HOME=str(args.model),
        )
        self.failures = []

    def run(
        self,
        name,
        artifact,
        store,
        routes,
        *,
        family="G-R",
        samples=120,
        warmup=20,
        observe=False,
        first=True,
        repeat=0,
        queries=None,
        replacements=None,
        fixed_at=None,
    ):
        directory = self.args.out / name
        directory.mkdir()
        config = {
            "family": family,
            "adapters": ["sdk"],
            "routes": routes,
            "provider": "default",
            "samples": samples,
            "warmup": warmup,
            "repeat": repeat,
            "observe": observe,
            "record_first_call": first,
            "store": str(store),
            "queries": str(queries or self.args.queries),
        }
        if family == "G-W":
            config.update(
                batch_size=1, searches_per_batch=10, corpus=str(self.args.corpus)
            )
        if replacements is not None:
            config["diagnostic_reopen_replacements"] = replacements
        if fixed_at is not None:
            config["diagnostic_valid_as_of_sequence"] = [fixed_at]
        dump(directory / "config.json", config)
        print(f"start {name}", flush=True)
        started = context()
        limit = min(CELL_SECONDS, 5 * 60 * 60 - (time.monotonic() - self.start))
        deadline = time.monotonic() + max(limit, 0)
        timed_out = False
        wal = Path(str(store) + "-wal")
        wal_peak = 0
        with (directory / "native.log").open("wb") as log:
            process = subprocess.Popen(
                [
                    "/usr/bin/time",
                    "-o",
                    directory / "resources.tsv",
                    "-f",
                    "%U\t%S\t%M\t%I\t%O\t%e",
                    artifact,
                    directory / "config.json",
                    directory / "native",
                ],
                stdout=log,
                stderr=subprocess.STDOUT,
                env=self.env,
                start_new_session=True,
            )
            while process.poll() is None:
                try:
                    wal_peak = max(wal_peak, wal.stat().st_size)
                except FileNotFoundError:
                    pass
                if time.monotonic() >= deadline:
                    timed_out = True
                    os.killpg(process.pid, signal.SIGKILL)
                    break
                time.sleep(0.05)
            code = process.wait()
        dump(
            directory / "attempt.json",
            {
                "started": started,
                "finished": context(),
                "exit": code,
                "watchdog": timed_out,
                "wal_peak_sampled_bytes": wal_peak,
                "wal_sample_interval_ms": 50,
                "artifact": str(artifact),
                "plan_attempts": samples * len(routes),
            },
        )
        print(f"exit {code} {name}", flush=True)
        if code != 0:
            self.failures.append(name)
            return None
        result_path = directory / "native/result.json"
        result = load(result_path) if result_path.is_file() else None
        if result is None or not result["complete"]:
            self.failures.append(name)
            return None
        if family != "G-W":
            for fixture, report in zip(
                result["fixtures"], result["open_reports"], strict=True
            ):
                validate_dense(fixture, report, replacements or 0)
        else:
            validate_dense(result["before_workload"], result["open_report"])
        return result

    def baseline(self):
        with fresh_copies(self.args.seed, 1) as (store,):
            result = self.run(
                "qualify",
                self.args.diagnostic,
                store,
                ["hybrid", "text"],
                family="G-O",
                samples=4,
                warmup=0,
                observe=True,
                first=False,
            )
            if result is None:
                raise ValueError("100k CPU qualification failed")
            dump(self.args.out / "qualification.json", validate_routes(result))
        for repeat, order in enumerate(READ_ORDER[:4]):
            with fresh_copies(self.args.seed, 2) as stores:
                for arm, store in zip(order, stores, strict=True):
                    route = "hybrid" if arm == "A" else "text"
                    self.run(
                        f"read-{repeat}-{route}",
                        self.args.ordinary,
                        store,
                        [route],
                        repeat=repeat,
                    )
                if repeat == 0:
                    for arm, store in zip(order, stores, strict=True):
                        route = "hybrid" if arm == "A" else "text"
                        for run in ("a", "b"):
                            self.run(
                                f"reopen-{route}-{run}",
                                self.args.diagnostic,
                                store,
                                [route],
                                family="G-O",
                                samples=6,
                                warmup=0,
                                observe=True,
                                first=False,
                            )
        for route in ("hybrid", "text"):
            with fresh_copies(self.args.seed, 1) as (store,):
                self.run(
                    f"diagnose-{route}",
                    self.args.diagnostic,
                    store,
                    [route],
                    family="G-O",
                    samples=12,
                    warmup=0,
                    observe=True,
                    first=False,
                )
        for suffix, artifact, sample_count, warmups, observe in (
            ("ordinary", self.args.ordinary, 80, 20, False),
            ("diagnostic", self.args.diagnostic, 20, 4, True),
        ):
            with fresh_copies(self.args.seed, 1) as (store,):
                result = self.run(
                    f"write-{suffix}",
                    artifact,
                    store,
                    ["hybrid"],
                    family="G-W",
                    samples=sample_count,
                    warmup=warmups,
                    observe=observe,
                )
                if result and observe:
                    self.reopen_markers(store, result, artifact, "write-diagnostic")

    def reopen_markers(self, store, result, artifact, label, original_queries=()):
        queries = self.args.out / f"{label}-reopen-queries.jsonl"
        rows = [item["revision_token"] for item in result["probes"]]
        marker_count = len(rows)
        rows.extend(original_queries)
        with self.args.queries.open(encoding="utf-8") as stream:
            rows.append(json.loads(next(stream)))
        queries.write_text(
            "\n".join(map(json.dumps, rows)) + "\n",
            encoding="utf-8",
        )
        reopened = self.run(
            f"{label}-reopen",
            artifact,
            store,
            ["text", "hybrid"],
            family="G-O",
            samples=len(rows),
            warmup=0,
            observe=True,
            first=False,
            queries=queries,
            replacements=marker_count,
        )
        if reopened is None:
            return None
        for index, probe in enumerate(result["probes"]):
            marker = reopened["samples"][index]
            if marker["route"] != "text" or marker["ids"] != [
                f"l:{probe['logical_id']}"
            ]:
                raise ValueError("reopen marker should reach the current record")
            if (
                marker["hits"][0]["write_cursor"]
                != probe["point_probe"]["write_cursor"]
            ):
                raise ValueError("reopen marker should retain the committed cursor")
        return reopened

    def written_reopen_regression(self):
        with fresh_copies(self.args.seed, 1) as (store,):
            result = self.run(
                "write-diagnostic",
                self.args.diagnostic,
                store,
                ["hybrid"],
                family="G-W",
                samples=20,
                warmup=4,
                observe=True,
            )
            if result is not None:
                self.reopen_markers(
                    store, result, self.args.diagnostic, "write-diagnostic"
                )

    def comparison(self):
        for family, orders, samples in (
            ("G-R", READ_ORDER, 120),
            ("G-W", READ_ORDER[:4], 80),
        ):
            for repeat, order in enumerate(orders):
                with fresh_copies(self.args.seed, 2) as stores:
                    for arm, store in zip(order, stores, strict=True):
                        artifact = (
                            self.args.ordinary if arm == "A" else self.args.candidate
                        )
                        self.run(
                            f"pair-{family}-{repeat}-{arm}",
                            artifact,
                            store,
                            ["hybrid"],
                            family=family,
                            samples=samples,
                            repeat=repeat,
                        )
        with fresh_copies(self.args.seed, 2) as stores:
            for arm, store in zip("AB", stores, strict=True):
                artifact = (
                    self.args.diagnostic
                    if arm == "A"
                    else self.args.candidate_diagnostic
                )
                self.run(
                    f"fidelity-{arm}",
                    artifact,
                    store,
                    ["hybrid"],
                    family="G-O",
                    samples=12,
                    warmup=0,
                    observe=True,
                    first=False,
                    fixed_at=1800000000,
                )
        self.natural_fidelity()

    def natural_fidelity(self):
        with self.args.corpus.open(encoding="utf-8") as stream:
            original_queries = [
                " ".join(json.loads(next(stream)).split()[:60]) for _ in range(2)
            ]
        query_path = self.args.out / "prior-body-queries.jsonl"
        query_path.write_text(
            "\n".join(map(json.dumps, original_queries)) + "\n", encoding="utf-8"
        )
        with fresh_copies(self.args.seed, 2) as stores:
            for arm, store in zip("AB", stores, strict=True):
                artifact = (
                    self.args.diagnostic
                    if arm == "A"
                    else self.args.candidate_diagnostic
                )
                label = f"natural-proof-{arm}"
                self.run(
                    f"{label}-before",
                    artifact,
                    store,
                    ["text", "hybrid"],
                    family="G-O",
                    samples=2,
                    warmup=0,
                    observe=True,
                    first=False,
                    queries=query_path,
                )
                result = self.run(
                    label,
                    artifact,
                    store,
                    ["hybrid"],
                    family="G-W",
                    samples=20,
                    warmup=4,
                    observe=True,
                )
                if result is not None:
                    self.reopen_markers(
                        store, result, artifact, label, original_queries
                    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("seed", "queries", "corpus", "ordinary", "diagnostic", "model", "out"):
        parser.add_argument(
            f"--{name}", type=lambda path: Path(path).resolve(), required=True
        )
    parser.add_argument("--candidate", type=lambda path: Path(path).resolve())
    parser.add_argument(
        "--candidate-diagnostic", type=lambda path: Path(path).resolve()
    )
    parser.add_argument(
        "--stage",
        choices=(
            "baseline",
            "written-reopen-regression",
            "comparison",
            "natural-fidelity",
        ),
        default="baseline",
    )
    args = parser.parse_args()
    if args.stage == "comparison" and not (
        args.candidate and args.candidate_diagnostic
    ):
        parser.error("comparison needs ordinary and diagnostic candidate artifacts")
    artifact_paths = (
        args.ordinary,
        args.diagnostic,
        args.candidate,
        args.candidate_diagnostic,
    )
    args.out.mkdir()
    wal = Path(str(args.seed) + "-wal")
    if wal.exists() and wal.stat().st_size:
        raise ValueError("expected the closed baseline to have no pending WAL")
    loader = LOADER.read_text(encoding="utf-8")
    pins = dict(re.findall(r'(\w+_SHA256):\s*&str\s*=\s*"([0-9a-f]{64})"', loader))
    expected = {
        args.seed: FIXTURE_SHA,
        args.queries: QUERY_SHA,
        args.corpus: CORPUS_SHA,
        **{
            args.model / CACHE / name: pins[constant]
            for name, constant in MODEL.items()
        },
    }
    actual = {path: sha(path) for path in expected}
    if actual != expected:
        raise ValueError("selected store, queries, corpus or pinned model mismatch")
    dump(
        args.out / "inputs.json",
        {
            "inputs": {str(path): digest for path, digest in actual.items()},
            "artifacts": {
                str(path): sha(path) for path in artifact_paths if path is not None
            },
            "host": os.uname(),
            "context": context(),
            "artifact_path_roles": {
                "ordinary": str(args.ordinary),
                "diagnostic": str(args.diagnostic),
            },
        },
    )
    study = Study(args)
    if args.stage == "baseline":
        study.baseline()
    elif args.stage == "written-reopen-regression":
        study.written_reopen_regression()
    elif args.stage == "comparison":
        study.comparison()
    else:
        study.natural_fidelity()
    dump(args.out / "failures.json", study.failures)
    return int(bool(study.failures))


if __name__ == "__main__":
    raise SystemExit(main())
