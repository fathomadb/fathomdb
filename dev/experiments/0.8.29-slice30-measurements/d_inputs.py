"""Deterministic D bodies from the existing AC-013 dense-provider recipe."""

from bisect import bisect_left


def records(count):
    """Yield the fixed-seed ordered doc population, independent of requested size."""
    mask=(1<<64)-1
    state=(0x0AC013D013D0*0x9E3779B97F4A7C15+1)&mask

    def next_u64():
        nonlocal state
        state=(state*6364136223846793005+1442695040888963407)&mask
        return state

    vocab=[f"{chr(97+i//676%26)}{chr(97+i//26%26)}{chr(97+i%26)}{i:04}" for i in range(1024)]
    cumulative=[]
    total=0.0
    for rank in range(1,1025):
        total+=1.0/rank
        cumulative.append(total)
    for index in range(count):
        tokens=55+next_u64()%20
        words=[]
        for _ in range(tokens):
            point=(next_u64()>>11)/(1<<53)*total
            words.append(vocab[min(bisect_left(cumulative,point),1023)])
        yield {"id":f"d-{index}","kind":"doc","body":" ".join(words)}
