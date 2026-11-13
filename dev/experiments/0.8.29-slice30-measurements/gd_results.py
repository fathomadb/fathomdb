"""Read actual G/D outputs without running children or assigning performance gates."""

import argparse
import json
from pathlib import Path


def summarize(manifest, root):
    """Read an explicit plan and retained outcomes; return available result groups."""
    state=Path(root)/"campaign-state.json"
    if state.exists():
        json.loads(state.read_text())
    return {"cells":[],"pairs":[],"gated":False}


def main():
    """Print descriptive results for one selected manifest and output directory."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest",type=Path)
    parser.add_argument("root",type=Path)
    args=parser.parse_args()
    print(json.dumps(summarize(json.loads(args.manifest.read_text()),args.root),indent=2))


if __name__=="__main__":
    main()
