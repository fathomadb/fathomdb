#!/usr/bin/env python3
# Final process: sanitize inherited descriptors before validation and direct exec.
import importlib
import os
import stat
import sys


def sanitize_descriptors():
    for fd in range(3):
        try:
            kind = os.fstat(fd).st_mode
        except OSError:
            os._exit(126)
        if not (stat.S_ISREG(kind) or stat.S_ISFIFO(kind) or stat.S_ISCHR(kind)):
            if fd != 2 and not stat.S_ISSOCK(os.fstat(2).st_mode):
                os.write(2, b"refusing nonlocal/socket standard stream\n")
            os._exit(126)
    closed = []
    for name in os.listdir("/proc/self/fd"):
        fd = int(name)
        if fd >= 3:
            try:
                kind = os.fstat(fd).st_mode
                os.close(fd)
                closed.append(
                    {
                        "fd": fd,
                        "socket": stat.S_ISSOCK(kind),
                        "regular": stat.S_ISREG(kind),
                    }
                )
            except OSError:
                pass  # /proc enumeration's descriptor has already closed.
    for name in os.listdir("/proc/self/fd"):
        fd = int(name)
        if fd >= 3:
            try:
                os.fstat(fd)
            except OSError:
                continue
            os._exit(126)

    return closed


closed_inherited = sanitize_descriptors()
csv, hashlib, json, pathlib, subprocess = map(
    importlib.import_module, ["csv", "hashlib", "json", "pathlib", "subprocess"]
)
print(json.dumps({"closed_inherited_descriptors": closed_inherited}), flush=True)
mode, host, binary, run_root = sys.argv[1:]
current = os.readlink("/proc/self/ns/net")
assert current != host, "host network namespace forbidden"
links = json.loads(subprocess.check_output(["ip", "-j", "link", "show"]))
assert (
    len(links) == 1 and links[0]["ifname"] == "lo" and "UP" not in links[0]["flags"]
), links
for args in [[], ["-6"]]:
    routes = json.loads(
        subprocess.check_output(["ip", "-j", *args, "route", "show", "table", "all"])
    )
    assert routes == [], routes
print(
    json.dumps(
        {
            "netns": current,
            "host_netns": host,
            "links": links,
            "ipv4_routes": [],
            "ipv6_routes": [],
            "cuda_visible_devices": os.environ["CUDA_VISIBLE_DEVICES"],
            "uid_map": pathlib.Path("/proc/self/uid_map").read_text(),
        }
    ),
    flush=True,
)
if mode == "run":
    root = (
        pathlib.Path(os.environ["XDG_CACHE_HOME"]) / "fathomdb/embedders/0b2926f8a9b1"
    )
    pins = [
        (
            "config.json",
            "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
        ),
        (
            "tokenizer.json",
            "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
        ),
        (
            "model.safetensors",
            "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
        ),
    ]
    for name, want in pins:
        path = root / name
        assert path.is_file() and not path.is_symlink()
        assert hashlib.sha256(path.read_bytes()).hexdigest() == want, name
    output = subprocess.check_output(
        [
            "nvidia-smi",
            "--query-gpu=uuid,name,utilization.gpu,memory.free,driver_version",
            "--format=csv,noheader,nounits",
        ],
        text=True,
    )
    rows = [[part.strip() for part in row] for row in csv.reader(output.splitlines())]
    selected = [row for row in rows if row[0] == os.environ["CUDA_VISIBLE_DEVICES"]]
    assert len(selected) == 1
    row = selected[0]
    assert (
        "3090" in row[1]
        and int(row[2]) <= 5
        and int(row[3]) >= 20480
        and row[4] == "580.178.04"
    ), row
    apps = subprocess.check_output(
        [
            "nvidia-smi",
            "--query-compute-apps=gpu_uuid,pid,used_gpu_memory",
            "--format=csv,noheader,nounits",
        ],
        text=True,
    )
    assert not any(
        line.split(",")[0].strip() == row[0] for line in apps.splitlines()
    ), apps
    print(output, flush=True)
    assert pathlib.Path("/usr/local/cuda").resolve() == pathlib.Path(
        "/usr/local/cuda-12.6"
    )
else:
    assert mode == "probe"

sys.stdout.flush()
sys.stderr.flush()
sanitize_descriptors()
os.execv(binary, [binary, "produce", run_root])
