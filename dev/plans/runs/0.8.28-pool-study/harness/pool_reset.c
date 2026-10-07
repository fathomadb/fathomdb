/*
 * pool_reset.c - 0.8.28 pool study, protocol C7 for the private pool: what a
 * co-resident library's cuDevicePrimaryCtxReset does to a private pool and to
 * memory allocated from it.
 *
 * Steps: cuInit; retain and set the primary context; create a private pool
 * (never installed); allocate 16 MiB from it (cuMemAllocFromPoolAsync) and
 * synchronize; then, as another library would, cuDevicePrimaryCtxReset;
 * retain and set the primary context again; then
 *   (a) allocate 16 MiB from the same pool handle and synchronize,
 *   (b) free the pre-reset pointer with cuMemFreeAsync and synchronize,
 *   (c) read the pool's used/reserved attributes,
 *   (d) destroy the pool.
 * Every CUresult is printed; a crash is the process's exit status.
 *
 * Build: gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include \
 *            pool_reset.c -o pool_reset -lcuda
 * Usage: pool_reset --maxsize MiB [--tag S]
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MiB (1ULL << 20)

static const char *nm(CUresult r) { const char *n = "?"; cuGetErrorName(r, &n); return n ? n : "?"; }

static long long attr(CUmemoryPool pool, CUmemPool_attribute a) {
    cuuint64_t v = 0;
    return cuMemPoolGetAttribute(pool, a, &v) == CUDA_SUCCESS ? (long long)v : -1;
}

int main(int argc, char **argv) {
    const char *tag = "-";
    size_t maxsize_mib = 3072;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--maxsize") && i + 1 < argc) maxsize_mib = strtoull(argv[++i], NULL, 0);
        else if (!strcmp(argv[i], "--tag") && i + 1 < argc) tag = argv[++i];
        else { fprintf(stderr, "unknown option %s\n", argv[i]); return 2; }
    }
    if (maxsize_mib > 8192) { fprintf(stderr, "refusing maxSize > 8 GiB\n"); return 2; }
    CUdevice dev;
    CUcontext ctx;
    CUresult r = cuInit(0);
    if (r != CUDA_SUCCESS) { printf("RESULT tag=%s init=%s\n", tag, nm(r)); return 1; }
    cuDeviceGet(&dev, 0);
    cuDevicePrimaryCtxRetain(&ctx, dev);
    cuCtxSetCurrent(ctx);
    CUmemPoolProps props;
    memset(&props, 0, sizeof props);
    props.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    props.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    props.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    props.location.id = (int)dev;
    props.maxSize = maxsize_mib * MiB;
    CUmemoryPool pool = NULL;
    CUresult r_create = cuMemPoolCreate(&pool, &props);
    if (r_create != CUDA_SUCCESS) { printf("RESULT tag=%s create=%s\n", tag, nm(r_create)); return 1; }
    CUdeviceptr before = 0, after = 0;
    CUresult r_alloc0 = cuMemAllocFromPoolAsync(&before, 16 * MiB, pool, NULL);
    CUresult r_sync0 = cuCtxSynchronize();
    CUresult r_reset = cuDevicePrimaryCtxReset(dev);
    CUresult r_retain = cuDevicePrimaryCtxRetain(&ctx, dev);
    cuCtxSetCurrent(ctx);
    CUresult r_alloc1 = cuMemAllocFromPoolAsync(&after, 16 * MiB, pool, NULL);
    CUresult r_sync1 = cuCtxSynchronize();
    CUresult r_free_before = cuMemFreeAsync(before, NULL);
    CUresult r_sync2 = cuCtxSynchronize();
    long long used = attr(pool, CU_MEMPOOL_ATTR_USED_MEM_CURRENT);
    long long reserved = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT);
    CUresult r_free_after = r_alloc1 == CUDA_SUCCESS ? cuMemFreeAsync(after, NULL) : CUDA_ERROR_NOT_PERMITTED;
    CUresult r_sync3 = cuCtxSynchronize();
    CUresult r_destroy = cuMemPoolDestroy(pool);
    printf("RESULT tag=%s create=%s alloc_before=%s sync_before=%s reset=%s retain=%s alloc_after=%s sync_after=%s "
           "free_pre_reset_ptr=%s sync_after_free=%s used_after=%lld reserved_after=%lld free_post_reset_ptr=%s "
           "sync_final=%s destroy=%s\n",
           tag, nm(r_create), nm(r_alloc0), nm(r_sync0), nm(r_reset), nm(r_retain), nm(r_alloc1), nm(r_sync1),
           nm(r_free_before), nm(r_sync2), used, reserved, nm(r_free_after), nm(r_sync3), nm(r_destroy));
    cuDevicePrimaryCtxRelease(dev);
    return 0;
}
