/*
 * minimal_repro.c - Jetson AGX Orin: cuMemAllocAsync / default memory pool
 * returns CUDA_ERROR_OUT_OF_MEMORY when three 4 KiB PROT_NONE pages exist in
 * the process address space at 38, 68 and 98 GiB before cuInit.
 *
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include minimal_repro.c -o minimal_repro -lcuda
 * Run:   ./minimal_repro          # with the three pages -> async alloc fails
 *        ./minimal_repro control  # without them         -> async alloc succeeds
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

static void check(const char *what, CUresult r) {
    const char *name = "?";
    cuGetErrorName(r, &name);
    printf("%-28s -> %d (%s)\n", what, (int)r, name);
}

int main(int argc, char **argv) {
    if (!(argc > 1 && strcmp(argv[1], "control") == 0)) {
        const unsigned long long gib = 1ULL << 30;
        const unsigned long long at[3] = {38 * gib, 68 * gib, 98 * gib};
        for (int i = 0; i < 3; i++) {
            void *p = mmap((void *)at[i], 4096, PROT_NONE,
                           MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            printf("mmap 4 KiB PROT_NONE at %#llx -> %p\n", at[i], p);
        }
    }

    CUdevice dev;
    CUcontext ctx;
    CUmemoryPool pool;
    CUdeviceptr p;
    check("cuInit", cuInit(0));
    check("cuDeviceGet", cuDeviceGet(&dev, 0));
    check("cuDevicePrimaryCtxRetain", cuDevicePrimaryCtxRetain(&ctx, dev));
    check("cuCtxSetCurrent", cuCtxSetCurrent(ctx));

    size_t free_b, total_b;
    check("cuMemGetInfo", cuMemGetInfo(&free_b, &total_b));
    printf("  free=%zu total=%zu\n", free_b, total_b);

    check("cuMemAlloc(4)", cuMemAlloc(&p, 4));
    check("cuMemFree", cuMemFree(p));
    check("cuDeviceGetDefaultMemPool", cuDeviceGetDefaultMemPool(&pool, dev));
    CUresult r = cuMemAllocAsync(&p, 4, NULL);
    check("cuMemAllocAsync(4)", r);
    if (r == CUDA_SUCCESS) {
        check("cuMemFreeAsync", cuMemFreeAsync(p, NULL));
        check("cuCtxSynchronize", cuCtxSynchronize());
    }
    cuDevicePrimaryCtxRelease(dev);
    return r == CUDA_SUCCESS ? 0 : 1;
}
