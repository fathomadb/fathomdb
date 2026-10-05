/*
 * pool_lifecycle.c - explicit memory pool lifecycle / exhaustion / caching probes
 * on Jetson AGX Orin, in a layout where the default pool is unavailable.
 *
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include pool_lifecycle.c -o pool_lifecycle -lcuda
 * Usage: ./pool_lifecycle [--control] [--maxsize BYTES_MIB] [--threshold max|0]
 *   default layout: three 4 KiB PROT_NONE pages at 38/68/98 GiB before cuInit
 *   (default pool unavailable); --control skips them.
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif
#define MiB (1ULL << 20)

static const char *nm(CUresult r) { const char *n = "?"; cuGetErrorName(r, &n); return n; }
#define P(what, expr) do { CUresult r_p_ = (expr); printf("%-58s rc=%d (%s)\n", what, (int)r_p_, nm(r_p_)); } while (0)
static double now_ms(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec * 1e3 + t.tv_nsec / 1e6; }

static CUmemoryPool make_pool(CUdevice dev, size_t maxsize, int thr_max, CUresult *rc) {
    CUmemPoolProps p;
    memset(&p, 0, sizeof p);
    p.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    p.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    p.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    p.location.id = (int)dev;
    p.maxSize = maxsize;
    CUmemoryPool pool = NULL;
    *rc = cuMemPoolCreate(&pool, &p);
    if (*rc == CUDA_SUCCESS && thr_max) {
        cuuint64_t t = UINT64_MAX;
        P("  cuMemPoolSetAttribute(RELEASE_THRESHOLD=UINT64_MAX)", cuMemPoolSetAttribute(pool, CU_MEMPOOL_ATTR_RELEASE_THRESHOLD, &t));
    }
    return pool;
}

static void pool_attrs(const char *label, CUmemoryPool pool) {
    cuuint64_t thr = 0, rc_ = 0, rh = 0, uc = 0, uh = 0;
    CUresult r1 = cuMemPoolGetAttribute(pool, CU_MEMPOOL_ATTR_RELEASE_THRESHOLD, &thr);
    cuMemPoolGetAttribute(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT, &rc_);
    cuMemPoolGetAttribute(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH, &rh);
    cuMemPoolGetAttribute(pool, CU_MEMPOOL_ATTR_USED_MEM_CURRENT, &uc);
    cuMemPoolGetAttribute(pool, CU_MEMPOOL_ATTR_USED_MEM_HIGH, &uh);
    printf("  attrs[%s] get_rc=%d threshold=%llu reserved_cur=%llu reserved_high=%llu used_cur=%llu used_high=%llu\n",
           label, (int)r1, (unsigned long long)thr, (unsigned long long)rc_, (unsigned long long)rh,
           (unsigned long long)uc, (unsigned long long)uh);
}

/* Candle-like churn: alloc/free mixed sizes on one stream, synchronizing every 8 ops. */
static double churn(CUstream s, int iters, int sync_alloc, CUresult *first_err) {
    static const size_t sizes[6] = {4096, 64 << 10, 1 << 20, 3 << 20, 12 << 20, 256};
    *first_err = CUDA_SUCCESS;
    double t0 = now_ms();
    for (int i = 0; i < iters; i++) {
        CUdeviceptr a = 0, b = 0;
        size_t sa = sizes[i % 6], sb = sizes[(i * 7 + 3) % 6];
        CUresult r = sync_alloc ? cuMemAlloc(&a, sa) : cuMemAllocAsync(&a, sa, s);
        CUresult r2 = sync_alloc ? cuMemAlloc(&b, sb) : cuMemAllocAsync(&b, sb, s);
        if ((r || r2) && *first_err == CUDA_SUCCESS) *first_err = r ? r : r2;
        if (!r) { cuMemsetD8Async(a, 1, sa, s); }
        if (sync_alloc) {
            cuStreamSynchronize(s);
            if (!r) cuMemFree(a);
            if (!r2) cuMemFree(b);
        } else {
            if (!r) cuMemFreeAsync(a, s);
            if (!r2) cuMemFreeAsync(b, s);
            if (i % 8 == 7) cuStreamSynchronize(s);
        }
    }
    cuStreamSynchronize(s);
    return now_ms() - t0;
}

