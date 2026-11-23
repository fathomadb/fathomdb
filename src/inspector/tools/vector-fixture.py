"""Build an ordinary pinned CPU vector store using the checkout-owned native wheel.

The caller owns the empty directory. Existing cached files are copied into sealed
memory; the production loader cannot write its private view or create sockets.
Two ordinary Default sessions establish the Engine-owned equivalence baseline.
"""

import ctypes
import os
import fcntl
import hashlib
import pathlib
import sys
import socket
import json
import subprocess
from fathomdb import Engine, EngineConfig, ProjectionSpec, ProjectionRole

root = pathlib.Path(sys.argv[1]).resolve()
cache = root / "snapshot" / "fathomdb" / "embedders" / "0b2926f8a9b1"
cache.mkdir(parents=True)
work = root / "work"
work.mkdir(exist_ok=True)
fds = []
project = pathlib.Path(__file__).resolve().parents[3]
description = json.loads(
    subprocess.check_output(
        [str(project / "target/debug/fathomdb-inspector"), "--build-description"],
        env={},
    )
)
pins = description["assets"]
source_cache = (
    pathlib.Path(os.environ.get("XDG_CACHE_HOME", str(pathlib.Path.home() / ".cache")))
    / "fathomdb"
    / "embedders"
    / "0b2926f8a9b1"
)
libc = ctypes.CDLL(None, use_errno=True)
MFD_ALLOW_SEALING = getattr(os, "MFD_ALLOW_SEALING", 2)
F_ADD_SEALS = getattr(fcntl, "F_ADD_SEALS", 1033)
F_SEAL_SEAL = getattr(fcntl, "F_SEAL_SEAL", 1)
F_SEAL_SHRINK = getattr(fcntl, "F_SEAL_SHRINK", 2)
F_SEAL_GROW = getattr(fcntl, "F_SEAL_GROW", 4)
F_SEAL_WRITE = getattr(fcntl, "F_SEAL_WRITE", 8)


def _memfd_create(name: str, flags: int) -> int:
    if hasattr(os, "memfd_create"):
        return os.memfd_create(name, flags)
    # Standalone Linux Python builds compiled against an older glibc sysroot may omit
    # os.memfd_create even though the host x86_64 Linux kernel supports SYS_memfd_create (319).
    fd = libc.syscall(319, ctypes.c_char_p(name.encode("utf-8")), ctypes.c_uint(flags))
    if fd < 0:
        err = ctypes.get_errno()
        raise OSError(err, os.strerror(err))
    return int(fd)


for name, pin in pins.items():
    fd = _memfd_create(name, MFD_ALLOW_SEALING)
    fds.append(fd)
    with open(source_cache / name, "rb") as source:
        while data := source.read(65536):
            os.write(fd, data)
    fcntl.fcntl(
        fd,
        F_ADD_SEALS,
        F_SEAL_WRITE | F_SEAL_GROW | F_SEAL_SHRINK | F_SEAL_SEAL,
    )
    os.lseek(fd, 0, 0)
    h = hashlib.sha256()
    while data := os.read(fd, 65536):
        h.update(data)
    assert h.hexdigest() == pin
    (cache / name).symlink_to("/proc/self/fd/" + str(fd))
rights = sum(1 << n for n in [1, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14])


class Rule(ctypes.Structure):
    _fields_ = [("rights", ctypes.c_uint64), ("fd", ctypes.c_int32)]
    _pack_ = 1


class Set(ctypes.Structure):
    _fields_ = [("rights", ctypes.c_uint64)]


assert libc.syscall(444, 0, 0, 1) >= 3
ruleset = Set(rights)
rfd = libc.syscall(444, ctypes.byref(ruleset), ctypes.sizeof(ruleset), 0)
assert rfd >= 0, ctypes.get_errno()
wfd = os.open(work, os.O_PATH)
rule = Rule(rights, wfd)
assert libc.syscall(445, rfd, 1, ctypes.byref(rule), 0) == 0, ctypes.get_errno()
assert libc.prctl(38, 1, 0, 0, 0) == 0
assert libc.syscall(446, rfd, 0) == 0, ctypes.get_errno()


class Ins(ctypes.Structure):
    _fields_ = [
        ("code", ctypes.c_ushort),
        ("jt", ctypes.c_ubyte),
        ("jf", ctypes.c_ubyte),
        ("k", ctypes.c_uint32),
    ]


class Prog(ctypes.Structure):
    _fields_ = [("len", ctypes.c_ushort), ("filter", ctypes.POINTER(Ins))]


