import json, os, pathlib, socket, subprocess

root = pathlib.Path(__file__).resolve().parent
output = pathlib.Path(os.environ["S60_TASK"]).resolve() / "gpu-metadata/fd-probes"
output.mkdir(exist_ok=False)
host = os.readlink("/proc/self/ns/net")
command = [
    "unshare",
    "-rUn",
    "--",
    "bash",
    str(root / "netns_exec.sh"),
    "probe",
    host,
    str(root / "netns_probe_target.py"),
]
results = []
for case in ["inherited-fds", "socket-stdin", "socket-stdout", "socket-stderr"]:
    run = output / f"probe-v3-{case}"
    run.mkdir(exist_ok=False)
    left, right = socket.socketpair()
    with (
        (run / "seed.txt").open("w+") as seed,
        open(os.devnull, "rb") as null,
        (run / "stdout.log").open("wb") as out,
        (run / "stderr.log").open("wb") as err,
    ):
        env = os.environ.copy()
        fds = [left.fileno(), seed.fileno()]
        env["S60_PROBE_SEED_FDS"] = json.dumps(fds)
        streams = {"stdin": null, "stdout": out, "stderr": err}
        if case.startswith("socket-"):
            streams[case.removeprefix("socket-")] = left
        child = subprocess.run(
            command + [str(run)], env=env, pass_fds=tuple(fds), **streams
        )
    left.close()
    right.close()
    target = run / "target.json"
    if case == "inherited-fds":
        assert child.returncode == 0, (
            case,
            child.returncode,
            (run / "stderr.log").read_text(),
        )
        report = json.loads(target.read_text())
        assert len(report["seed_fd_results"]) == 2
        first = json.loads((run / "stdout.log").read_text().splitlines()[0])
        closed = {item["fd"]: item for item in first["closed_inherited_descriptors"]}
        assert closed[fds[0]]["socket"] and closed[fds[1]]["regular"], closed
    else:
        assert child.returncode == 126 and not target.exists(), (case, child.returncode)
    results.append(
        {
            "case": case,
            "exit": child.returncode,
            "seed_fds": fds,
            "target_executed": target.exists(),
        }
    )
print(
    json.dumps(
        {
            "command_prefix": command,
            "cases": results,
            "no_model_access_or_compute": True,
        },
        indent=2,
    )
)
(output / "netns-probes-v3-results.json").write_text(
    json.dumps(results, indent=2) + "\n"
)
