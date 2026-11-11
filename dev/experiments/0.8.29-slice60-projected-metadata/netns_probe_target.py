#!/usr/bin/env python3
import errno, json, os, pathlib, subprocess, sys

assert sys.argv[1] == "produce"
root = pathlib.Path(sys.argv[2])
assert root.is_relative_to(
    pathlib.Path(os.environ["S60_TASK"]).resolve() / "gpu-metadata"
)
results = []
for fd in json.loads(os.environ.get("S60_PROBE_SEED_FDS", "[]")):
    try:
        os.fstat(fd)
    except OSError as error:
        assert error.errno == errno.EBADF
        results.append({"fd": fd, "errno": "EBADF"})
    else:
        raise AssertionError(f"inherited descriptor survived: {fd}")
assert os.environ["CUDA_VISIBLE_DEVICES"] == os.environ["S60_GPU_UUID"]
output = subprocess.check_output(
    ["nvidia-smi", "--query-gpu=uuid,name", "--format=csv,noheader"], text=True
)
assert os.environ["S60_GPU_UUID"] in output
result = {
    "target_executed": True,
    "seed_fd_results": results,
    "cuda_visible_devices": os.environ["CUDA_VISIBLE_DEVICES"],
    "nvml": output,
    "netns": os.readlink("/proc/self/ns/net"),
}
(root / "target.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result), flush=True)
