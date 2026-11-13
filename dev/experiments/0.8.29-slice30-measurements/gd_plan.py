"""Generate the fixed G/D campaign catalog; generation never launches preparation."""

import argparse
import json
from pathlib import Path


def build(inputs):
    """Return descriptors for pinned input/artifact references, without opening them."""
    return {"schema":"slice30-gd/catalog-v1", "campaign_seconds":14400,"cell_seconds":900,
            "artifacts":inputs["artifacts"],"resources":inputs["resources"],"cells":[]}


def main():
    """Read input references and print the catalog; all execution is separate."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs",type=Path)
    args=parser.parse_args()
    print(json.dumps(build(json.loads(args.inputs.read_text())),indent=2))


if __name__=="__main__":
    main()
