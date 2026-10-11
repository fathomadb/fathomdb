"""Explicitly acquire one SHA-pinned, multi-GB vec1bench source corpus.

No network request is made unless ``--allow-network`` is provided and the
requested local source file is absent. All downloads are verified before use.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import urllib.request
from pathlib import Path

from scripts.vec1bench.prepare import load_specs, sha256_file


def acquire_dataset(dataset_id: str, output_root: Path, *, allow_network: bool) -> Path:
    """Return a SHA-verified local HDF5 path or explicitly download it."""
    spec = load_specs()[dataset_id]
    destination = output_root / "raw" / spec["filename"]
    if destination.exists():
        observed = sha256_file(destination)
        if observed != spec["sha256"]:
            raise ValueError(f"source SHA256 mismatch: expected {spec['sha256']}, got {observed}")
        return destination
    if not allow_network:
        raise FileNotFoundError(
            f"source corpus absent at {destination}; pass --allow-network to fetch it explicitly"
        )

    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".part")
    digest = hashlib.sha256()
    request = urllib.request.Request(spec["url"], headers={"User-Agent": "fathomdb-vec1bench/1"})
    with urllib.request.urlopen(request, timeout=60) as response, temporary.open("wb") as stream:
        while chunk := response.read(8 * 1024 * 1024):
            stream.write(chunk)
            digest.update(chunk)
    observed = digest.hexdigest()
    if observed != spec["sha256"]:
        raise ValueError(
            f"download SHA256 mismatch: expected {spec['sha256']}, got {observed}; "
            f"untrusted partial file remains at {temporary}"
        )
    os.replace(temporary, destination)
    return destination


def main() -> None:
    """Fetch one source only after an explicit network opt-in."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dataset", required=True, choices=sorted(load_specs()))
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--allow-network", action="store_true")
    args = parser.parse_args()
    print(acquire_dataset(args.dataset, args.output_root, allow_network=args.allow_network))


if __name__ == "__main__":
    main()
