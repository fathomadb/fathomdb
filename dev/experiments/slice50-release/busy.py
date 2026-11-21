import concurrent.futures, threading, time
from fathomdb import embed_batch_cls, release_models
from fathomdb.errors import OverloadedError

inputs = ["the cat sat on the mat"] * 32
expected = embed_batch_cls(inputs)
started = threading.Event()


def infer():
    started.set()
    return embed_batch_cls(inputs)


with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
    future = pool.submit(infer)
    assert started.wait(10), "worker did not start"
    deadline = time.monotonic() + 30
    attempts = 0
    busy = False
    while not future.done() and time.monotonic() < deadline and attempts < 10000:
        attempts += 1
        try:
            release_models()
        except OverloadedError:
            assert not future.done(), "worker finished before Busy observation"
            busy = True
            break
    assert busy, f"no concurrent typed Busy after {attempts} attempts"
    actual = future.result(timeout=60)
    assert actual == expected
release_models()
assert embed_batch_cls(inputs) == expected
release_models()
print(f"BINDING_BUSY_OK attempts={attempts}", flush=True)
