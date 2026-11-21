if not __debug__:
    raise RuntimeError(
        "qualification requires assertions enabled; optimized execution refused"
    )
import ctypes as C
import hashlib, json, math, os, pathlib, platform
import fathomdb
from fathomdb import Engine, embed_batch_cls, rerank, release_models
from fathomdb import _fathomdb as native
from fathomdb.errors import CudaContextLostError

actual_native = pathlib.Path(native.__file__).resolve()
assert actual_native == pathlib.Path(os.environ["FATHOMDB_PY_NATIVE"]).resolve()
assert (
    hashlib.sha256(actual_native.read_bytes()).hexdigest()
    == os.environ["FATHOMDB_PY_NATIVE_SHA256"]
)

mode = os.environ["FATHOMDB_COORDINATED_RESET"]
assert os.environ["FATHOMDB_REQUIRE_LIVE"] == "1" and mode in (
    "reset-refuse",
    "recover",
)
assert (
    os.environ["FATHOMDB_EMBED_DEVICE"] == "cuda:0"
    and os.environ["FATHOMDB_RERANK_DEVICE"] == "cuda:0"
)
assert os.environ["CUDA_VISIBLE_DEVICES"] == os.environ["FATHOMDB_SELECTED_UUID"]
root = pathlib.Path(os.environ["FATHOMDB_CAPTURE_DIR"])
root.mkdir(parents=True, exist_ok=True)
db = root / "installed-coordinate.fdb"
assert db.exists() if mode == "recover" else not db.exists()
Q = "What is the capital of France?"
P = [
    "Paris.",
    "Paris is the capital and most populous city of France.",
    "France is a country in Western Europe; its capital and largest city is Paris, a major global center for art, fashion, and culture.",
]
passages = [{"id": i, "body": p, "score": 1.0} for i, p in enumerate(P)]
inputs = [
    "the cat sat on the mat",
    "a cat is on a mat",
    "quantum mechanics dictates the behaviour of subatomic particles",
]
engine = Engine.open(str(db), use_default_embedder=True)
resolution = engine.open_report().embedder_device_resolution
assert (
    resolution.effective_device.kind == "cuda"
    and resolution.selected_cuda_uuid == os.environ["FATHOMDB_SELECTED_UUID"]
)
vectors = [engine.embed(x) for x in inputs]
for v in vectors:
    assert len(v) == 384 and abs(math.sqrt(sum(x * x for x in v)) - 1) < 1e-5


def cos(a, b):
    return sum(x * y for x, y in zip(a, b))


near = cos(vectors[0], vectors[1])
assert (
    near - cos(vectors[0], vectors[2]) > 0.15
    and near - cos(vectors[1], vectors[2]) > 0.15
)
batch = embed_batch_cls(inputs)
assert len(batch) == 3 and all(len(v) == 384 for v in batch)
scores = rerank(Q, passages, 3)
assert len(scores) == 3 and all(math.isfinite(x["ce_score"]) for x in scores)
if mode != "recover":
    engine.write(
        [
            {
                "kind": "doc",
                "body": "Paris is the capital of France.",
                "source_id": "coordinate-paris",
            },
            {
                "kind": "doc",
                "body": "France has its capital in Paris.",
                "source_id": "coordinate-france",
            },
        ]
    )
engine.drain(timeout_s=30)
hits = engine.search(Q, rerank_depth=2, limit=2)
assert len(hits.results) == 2 and all(
    x.ce_score is not None and math.isfinite(x.ce_score) for x in hits.results
)
engine.drain(timeout_s=30)
if mode == "recover":
    assert {
        (x.source_id, x.body) for x in engine.search_text_only("Paris").results
    } == {
        ("coordinate-paris", "Paris is the capital of France."),
        ("coordinate-france", "France has its capital in Paris."),
    }
    release_models()
    release_models()
    assert len(embed_batch_cls(inputs)) == 3
    release_models()
    engine.close()
    engine = None
    print("INSTALLED-COORDINATE fresh-process-recovery post-drop", flush=True)
    raise SystemExit(0)

# Keep this handle until process exit. No CUDA constructor or availability probe:
# FathomDB has already completed all construction/forwards and drained workers.
driver = C.CDLL("libcuda.so.1")


def function(name, types):
    fn = getattr(driver, name)
    fn.argtypes = types
    fn.restype = C.c_int
    return fn


def check(status, phase):
    if status:
        raise RuntimeError(f"actor phase={phase} actual-status={status}")


