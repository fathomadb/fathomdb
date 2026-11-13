"""Owned preparation/copy stage; executes inside the GD child group and clocks."""

import argparse
import hashlib
import json
import os
import shutil
import time
from pathlib import Path

from g_inputs import encode_queries, encode_records
from d_inputs import records as d_records


def run(config,out):
    """Dispatch an owned stage without changing original cell/attempt endpoints."""
    out.mkdir(parents=True,exist_ok=True)
    deadline=min(config[key] for key in ["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])

    def admitted(phase):
        now=time.monotonic_ns()
        if now<deadline:
            return True
        (out/"budget.json").write_text(json.dumps({"phase":phase,"disposition":"deadline_refused","observed_at_ns":now})+"\n")
        return False

    if config["family"]=="D_QUERY_FIXTURE" and "n" in config and "write_records" not in config:
        population=[]
        for index,row in enumerate(d_records(config["n"])):
            if index%1024==0 and not admitted("generate_population"):
                return
            population.append(row)
        native=dict(config,write_records=population)
    elif config["family"]=="G_PREPARE":
        if not admitted("encode_corpus"):
            return
        records=out/"records.jsonl"
        record_count=encode_records(config["corpus"],records)["records"]
        if record_count!=config["n"]:
            raise ValueError(f"selected corpus count {record_count} differs from declared {config['n']}")
        if not admitted("encode_questions"):
            return
        questions=out/"questions.jsonl"
        question_count=encode_queries(config["questions"],questions)["records"]
        if question_count!=config.get("query_count",500):
            raise ValueError("selected question population does not match the declared cell")
        native=dict(config,family="G_TEMPLATE",templates=[{"records":str(records)}],
                    encoded_inputs={"records":record_count,"questions":question_count})
    elif config["family"]=="G1_CELL" and config.get("template_from"):
        if not admitted("template_source"):
            return
        state={row["id"]:row for row in json.loads((Path(config["campaign_root"])/"campaign-state.json").read_text())}
        prepared=state[config["template_from"]]
        questions=state[config["queries_from"]]
        if prepared["status"]!="complete" or questions["status"]!="complete":
            raise ValueError("dependent cell requires completed closed preparation")
        source=Path(prepared["attempt"])/"case-0/template.sqlite"
        metadata=json.loads((source.parent/"template.json").read_text())
        if metadata["preparation"]["write_rows"]!=config["n"]:
            raise ValueError("prepared template population differs from dependent cell")
        wal=Path(str(source)+"-wal")
        if wal.exists() and wal.stat().st_size:
            raise ValueError("prepared source has nonempty WAL")
        started=time.monotonic_ns()
        source_hash=hashlib.sha256(source.read_bytes()).hexdigest()
        source_hash_ns=time.monotonic_ns()-started
        if source_hash!=metadata["closed_main_sha256"]:
            raise ValueError("prepared closed source differs from recorded main file")
        if not admitted("copy_template"):
            return
        store=out/"store.sqlite"
        if store.exists():
            raise ValueError("dependent copy must be fresh")
        started=time.monotonic_ns()
        shutil.copyfile(source,store)
        copy_ns=time.monotonic_ns()-started
        if not admitted("verify_copy"):
            return
        started=time.monotonic_ns()
        copy_hash=hashlib.sha256(store.read_bytes()).hexdigest()
        copy_hash_ns=time.monotonic_ns()-started
        if copy_hash!=source_hash:
            raise ValueError("fresh writable copy differs from closed source")
        question_copy=out/"questions.jsonl"
        shutil.copyfile(Path(questions["attempt"])/"questions.jsonl",question_copy)
        (out/"copy.json").write_text(json.dumps({"source":str(source),"source_main_sha256":source_hash,
            "initial_copy_sha256":copy_hash,"source_hash_ns":source_hash_ns,"copy_ns":copy_ns,"copy_hash_ns":copy_hash_ns})+"\n")
        native=dict(config,store=str(store),queries=str(question_copy))
    else:
        raise ValueError("unsupported owned preparation stage")
    native_path=out/"native-config.json"
    native_path.write_text(json.dumps(native)+"\n")
    if not admitted("prepare_native"):
        return
    binary=config["artifact_identity"]["path"]
    os.execv(binary,[binary,str(native_path),str(out)])


def main():
    """Read the controller's cell and output paths; never start a detached child."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config",type=Path)
    parser.add_argument("output",type=Path)
    args=parser.parse_args()
    run(json.loads(args.config.read_text()),args.output)


if __name__=="__main__":
    main()
