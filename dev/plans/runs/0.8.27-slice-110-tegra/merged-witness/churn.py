# Interference control: repeatedly touch and release 2 GiB of host memory so the
# integrated GPU's system-wide cuMemGetInfo free counter moves during a witness run.
import mmap
import sys
import time

size = 2 << 30
deadline = time.time() + float(sys.argv[1])
while time.time() < deadline:
    m = mmap.mmap(-1, size)
    for off in range(0, size, 4096):
        m[off] = 1
    time.sleep(0.4)
    m.close()
    time.sleep(0.4)
