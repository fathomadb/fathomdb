"""Owned preparation/copy stage; executes inside the GD child group and clocks."""

import argparse
import json
from pathlib import Path


def run(config,out):
    """Dispatch an owned stage without changing original cell/attempt endpoints."""
    if config["family"]!="G_PREPARE":
        raise ValueError("unsupported owned preparation stage")
    out.mkdir(parents=True,exist_ok=True)


def main():
    """Read the controller's cell and output paths; never start a detached child."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config",type=Path)
    parser.add_argument("output",type=Path)
    args=parser.parse_args()
    run(json.loads(args.config.read_text()),args.output)


if __name__=="__main__":
    main()
