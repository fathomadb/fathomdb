"""Owned preparation/copy stage; executes inside the GD child group and clocks."""

import argparse
import json
import os
import time
from pathlib import Path

from g_inputs import encode_queries, encode_records


def run(config,out):
    """Dispatch an owned stage without changing original cell/attempt endpoints."""
    if config["family"]=="G1_CELL" and config.get("template_from"):
        out.mkdir(parents=True,exist_ok=True)
        return
    if config["family"]!="G_PREPARE":
        raise ValueError("unsupported owned preparation stage")
    out.mkdir(parents=True,exist_ok=True)
    deadline=min(config[key] for key in ["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])

    def admitted(phase):
        now=time.monotonic_ns()
        if now<deadline:
            return True
        (out/"budget.json").write_text(json.dumps({"phase":phase,"disposition":"deadline_refused","observed_at_ns":now})+"\n")
        return False

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