getdev = function("cuDeviceGet", [C.POINTER(C.c_int), C.c_int])
state = function(
    "cuDevicePrimaryCtxGetState", [C.c_int, C.POINTER(C.c_uint), C.POINTER(C.c_int)]
)
retain = function("cuDevicePrimaryCtxRetain", [C.POINTER(C.c_void_p), C.c_int])
release = function("cuDevicePrimaryCtxRelease_v2", [C.c_int])
getcurrent = function("cuCtxGetCurrent", [C.POINTER(C.c_void_p)])
setcurrent = function("cuCtxSetCurrent", [C.c_void_p])
getid = function("cuCtxGetId", [C.c_void_p, C.POINTER(C.c_ulonglong)])
sync = function("cuCtxSynchronize", [])
reset = function("cuDevicePrimaryCtxReset_v2", [C.c_int])
getuuid = function("cuDeviceGetUuid_v2", [C.c_void_p, C.c_int])
dev = C.c_int()
check(getdev(C.byref(dev), 0), "device")
uuid = (C.c_ubyte * 16)()
check(getuuid(uuid, dev), "uuid")
h = bytes(uuid).hex()
actual_uuid = (
    "GPU-" + h[:8] + "-" + h[8:12] + "-" + h[12:16] + "-" + h[16:20] + "-" + h[20:]
)
assert actual_uuid == os.environ["FATHOMDB_SELECTED_UUID"]
flags = C.c_uint()
active = C.c_int()
check(state(dev, C.byref(flags), C.byref(active)), "primary-state")
assert active.value == 1
previous = C.c_void_p()
check(getcurrent(C.byref(previous)), "previous-context")
context = C.c_void_p()
identifier = C.c_ulonglong()
problem = None
released = None
check(retain(C.byref(context), dev), "retain-existing-primary")
try:
    check(getid(context, C.byref(identifier)), "context-id")
    assert previous.value is None or previous.value == context.value, (
        "prior context differs from retained primary"
    )
    check(setcurrent(context), "bind-existing-primary")
    check(sync(), "all-streams-synchronize")
    before = json.loads(native._cuda_pool_observation_for_qualification())
    assert platform.machine() in ("aarch64", "x86_64")
    if platform.machine() == "aarch64":
        assert before["status"] == "private", before
    if before["status"] == "private":
        assert str(before["context_id"]) == str(identifier.value)
        assert int(before["live_models_after"]) == 3, before
        assert (
            int(before["used_current_bytes"])
            <= int(before["configured_max_size_bytes"])
            == 3 << 30
        )
    print(
        "INSTALLED-COORDINATE READY "
        + json.dumps(
            {
                "mode": mode,
                "pid": os.getpid(),
                "uuid": actual_uuid,
                "context_id": str(identifier.value),
                "pool": before,
                "native_sha256": hashlib.sha256(
                    pathlib.Path(native.__file__).read_bytes()
                ).hexdigest(),
            }
        ),
        flush=True,
    )
    if mode == "healthy":
        check(setcurrent(previous), "healthy-restore-prior")
    elif mode == "reset-refuse":
        check(reset(dev), "process-primary-reset")
    else:
        raise RuntimeError("explicit coordinated mode required")
except BaseException as error:
    problem = error
finally:
    released = release(dev)
if problem is not None:
    print("actor-error balanced-release-actual-status=" + str(released), flush=True)
    raise problem
check(released, "balanced-primary-release")
release_models()
release_models()
print("RELEASE_AFTER_LOSS_OK", flush=True)
print("INSTALLED-COORDINATE coordinated " + mode, flush=True)
if mode == "healthy":
    assert len(engine.embed(inputs[0])) == 384
    assert len(embed_batch_cls(inputs)) == 3
    assert all(math.isfinite(x["ce_score"]) for x in rerank(Q, passages, 3))
elif mode == "reset-refuse":
    first = None
    for _ in range(1):
        for call in (
            lambda: engine.embed("after reset"),
            lambda: embed_batch_cls(inputs),
            lambda: rerank(Q, passages, 3),
        ):
            try:
                call()
            except BaseException as error:
                assert isinstance(error, CudaContextLostError), (
                    type(error),
                    repr(error),
                )
                cause = (
                    str(error.recorded_context_id),
                    error.current_context_id,
                    error.driver_error,
                )
                assert cause[0] == str(identifier.value), (cause, identifier.value)
                if first is None:
                    first = cause
                assert cause == first, (cause, first)
            else:
                raise AssertionError("lost generation unexpectedly forwarded")
    release_models()
    release_models()
    try:
        embed_batch_cls(inputs)
    except CudaContextLostError as error:
        assert (
            str(error.recorded_context_id),
            error.current_context_id,
            error.driver_error,
        ) == first
    else:
        raise AssertionError("loss fence cleared")
    print("INSTALLED-COORDINATE typed-refusal " + json.dumps(first), flush=True)
engine.close()
engine = None
after = json.loads(native._cuda_pool_observation_for_qualification())
if before["status"] == "private":
    assert (
        after["context_id"] == before["context_id"]
        and after["pool_id"] == before["pool_id"]
    )
    assert int(after["live_models_after"]) == 0
    if mode != "healthy":
        assert (
            after["status"] == "lost"
            and after["used_current_bytes"] is None
            and after["reserved_current_bytes"] is None
        )
print(
    "INSTALLED-COORDINATE POST-DROP "
    + json.dumps({"mode": mode, "pool": after, "released_model_owners": 0}),
    flush=True,
)
