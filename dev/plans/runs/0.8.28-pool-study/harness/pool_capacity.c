/*
 * pool_capacity.c - 0.8.28 pool study, protocol section 4.5 (C3 capacity).
 *
 * Measures how much an explicit memory pool of a given maxSize can actually
 * hand out, directly, instead of inferring it from where the product fails.
 * Derived from explicit-pool-experiment/pool_lifecycle.c section C.
 *
 * Build: gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include \
 *            pool_capacity.c -o pool_capacity -lcuda
 * Usage: pool_capacity --maxsize MiB [--layout control|3pages|at] [--at A,B,...]
 *                      [--threshold 0|max] [--chunk MiB] [--single] [--tag S]
 *   control: no blockers; 3pages: 4 KiB PROT_NONE pages at 38/68/98 GiB
 *   before cuInit (the default pool is unavailable there); at: pages at the
 *   given hex addresses.
 * Steps: blockers; cuInit; retain + set the primary context; create the pool
 * (pinned, device 0, maxSize, release threshold) and install it with
 * cuDeviceSetMemPool;
 *   (a) cuMemAllocAsync --chunk MiB chunks on the null stream until the first
 *       non-success, touching each with cuMemsetD8Async; record the count,
 *       bytes, CUresult and the pool's reserved/used attributes;
 *   (b) free everything, synchronize, repeat (a) once (recovery), free again;
 *   (c) with --single, trim the pool to 0 and bisect (1 MiB resolution) the
 *       largest single cuMemAllocAsync that succeeds from the empty pool;
 *   (d) cuMemGetInfo before and after (sole consumer; informational).
 * Host memory: /proc/meminfo MemAvailable (KiB) is sampled before the pool
 * exists, at the first error, after freeing everything and synchronizing
 * (where the release threshold decides what the pool keeps), and after
 * cuMemPoolTrimTo(0); with the pool's reserved_cur at the same points.
 * VmRSS of this process is sampled at the same points: on Tegra MemAvailable
 * under-reads device allocations, so VmRSS and the pool counters are the
 * primary signals and MemAvailable is secondary.
 * Memory safety (protocol 1.2): refuses maxSize > 8 GiB; stops allocating if
 * MemAvailable falls below 8 GiB or VmRSS exceeds maxSize + 2 GiB (reported
 * as abort=1).
 * Prints one RESULT line:
 *   RESULT layout maxsize_mib chunk_mib threshold allocated_mib first_error
 *   reserved_high used_high single_max_mib recovered_mib mem_free_before
 *   mem_free_after [extra key=value fields]
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif
#define MiB (1ULL << 20)
#define GiB (1ULL << 30)

static const char *nm(CUresult r) { const char *n = "?"; cuGetErrorName(r, &n); return n ? n : "?"; }

static long long mem_available_kib(void) {
    FILE *f = fopen("/proc/meminfo", "r");
    char line[256];
    long long v = -1;
    while (f && fgets(line, sizeof line, f))
        if (sscanf(line, "MemAvailable: %lld kB", &v) == 1) break;
    if (f) fclose(f);
    return v;
}

static long long vm_rss_kib(void) {
    FILE *f = fopen("/proc/self/status", "r");
    char line[256];
    long long v = -1;
    while (f && fgets(line, sizeof line, f))
        if (sscanf(line, "VmRSS: %lld kB", &v) == 1) break;
    if (f) fclose(f);
    return v;
}

static long long g_rss_cap_kib;

static cuuint64_t attr(CUmemoryPool pool, CUmemPool_attribute a) {
    cuuint64_t v = 0;
    if (cuMemPoolGetAttribute(pool, a, &v) != CUDA_SUCCESS) return (cuuint64_t)-1;
    return v;
}

static void print_attrs(const char *label, CUmemoryPool pool) {
    printf("ATTRS %s threshold=%llu reserved_cur=%llu reserved_high=%llu used_cur=%llu used_high=%llu\n", label,
           (unsigned long long)attr(pool, CU_MEMPOOL_ATTR_RELEASE_THRESHOLD),
           (unsigned long long)attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT),
           (unsigned long long)attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH),
           (unsigned long long)attr(pool, CU_MEMPOOL_ATTR_USED_MEM_CURRENT),
           (unsigned long long)attr(pool, CU_MEMPOOL_ATTR_USED_MEM_HIGH));
}

static int g_abort;

/* Allocates chunk-byte blocks until the first failure. Returns the count;
 * *err gets the failing CUresult (CUDA_SUCCESS if the cap was reached). */
static size_t fill(CUdeviceptr *held, size_t cap, size_t chunk, CUresult *err) {
    size_t n = 0;
    *err = CUDA_SUCCESS;
    while (n < cap) {
        if ((n * chunk) % (256 * MiB) < chunk &&
            (mem_available_kib() < (long long)(8 * GiB / 1024) || vm_rss_kib() > g_rss_cap_kib)) {
            g_abort = 1;
            break;
        }
        CUdeviceptr q = 0;
        CUresult r = cuMemAllocAsync(&q, chunk, NULL);
        if (r != CUDA_SUCCESS) { *err = r; break; }
        cuMemsetD8Async(q, 0x5a, chunk, NULL);
        held[n++] = q;
    }
    return n;
}

