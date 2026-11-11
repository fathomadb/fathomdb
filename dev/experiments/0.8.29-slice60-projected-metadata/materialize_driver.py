"""Materialize the manual SDK driver only inside fresh task-owned resources."""

import hashlib
import json
import os
import pathlib
import shutil
import sys
import tomllib

source = pathlib.Path(os.environ["S60_SOURCE"]).resolve(strict=True)
task = pathlib.Path(os.environ["S60_TASK"]).resolve(strict=True)
operators = pathlib.Path(__file__).resolve().parent
destination = pathlib.Path(sys.argv[1]).resolve()
assert destination.is_relative_to(task / "gpu-metadata")
assert not destination.exists(), "fresh task-owned destination required"
archive = json.loads((operators / "executed-sources.json").read_text())["sources"]
original = archive["Cargo.toml"]
text = original["utf8_source"]
assert (
    hashlib.sha256(text.encode()).hexdigest()
    == original["sha256"]
    == "284d71484fb7063d7be342b5259fdceab4392837172ed396cacc1a4f067d50fe"
)
manifest = tomllib.loads(text)
root_manifest = tomllib.loads((source / "Cargo.toml").read_text())
patch = root_manifest["patch"]["crates-io"]
expected = {
    "candle-kernels": {
        "git": "https://github.com/coreyt/candle-fathomdb.git",
        "rev": "182ceab0547348993b6dbec4b1aed610dcc8b23f",
    }
}
assert patch == expected == manifest["patch"]["crates-io"], (
    "only the exact governed root patch is permitted"
)
sdk = (source / "src/rust/crates/fathomdb-sdk").resolve(strict=True)
assert sdk.is_relative_to(source)
old_sdk = manifest["dependencies"]["fathomdb-sdk"]["path"]
assert text.count(old_sdk) == 1
text = text.replace(old_sdk, str(sdk))
lock = operators / "driver/Cargo.lock"
assert (
    hashlib.sha256(lock.read_bytes()).hexdigest()
    == "99110175134552542ddc4200d9bfb8cee892706ef760cfa375efbf923c63e543"
)
root_packages = tomllib.loads((source / "Cargo.lock").read_text())["package"]
packages = tomllib.loads(lock.read_text())["package"]


def identity(package):
    return package["name"], package["version"], package.get("source")


root_packages = {identity(package): package for package in root_packages}
added = []
for package in packages:
    if package["name"] == "s60-manual-metadata-acceptance":
        assert package["version"] == "0.0.0" and "source" not in package
        added.append(package["name"])
        continue
    assert identity(package) in root_packages
    assert package.get("checksum") == root_packages[identity(package)].get("checksum")
assert added == ["s60-manual-metadata-acceptance"]
(destination / "src").mkdir(parents=True, exist_ok=False)
(destination / "Cargo.toml").write_text(text)
shutil.copyfile(operators / "driver/src/main.rs", destination / "src/main.rs")
shutil.copyfile(lock, destination / "Cargo.lock")
print(
    json.dumps(
        {
            "private_manifest": str(destination / "Cargo.toml"),
            "exact_root_patch": expected,
            "root_lock_sha256": hashlib.sha256(
                (source / "Cargo.lock").read_bytes()
            ).hexdigest(),
            "private_lock_sha256": hashlib.sha256(lock.read_bytes()).hexdigest(),
            "retained_packages": len(packages) - 1,
        }
    )
)
