import json, os, math, time
from fathomdb import (
    Engine,
    embed_batch_cls,
    rerank,
    release_models,
    _fathomdb as native,
)


def observe():
    return json.loads(native._cuda_pool_observation_for_qualification())


inputs = [
    "the cat sat on the mat",
    "a cat is on a mat",
    "quantum mechanics dictates the behaviour of subatomic particles",
]
q = "What is the capital of France?"
p = [
    {"id": i, "body": s, "score": 1.0}
    for i, s in enumerate(
        [
            "Paris.",
            "Paris is the capital and most populous city of France.",
            "France is a country in Western Europe; its capital and largest city is Paris, a major global center for art, fashion, and culture.",
        ]
    )
]
release_models()
release_models()
engine = Engine.open(os.environ["FATHOMDB_DB"], use_default_embedder=True)
mean = engine.embed(inputs[0])
assert len(mean) == 384
cls = embed_batch_cls(inputs)
scores = rerank(q, p, 3)
assert len(cls) == 3 and all(math.isfinite(x["ce_score"]) for x in scores)
engine.write(
    [
        {
            "kind": "doc",
            "body": "Paris is the capital of France.",
            "source_id": "release-paris",
        }
    ]
)
engine.drain(timeout_s=30)
before = observe()
assert before["status"] in ("not_built", "private")
if before["status"] == "private":
    assert int(before["live_models_after"]) == 3
release_models()
released = observe()
assert released["status"] == before["status"]
if released["status"] == "private":
    assert int(released["live_models_after"]) == 1 and int(
        released["used_current_bytes"]
    ) < int(before["used_current_bytes"])
assert engine.embed(inputs[0]) == mean
release_models()
assert embed_batch_cls(inputs) == cls and rerank(q, p, 3) == scores
reloaded = observe()
assert reloaded["status"] == before["status"]
if reloaded["status"] == "private":
    assert int(reloaded["live_models_after"]) == 3
release_models()
engine.close()
final = observe()
deadline = time.monotonic() + 5
while (
    final["status"] == "private"
    and any(
        int(final[k]) != 0
        for k in ("live_models_after", "used_current_bytes", "reserved_current_bytes")
    )
    and time.monotonic() < deadline
):
    final = observe()
if final["status"] == "private":
    assert all(
        int(final[k]) == 0
        for k in ("live_models_after", "used_current_bytes", "reserved_current_bytes")
    )
print(
    json.dumps(
        {
            "marker": "RELEASE_HEALTHY_OK",
            "before": before,
            "released": released,
            "reloaded": reloaded,
            "final": final,
        }
    ),
    flush=True,
)