# x86_64: forbid socket creation (all domains); inherited worker IPC remains usable.
filt = (Ins * 4)(
    Ins(0x20, 0, 0, 0),
    Ins(0x15, 0, 1, 41),
    Ins(0x06, 0, 0, 0x00050000 | 13),
    Ins(0x06, 0, 0, 0x7FFF0000),
)
prog = Prog(4, filt)
assert libc.prctl(22, 2, ctypes.byref(prog), 0, 0) == 0, ctypes.get_errno()
try:
    socket.socket()
    raise AssertionError("socket allowed")
except PermissionError:
    pass
try:
    open(cache / "repair", "w")
    raise AssertionError("write allowed")
except PermissionError:
    pass
os.environ.clear()
os.environ.update(
    XDG_CACHE_HOME=str(root / "snapshot"),
    TMPDIR=str(work),
    FATHOMDB_EMBED_DEVICE="cpu",
    RAYON_NUM_THREADS="1",
)
e = Engine.open(
    str(work / "fixture.sqlite"),
    use_default_embedder=True,
    config=EngineConfig(
        physical_erasure_policy="explicit_maintenance",
        scheduler_runtime_threads=1,
        embedder_pool_size=1,
        embedder_call_timeout_ms=25000,
    ),
)
e.configure_projections(
    [
        ProjectionSpec(
            name="summary", roles=frozenset({ProjectionRole.SEARCHABLE}), vector=True
        )
    ]
)
e.write(
    [
        {
            "kind": "doc",
            "logical_id": "dog",
            "source_id": "fixture:vector",
            "body": '{"summary":"a domestic dog playing with a ball"}',
        },
        {
            "kind": "doc",
            "logical_id": "planet",
            "source_id": "fixture:vector",
            "body": '{"summary":"an icy planet orbiting a distant star"}',
        },
    ]
)
e.drain(timeout_s=25)
e.close()
e = Engine.open(
    str(work / "fixture.sqlite"),
    use_default_embedder=True,
    config=EngineConfig(
        physical_erasure_policy="explicit_maintenance",
        scheduler_runtime_threads=1,
        embedder_pool_size=1,
        embedder_call_timeout_ms=25000,
    ),
)
result = e.search("puppy", explain=True)
assert (
    result.explanation.trace.vector_hits > 0 and result.explanation.trace.text_hits == 0
)
dangling_ids = []
if len(sys.argv) > 2 and sys.argv[2] == "person":
    from person_fixture import add_people

    dangling_ids.extend(add_people(e) or [])
if len(sys.argv) > 2 and sys.argv[2] in ("study-training", "study-unseen"):
    from study_fixture import add_study

    dangling_ids.extend(add_study(e, sys.argv[2].removeprefix("study-")) or [])
if len(sys.argv) > 2 and sys.argv[2] == "history-queued":
    e.write(
        [
            {
                "kind": "doc",
                "logical_id": "dog",
                "source_id": "fixture:vector",
                "body": '{"summary":"retained second dog revision"}',
            }
        ]
    )
    e.drain(timeout_s=25)
if len(sys.argv) > 2 and sys.argv[2] in ("queued", "history-queued"):
    maintenance = e.begin_erasure_maintenance(1000)
    queued = maintenance.queue_physical_erasure(
        {"kind": "source_all_at_cut", "source_id": "fixture:vector"}, "40" * 16
    )
    assert queued["outcome"] == "accepted"
    maintenance.close()
e.close()
if dangling_ids:
    import sqlite3

    conn = sqlite3.connect(str(work / "fixture.sqlite"))
    try:
        with conn:
            cursors = [
                row[0]
                for logical_id in dangling_ids
                for row in conn.execute(
                    "SELECT write_cursor FROM canonical_nodes WHERE logical_id = ?",
                    (logical_id,),
                ).fetchall()
            ]
            for cursor in cursors:
                conn.execute("DELETE FROM search_index WHERE write_cursor = ?", (cursor,))
                conn.execute(
                    "DELETE FROM _fathomdb_artifact_revisions WHERE artifact_class = 'node' AND write_cursor = ?",
                    (cursor,),
                )
                conn.execute(
                    "DELETE FROM _fathomdb_projection_terminal WHERE write_cursor = ?",
                    (cursor,),
                )
                conn.execute(
                    "DELETE FROM canonical_nodes WHERE write_cursor = ?", (cursor,)
                )
    finally:
        conn.close()
print(work / "fixture.sqlite", flush=True)
