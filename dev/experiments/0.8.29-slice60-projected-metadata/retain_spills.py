import ctypes, hashlib, json, os, pathlib, select, stat, struct, subprocess, sys, threading, time

base = pathlib.Path(os.environ["S60_TASK"]).resolve()
phase = sys.argv[1]
command = sys.argv[2:]
watched = base / "logs" / "spills"
destination = base / "logs" / (phase + "-retained")
destination.mkdir(mode=0o700, exist_ok=False)
opened = {}
missed = []
stopping = threading.Event()


def retain(name):
    if not name.startswith("fathomdb-agent-") or not name.endswith(".log"):
        return
    try:
        fd = os.open(watched / name, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode):
            os.close(fd)
            raise RuntimeError("nonregular spill")
        identity = (info.st_dev, info.st_ino)
        if identity in opened:
            os.close(fd)
        else:
            opened[identity] = (name, fd, time.time())
    except FileNotFoundError:
        missed.append(name)


def observe():
    while not stopping.is_set():
        for entry in os.scandir(watched):
            if entry.name.startswith("fathomdb-agent-") and entry.name.endswith(".log"):
                try:
                    info = entry.stat(follow_symlinks=False)
                    if (info.st_dev, info.st_ino) not in opened:
                        retain(entry.name)
                except FileNotFoundError:
                    pass
        stopping.wait(0.002)


thread = threading.Thread(target=observe)
thread.start()
started = time.time()
try:
    child = subprocess.run(command, cwd=os.environ["S60_SOURCE"])
    code = child.returncode
finally:
    stopping.set()
    thread.join()
records = []
for (device, inode), (name, fd, when) in opened.items():
    path = destination / (str(inode) + "-" + name)
    digest = hashlib.sha256()
    total = 0
    with path.open("xb") as target:
        while True:
            data = os.pread(fd, 1024 * 1024, total)
            if not data:
                break
            target.write(data)
            digest.update(data)
            total += len(data)
    os.close(fd)
    records.append(
        dict(
            original=name,
            path=str(path),
            device=device,
            inode=inode,
            bytes=total,
            sha256=digest.hexdigest(),
            opened=when,
        )
    )
receipt = dict(
    phase=phase,
    command=command,
    started=started,
    ended=time.time(),
    exit=code,
    method="2ms read-only owned-directory polling plus retained file descriptors across canonical unlink; no inotify/global-setting change",
    missed_before_open=missed,
    records=records,
)
(base / "receipts" / (phase + "-retained.json")).write_text(
    json.dumps(receipt, indent=2) + "\n"
)
print(
    json.dumps(dict(retained_spills=len(records), missed=missed, exit=code)), flush=True
)
raise SystemExit(code if code >= 0 else 128 - code)
