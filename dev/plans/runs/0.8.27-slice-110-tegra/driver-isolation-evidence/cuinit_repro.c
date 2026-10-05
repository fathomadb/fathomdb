/* cuinit_repro.c
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include cuinit_repro.c -o cuinit_repro -lcuda
 * Run:   ./cuinit_repro          # 4 KiB pages every 3.99 GiB in [8, 128) GiB -> cuInit fails
 *        ./cuinit_repro 4.0      # every 4.0 GiB (one hole of exactly 4 GiB) -> cuInit succeeds
 *        ./cuinit_repro control  # no pages                                  -> cuInit succeeds
 * After a failed cuInit the pages are unmapped and cuInit is called again.
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif
#define GIB (1ULL << 30)

static void check(const char *what, CUresult r) {
    const char *name = "?";
    cuGetErrorName(r, &name);
    printf("%-28s -> %d (%s)\n", what, (int)r, name);
}

int main(int argc, char **argv) {
    int control = argc > 1 && strcmp(argv[1], "control") == 0;
    double spacing_gib = (argc > 1 && !control) ? strtod(argv[1], NULL) : 3.99;
    unsigned long long step = (unsigned long long)(spacing_gib * GIB) & ~0xfffULL;
    void *page[64];
    int n = 0;
    if (!control && step > 0 && step < 120 * GIB) {
        for (unsigned long long a = 8 * GIB + step; a < 128 * GIB && n < 64; a += step) {
            page[n] = mmap((void *)a, 4096, PROT_NONE,
                           MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            if (page[n] == MAP_FAILED) { perror("mmap"); return 2; }
            n++;
        }
        printf("mapped %d x 4 KiB PROT_NONE pages, every %.2f GiB from %#llx to %p\n",
               n, spacing_gib, 8 * GIB + step, page[n - 1]);
    }

    CUresult r = cuInit(0);
    check("cuInit", r);
    if (r != CUDA_SUCCESS && n > 0) {
        for (int i = 0; i < n; i++) munmap(page[i], 4096);
        printf("unmapped the %d pages\n", n);
        r = cuInit(0);
        check("cuInit (retry)", r);
    }
    if (r != CUDA_SUCCESS) return 1;

    CUdevice dev;
    CUcontext ctx;
    CUmemoryPool pool;
    CUdeviceptr p;
    check("cuDeviceGet", cuDeviceGet(&dev, 0));
    check("cuDevicePrimaryCtxRetain", cuDevicePrimaryCtxRetain(&ctx, dev));
    check("cuCtxSetCurrent", cuCtxSetCurrent(ctx));
    check("cuMemAlloc(4)", cuMemAlloc(&p, 4));
    check("cuMemFree", cuMemFree(p));
    check("cuDeviceGetDefaultMemPool", cuDeviceGetDefaultMemPool(&pool, dev));
    cuDevicePrimaryCtxRelease(dev);
    return 0;
}