static void release_all(CUdeviceptr *held, size_t n) {
    for (size_t i = 0; i < n; i++) cuMemFreeAsync(held[i], NULL);
}

int main(int argc, char **argv) {
    const char *layout = "control", *at_list = NULL, *tag = "-";
    size_t maxsize_mib = 0, chunk_mib = 64;
    int thr_max = 0, single = 0;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i], *v = i + 1 < argc ? argv[i + 1] : NULL;
        if (!strcmp(a, "--maxsize") && v) { maxsize_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--layout") && v) { layout = v; i++; }
        else if (!strcmp(a, "--at") && v) { at_list = v; i++; }
        else if (!strcmp(a, "--threshold") && v) { thr_max = !strcmp(v, "max"); i++; }
        else if (!strcmp(a, "--chunk") && v) { chunk_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--single")) single = 1;
        else if (!strcmp(a, "--tag") && v) { tag = v; i++; }
        else { fprintf(stderr, "unknown/incomplete option %s\n", a); return 2; }
    }
    if (!maxsize_mib || !chunk_mib) { fprintf(stderr, "--maxsize and --chunk must be positive\n"); return 2; }
    if (maxsize_mib > 8192) { fprintf(stderr, "refusing maxSize > 8 GiB (protocol 1.2)\n"); return 2; }
    const size_t maxsize = maxsize_mib * MiB, chunk = chunk_mib * MiB;
    long long avail = mem_available_kib();
    if (avail < (long long)((2 * maxsize + 16 * GiB) / 1024)) {
        fprintf(stderr, "refusing: MemAvailable %lld KiB < 2*maxSize+16GiB\n", avail);
        return 3;
    }

    int placed = 0;
    if (!strcmp(layout, "3pages")) at_list = "0x980000000,0x1100000000,0x1880000000";
    else if (strcmp(layout, "control") && strcmp(layout, "at")) { fprintf(stderr, "bad --layout\n"); return 2; }
    if (at_list && strcmp(layout, "control")) {
        char *dup = strdup(at_list);
        for (char *t = strtok(dup, ","); t; t = strtok(NULL, ",")) {
            unsigned long long addr = strtoull(t, NULL, 16);
            void *p = mmap((void *)addr, 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            placed += p == (void *)addr;
        }
        free(dup);
    }

    CUdevice dev;
    CUcontext ctx;
    CUresult r_init = cuInit(0);
    if (r_init != CUDA_SUCCESS) { printf("RESULT tag=%s layout=%s init=%d\n", tag, layout, (int)r_init); return 1; }
    int drv = 0;
    cuDriverGetVersion(&drv);
    cuDeviceGet(&dev, 0);
    CUresult r_ctx = cuDevicePrimaryCtxRetain(&ctx, dev);
    if (r_ctx != CUDA_SUCCESS) { printf("RESULT tag=%s layout=%s ctx=%d\n", tag, layout, (int)r_ctx); return 1; }
    cuCtxSetCurrent(ctx);
    size_t free_before = 0, free_mid = 0, free_after = 0, total = 0;
    cuMemGetInfo(&free_before, &total);
    long long ma_before = mem_available_kib(), rss_before = vm_rss_kib();
    g_rss_cap_kib = rss_before + (long long)((maxsize + 2 * GiB) / 1024);

    CUmemPoolProps props;
    memset(&props, 0, sizeof props);
    props.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    props.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    props.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    props.location.id = (int)dev;
    props.maxSize = maxsize;
    CUmemoryPool pool = NULL;
    CUresult r_create = cuMemPoolCreate(&pool, &props);
    CUresult r_thr = CUDA_SUCCESS, r_set = CUDA_ERROR_NOT_PERMITTED;
    if (r_create == CUDA_SUCCESS) {
        cuuint64_t t = thr_max ? UINT64_MAX : 0;
        r_thr = cuMemPoolSetAttribute(pool, CU_MEMPOOL_ATTR_RELEASE_THRESHOLD, &t);
        r_set = cuDeviceSetMemPool(dev, pool);
    }
    printf("INFO pid=%d driver=%d layout=%s blockers=%d maxsize_mib=%zu chunk_mib=%zu threshold=%s create=%s set_threshold=%s set=%s\n",
           (int)getpid(), drv, layout, placed, maxsize_mib, chunk_mib, thr_max ? "max" : "0", nm(r_create), nm(r_thr), nm(r_set));
    if (r_create != CUDA_SUCCESS || r_set != CUDA_SUCCESS) {
        printf("RESULT tag=%s layout=%s maxsize_mib=%zu chunk_mib=%zu threshold=%s allocated_mib=-1 first_error=%s "
               "reserved_high=-1 used_high=-1 single_max_mib=-1 recovered_mib=-1 mem_free_before=%zu mem_free_after=-1 create=%s set=%s\n",
               tag, layout, maxsize_mib, chunk_mib, thr_max ? "max" : "0", nm(r_create != CUDA_SUCCESS ? r_create : r_set),
               free_before, nm(r_create), nm(r_set));
        return 1;
    }

    size_t cap = maxsize / chunk + 16;
    CUdeviceptr *held = calloc(cap, sizeof *held);
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    CUresult first_err;
    size_t n = fill(held, cap, chunk, &first_err);
    CUresult sync_after_fill = cuCtxSynchronize();
    clock_gettime(CLOCK_MONOTONIC, &t1);
    cuMemGetInfo(&free_mid, &total);
    cuuint64_t res_high = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH);
    cuuint64_t used_high = attr(pool, CU_MEMPOOL_ATTR_USED_MEM_HIGH);
    cuuint64_t res_cur = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT);
    print_attrs("at-first-error", pool);
    long long ma_at_cap = mem_available_kib(), rss_at_cap = vm_rss_kib();
    /* Is the pool still usable for a small request at exhaustion, and does
     * the device still serve a synchronous allocation? */
    CUdeviceptr s = 0;
    CUresult small_at_cap = cuMemAllocAsync(&s, 4, NULL);
    if (small_at_cap == CUDA_SUCCESS) cuMemFreeAsync(s, NULL);
    CUresult sync_alloc_at_cap = cuMemAlloc(&s, 64 * MiB);
    if (sync_alloc_at_cap == CUDA_SUCCESS) cuMemFree(s);

    release_all(held, n);
    CUresult sync_after_free = cuCtxSynchronize();
    print_attrs("after-free", pool);
    long long ma_after_free = mem_available_kib(), rss_after_free = vm_rss_kib();
    cuuint64_t res_after_free = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT);

    CUresult second_err = CUDA_SUCCESS;
    size_t n2 = g_abort ? 0 : fill(held, cap, chunk, &second_err);
    cuCtxSynchronize();
    release_all(held, n2);
    cuCtxSynchronize();
    free(held);

    long long single_max = -1;
    int single_probes = 0;
    if (single && !g_abort) {
        size_t lo = 0, hi = maxsize_mib + 1; /* lo succeeds (0 trivially), hi fails */
        while (hi - lo > 1) {
            size_t mid = lo + (hi - lo) / 2;
            cuMemPoolTrimTo(pool, 0);
            CUdeviceptr q = 0;
            CUresult rq = cuMemAllocAsync(&q, mid * MiB, NULL);
            single_probes++;
            if (rq == CUDA_SUCCESS) {
                cuMemsetD8Async(q, 0x5a, mid * MiB, NULL);
                cuMemFreeAsync(q, NULL);
                lo = mid;
            } else {
                hi = mid;
            }
            cuCtxSynchronize();
        }
        single_max = (long long)lo;
    }
    cuMemPoolTrimTo(pool, 0);
    cuCtxSynchronize();
    cuMemGetInfo(&free_after, &total);
    long long ma_after_trim = mem_available_kib(), rss_after_trim = vm_rss_kib();
    cuuint64_t res_after_trim = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT);
    double fill_ms = (t1.tv_sec - t0.tv_sec) * 1e3 + (t1.tv_nsec - t0.tv_nsec) / 1e6;

    printf("RESULT tag=%s layout=%s maxsize_mib=%zu chunk_mib=%zu threshold=%s allocated_mib=%zu first_error=%s "
           "reserved_high=%llu used_high=%llu single_max_mib=%lld recovered_mib=%zu mem_free_before=%zu mem_free_after=%zu "
           "chunks=%zu reserved_cur_at_cap=%llu mem_free_at_cap=%zu small_at_cap=%s sync_alloc_at_cap=%s "
           "sync_after_fill=%s sync_after_free=%s second_error=%s single_probes=%d fill_ms=%.1f abort=%d total=%zu driver=%d "
           "memavail_before_kib=%lld memavail_at_cap_kib=%lld memavail_after_free_kib=%lld memavail_after_trim_kib=%lld "
           "reserved_after_free=%llu reserved_after_trim=%llu rss_before_kib=%lld rss_at_cap_kib=%lld "
           "rss_after_free_kib=%lld rss_after_trim_kib=%lld\n",
           tag, layout, maxsize_mib, chunk_mib, thr_max ? "max" : "0", n * chunk_mib, nm(first_err),
           (unsigned long long)res_high, (unsigned long long)used_high, single_max, n2 * chunk_mib, free_before, free_after,
           n, (unsigned long long)res_cur, free_mid, nm(small_at_cap), nm(sync_alloc_at_cap), nm(sync_after_fill),
           nm(sync_after_free), nm(second_err), single_probes, fill_ms, g_abort, total, drv,
           ma_before, ma_at_cap, ma_after_free, ma_after_trim, (unsigned long long)res_after_free,
           (unsigned long long)res_after_trim, rss_before, rss_at_cap, rss_after_free, rss_after_trim);
    cuDevicePrimaryCtxRelease(dev);
    return 0;
}