int main(int argc, char **argv) {
    int control = 0, thr_max = 0;
    size_t maxsize = 1024 * MiB;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--control")) control = 1;
        else if (!strcmp(argv[i], "--maxsize") && i + 1 < argc) maxsize = strtoull(argv[++i], NULL, 0) * MiB;
        else if (!strcmp(argv[i], "--threshold") && i + 1 < argc) thr_max = !strcmp(argv[++i], "max");
    }
    printf("INFO layout=%s maxsize=%lluMiB threshold=%s\n", control ? "control" : "3-pages", (unsigned long long)(maxsize / MiB), thr_max ? "max" : "0(default)");
    if (!control) {
        const unsigned long long gib = 1ULL << 30, at[3] = {38 * gib, 68 * gib, 98 * gib};
        for (int i = 0; i < 3; i++)
            mmap((void *)at[i], 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    }
    CUdevice dev; CUcontext ctx; CUresult r; CUmemoryPool def = NULL, cur = NULL;
    cuInit(0); cuDeviceGet(&dev, 0);
    P("cuDevicePrimaryCtxRetain #1", cuDevicePrimaryCtxRetain(&ctx, dev));
    cuCtxSetCurrent(ctx);
    r = cuDeviceGetDefaultMemPool(&def, dev); P("cuDeviceGetDefaultMemPool", r);
    r = cuDeviceGetMemPool(&cur, dev); printf("%-58s rc=%d (%s) pool=%p\n", "cuDeviceGetMemPool (before set)", (int)r, nm(r), (void *)cur);

    /* A. create + set */
    CUresult rc;
    CUmemoryPool pool = make_pool(dev, maxsize, thr_max, &rc);
    printf("%-58s rc=%d (%s) pool=%p\n", "A cuMemPoolCreate(explicit maxSize)", (int)rc, nm(rc), (void *)pool);
    if (rc) return 1;
    P("A cuDeviceSetMemPool(explicit)", cuDeviceSetMemPool(dev, pool));
    r = cuDeviceGetMemPool(&cur, dev); printf("%-58s rc=%d (%s) is_explicit=%d\n", "A cuDeviceGetMemPool", (int)r, nm(r), cur == pool);
    CUdeviceptr p = 0;
    P("A cuMemAllocAsync(4)", cuMemAllocAsync(&p, 4, NULL)); cuMemFreeAsync(p, NULL); cuCtxSynchronize();
    pool_attrs("after A", pool);

    /* B. second retain (as a second CudaContext would do), then release all to refcount 0 and retain again */
    CUcontext ctx2;
    P("B cuDevicePrimaryCtxRetain #2", cuDevicePrimaryCtxRetain(&ctx2, dev));
    r = cuDeviceGetMemPool(&cur, dev); printf("%-58s rc=%d is_explicit=%d\n", "B cuDeviceGetMemPool after 2nd retain", (int)r, cur == pool);
    P("B release #2", cuDevicePrimaryCtxRelease(dev));
    P("B release #1 (refcount -> 0, primary ctx destroyed)", cuDevicePrimaryCtxRelease(dev));
    P("B cuDevicePrimaryCtxRetain #3 (fresh primary ctx)", cuDevicePrimaryCtxRetain(&ctx, dev));
    cuCtxSetCurrent(ctx);
    r = cuDeviceGetMemPool(&cur, dev); printf("%-58s rc=%d (%s) is_explicit=%d pool=%p\n", "B cuDeviceGetMemPool after ctx recreate", (int)r, nm(r), cur == pool, (void *)cur);
    pool_attrs("old handle after ctx recreate", pool);
    p = 0; r = cuMemAllocAsync(&p, 4, NULL); P("B cuMemAllocAsync(4) after ctx recreate", r);
    if (!r) { cuMemFreeAsync(p, NULL); cuCtxSynchronize(); }
    p = 0; r = cuMemAllocFromPoolAsync(&p, 4, pool, NULL); P("B cuMemAllocFromPoolAsync(4, old handle)", r);
    if (!r) { cuMemFreeAsync(p, NULL); cuCtxSynchronize(); }
    if (cur != pool) {
        /* re-establish for the following tests */
        pool = make_pool(dev, maxsize, thr_max, &rc);
        P("B re-create pool", rc);
        P("B re-set pool", cuDeviceSetMemPool(dev, pool));
    }

    /* C. exhaustion */
    {
        CUdeviceptr held[256]; int n = 0; CUresult last = CUDA_SUCCESS;
        size_t chunk = maxsize >= 3072 * MiB ? 256 * MiB : 64 * MiB;
        while (n < 256) {
            CUdeviceptr q = 0;
            last = cuMemAllocAsync(&q, chunk, NULL);
            if (last) break;
            held[n++] = q;
        }
        printf("%-58s allocated=%d x %lluMiB = %llu MiB, then rc=%d (%s)\n", "C exhaustion: async allocs until failure", n, (unsigned long long)(chunk / MiB), (unsigned long long)(n * chunk / MiB), (int)last, nm(last));
        pool_attrs("at exhaustion", pool);
        CUdeviceptr s = 0; CUresult rs = cuMemAlloc(&s, 64 * MiB);
        P("C cuMemAlloc(64MiB) sync while pool exhausted", rs);
        if (!rs) cuMemFree(s);
        P("C cuCtxSynchronize after failed async alloc (sticky?)", cuCtxSynchronize());
        for (int i = 0; i < n; i++) cuMemFreeAsync(held[i], NULL);
        P("C sync after frees", cuCtxSynchronize());
        CUdeviceptr q = 0; CUresult rq = cuMemAllocAsync(&q, 4, NULL);
        P("C cuMemAllocAsync(4) after frees", rq);
        if (!rq) cuMemFreeAsync(q, NULL);
        cuCtxSynchronize();
        pool_attrs("after frees+sync", pool);
    }

    /* E. caching benchmark */
    {
        CUstream s; cuStreamCreate(&s, CU_STREAM_NON_BLOCKING);
        CUresult fe;
        double t_warm = churn(s, 200, 0, &fe);
        double t = churn(s, 4000, 0, &fe);
        printf("E churn explicit pool: 4000 iters %.1f ms (warm %.1f) first_err=%d\n", t, t_warm, (int)fe);
        pool_attrs("after churn", pool);
        double ts = churn(s, 4000, 1, &fe);
        printf("E churn sync cuMemAlloc/cuMemFree: 4000 iters %.1f ms first_err=%d\n", ts, (int)fe);
        if (def) {
            P("E cuDeviceSetMemPool(default)", cuDeviceSetMemPool(dev, def));
            churn(s, 200, 0, &fe);
            double td = churn(s, 4000, 0, &fe);
            printf("E churn default pool: 4000 iters %.1f ms first_err=%d\n", td, (int)fe);
            pool_attrs("default pool after churn", def);
            P("E cuDeviceSetMemPool(explicit) again", cuDeviceSetMemPool(dev, pool));
        }
        cuStreamDestroy(s);
    }

    /* D. destroy while current */
    P("D cuMemPoolDestroy(explicit, while current)", cuMemPoolDestroy(pool));
    cur = NULL; r = cuDeviceGetMemPool(&cur, dev);
    printf("%-58s rc=%d (%s) pool=%p is_default=%d\n", "D cuDeviceGetMemPool after destroy", (int)r, nm(r), (void *)cur, def && cur == def);
    p = 0; r = cuMemAllocAsync(&p, 4, NULL); P("D cuMemAllocAsync(4) after destroy", r);
    if (!r) { cuMemFreeAsync(p, NULL); }
    P("D cuCtxSynchronize", cuCtxSynchronize());
    P("final release", cuDevicePrimaryCtxRelease(dev));
    return 0;
}
