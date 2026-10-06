/*
 * pool_c5.c - 0.8.28 pool study, protocol revision 3, Phase 1b step 2.
 *
 * Measures, for a private pool (cuMemPoolCreate, never installed; allocate
 * with cuMemAllocFromPoolAsync, free with cuMemFreeAsync):
 *   ctxfree  cuMemPoolCreate with cuInit done and no current context
 *            (hypothesis M5: the call needs no context).
 *   getpool  the (CUresult, handle) pairs of cuDeviceGetMemPool and
 *            cuDeviceGetDefaultMemPool, and the address-space bytes newly
 *            mapped by each call (whether reading the current pool can
 *            create the default pool; C9).
 *   zero     cuMemAllocFromPoolAsync of 0 bytes: CUresult and pointer, and the
 *            CUresult of freeing what it returned (C5).
 *   cap      1 MiB chunks from the private pool until the first failure: its
 *            CUresult, cuCtxSynchronize afterwards, VmRSS against the pool's
 *            reserved_high, recovery after freeing, reserved_cur after a
 *            synchronization and after cuMemPoolTrimTo(0) (CB1-CB3 unit
 *            halves).
 *   coexist  the current pool is unchanged by everything above, and a 16 MiB
 *            cuMemAllocAsync (another user of the current pool) raises the
 *            current pool's used memory and not the private pool's (C9).
 *
 * Build: gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include \
 *            pool_c5.c -o pool_c5 -lcuda
 * Usage: pool_c5 --maxsize MiB [--layout control|3pages] [--threshold 0|max]
 *                [--tag S]
 *   3pages: 4 KiB PROT_NONE pages at 38/68/98 GiB before cuInit (the default
 *   pool is unavailable there), as in pool_capacity.c.
 * Memory safety (protocol 1.2): refuses maxSize > 8 GiB; stops filling if
 * MemAvailable falls below 8 GiB or VmRSS exceeds maxSize + 2 GiB.
 * Prints INFO lines and one RESULT line of key=value fields.
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif
#define MiB (1ULL << 20)
#define GiB (1ULL << 30)
#define MAX_RANGES 8192

static const char *nm(CUresult r) { const char *n = "?"; cuGetErrorName(r, &n); return n ? n : "?"; }

static long long status_kib(const char *key) {
    FILE *f = fopen("/proc/self/status", "r");
    char line[256], fmt[64];
    long long v = -1;
    snprintf(fmt, sizeof fmt, "%s: %%lld kB", key);
    while (f && fgets(line, sizeof line, f))
        if (sscanf(line, fmt, &v) == 1) break;
    if (f) fclose(f);
    return v;
}

static long long mem_available_kib(void) {
    FILE *f = fopen("/proc/meminfo", "r");
    char line[256];
    long long v = -1;
    while (f && fgets(line, sizeof line, f))
        if (sscanf(line, "MemAvailable: %lld kB", &v) == 1) break;
    if (f) fclose(f);
    return v;
}

typedef struct { unsigned long long lo, hi; } range_t;
typedef struct { range_t r[MAX_RANGES]; int n; } maps_t;

static void read_maps(maps_t *m) {
    FILE *f = fopen("/proc/self/maps", "r");
    char line[512];
    m->n = 0;
    while (f && fgets(line, sizeof line, f) && m->n < MAX_RANGES) {
        unsigned long long lo, hi;
        if (sscanf(line, "%llx-%llx", &lo, &hi) == 2) { m->r[m->n].lo = lo; m->r[m->n].hi = hi; m->n++; }
    }
    if (f) fclose(f);
}

/* Bytes mapped in `after` that no range of `before` covers (sorted input). */
static unsigned long long new_bytes(const maps_t *before, const maps_t *after) {
    unsigned long long total = 0;
    for (int i = 0; i < after->n; i++) {
        unsigned long long lo = after->r[i].lo, hi = after->r[i].hi, covered = 0;
        for (int j = 0; j < before->n; j++) {
            unsigned long long a = before->r[j].lo > lo ? before->r[j].lo : lo;
            unsigned long long b = before->r[j].hi < hi ? before->r[j].hi : hi;
            if (b > a) covered += b - a;
        }
        total += (hi - lo) - covered;
    }
    return total;
}

static long long attr(CUmemoryPool pool, CUmemPool_attribute a) {
    cuuint64_t v = 0;
    if (!pool || cuMemPoolGetAttribute(pool, a, &v) != CUDA_SUCCESS) return -1;
    return (long long)v;
}

static maps_t g_m0, g_m1, g_m2;

int main(int argc, char **argv) {
    const char *layout = "control", *tag = "-";
    size_t maxsize_mib = 0;
    int thr_max = 0;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i], *v = i + 1 < argc ? argv[i + 1] : NULL;
        if (!strcmp(a, "--maxsize") && v) { maxsize_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--layout") && v) { layout = v; i++; }
        else if (!strcmp(a, "--threshold") && v) { thr_max = !strcmp(v, "max"); i++; }
        else if (!strcmp(a, "--tag") && v) { tag = v; i++; }
        else { fprintf(stderr, "unknown/incomplete option %s\n", a); return 2; }
    }
    if (!maxsize_mib) { fprintf(stderr, "--maxsize must be positive\n"); return 2; }
    if (maxsize_mib > 8192) { fprintf(stderr, "refusing maxSize > 8 GiB (protocol 1.2)\n"); return 2; }
    const size_t maxsize = maxsize_mib * MiB;
    if (mem_available_kib() < (long long)((2 * maxsize + 16 * GiB) / 1024)) {
        fprintf(stderr, "refusing: MemAvailable below 2*maxSize+16GiB\n");
        return 3;
    }
    int placed = 0;
    if (!strcmp(layout, "3pages")) {
        static const unsigned long long at[] = {0x980000000ULL, 0x1100000000ULL, 0x1880000000ULL};
        for (int i = 0; i < 3; i++) {
            void *p = mmap((void *)at[i], 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            placed += p == (void *)at[i];
        }
    } else if (strcmp(layout, "control")) { fprintf(stderr, "bad --layout\n"); return 2; }

    CUresult r = cuInit(0);
    if (r != CUDA_SUCCESS) { printf("RESULT tag=%s layout=%s init=%s\n", tag, layout, nm(r)); return 1; }
    CUdevice dev;
    cuDeviceGet(&dev, 0);
    CUmemPoolProps props;
    memset(&props, 0, sizeof props);
    props.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    props.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    props.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    props.location.id = (int)dev;
    props.maxSize = maxsize;

    /* ctxfree: no context has been retained or made current yet. */
    CUcontext cur = NULL;
    cuCtxGetCurrent(&cur);
    CUmemoryPool probe_pool = NULL;
    CUresult r_ctxfree = cuMemPoolCreate(&probe_pool, &props);
    if (r_ctxfree == CUDA_SUCCESS) cuMemPoolDestroy(probe_pool);

    CUcontext ctx;
    r = cuDevicePrimaryCtxRetain(&ctx, dev);
    if (r != CUDA_SUCCESS) { printf("RESULT tag=%s layout=%s ctx=%s\n", tag, layout, nm(r)); return 1; }
    cuCtxSetCurrent(ctx);

    /* getpool: maps diff around the first current-pool and default-pool reads. */
    CUmemoryPool cur0 = NULL, def0 = NULL;
    read_maps(&g_m0);
    CUresult r_get0 = cuDeviceGetMemPool(&cur0, dev);
    read_maps(&g_m1);
    CUresult r_def0 = cuDeviceGetDefaultMemPool(&def0, dev);
    read_maps(&g_m2);
    unsigned long long nb_get = new_bytes(&g_m0, &g_m1), nb_def = new_bytes(&g_m1, &g_m2);

    long long rss0 = status_kib("VmRSS"), swap0 = status_kib("VmSwap");
    CUmemoryPool pool = NULL;
    CUresult r_create = cuMemPoolCreate(&pool, &props), r_thr = CUDA_ERROR_NOT_PERMITTED;
    if (r_create == CUDA_SUCCESS) {
        cuuint64_t t = thr_max ? UINT64_MAX : 0;
        r_thr = cuMemPoolSetAttribute(pool, CU_MEMPOOL_ATTR_RELEASE_THRESHOLD, &t);
    }
    printf("INFO pid=%d layout=%s blockers=%d maxsize_mib=%zu threshold=%s ctx_current_before=%d ctxfree_create=%s "
           "get_current=%s current=%p get_default=%s default=%p new_bytes_get_current=%llu new_bytes_get_default=%llu "
           "create=%s set_threshold=%s\n",
           (int)getpid(), layout, placed, maxsize_mib, thr_max ? "max" : "0", cur != NULL, nm(r_ctxfree), nm(r_get0),
           (void *)cur0, nm(r_def0), (void *)def0, nb_get, nb_def, nm(r_create), nm(r_thr));
    if (r_create != CUDA_SUCCESS || r_thr != CUDA_SUCCESS) {
        printf("RESULT tag=%s layout=%s maxsize_mib=%zu ctxfree_create=%s get_current=%s get_default=%s create=%s\n", tag,
               layout, maxsize_mib, nm(r_ctxfree), nm(r_get0), nm(r_def0), nm(r_create));
        return 1;
    }

    /* zero */
    CUdeviceptr zp = 0xdeadbeef;
    CUresult r_zero = cuMemAllocFromPoolAsync(&zp, 0, pool, NULL);
    CUresult r_zero_free = CUDA_ERROR_NOT_PERMITTED;
    unsigned long long zero_ptr = r_zero == CUDA_SUCCESS ? (unsigned long long)zp : 0xffffffffffffffffULL;
    if (r_zero == CUDA_SUCCESS) r_zero_free = cuMemFreeAsync(zp, NULL);
    CUresult r_zero_sync = cuCtxSynchronize();

    /* cap */
    size_t cap = maxsize / MiB + 16, n = 0;
    CUdeviceptr *held = calloc(cap, sizeof *held);
    long long rss_cap_kib = rss0 + (long long)((maxsize + 2 * GiB) / 1024);
    CUresult r_fill = CUDA_SUCCESS;
    int aborted = 0;
    while (n < cap) {
        if (n % 256 == 0 && (mem_available_kib() < (long long)(8 * GiB / 1024) || status_kib("VmRSS") > rss_cap_kib)) {
            aborted = 1;
            break;
        }
        CUdeviceptr q = 0;
        r_fill = cuMemAllocFromPoolAsync(&q, MiB, pool, NULL);
        if (r_fill != CUDA_SUCCESS) break;
        cuMemsetD8Async(q, 0x5a, MiB, NULL);
        held[n++] = q;
    }
    CUresult r_sync_after_oom = cuCtxSynchronize();
    long long rss_at_cap = status_kib("VmRSS"), swap_at_cap = status_kib("VmSwap");
    long long res_high = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH);
    long long used_high = attr(pool, CU_MEMPOOL_ATTR_USED_MEM_HIGH);
    for (size_t i = 0; i < n; i++) cuMemFreeAsync(held[i], NULL);
    CUresult r_sync_free = cuCtxSynchronize();
    long long res_after_sync = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT);
    long long rss_after_sync = status_kib("VmRSS");
    CUdeviceptr again = 0;
    CUresult r_recover = cuMemAllocFromPoolAsync(&again, 64 * MiB, pool, NULL);
    if (r_recover == CUDA_SUCCESS) cuMemFreeAsync(again, NULL);
    CUresult r_sync_recover = cuCtxSynchronize();
    CUresult r_trim = cuMemPoolTrimTo(pool, 0);
    long long res_after_trim = attr(pool, CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT);
    long long rss_after_trim = status_kib("VmRSS");

    /* coexist: the current pool is untouched, and another user's
     * cuMemAllocAsync draws from it, not from the private pool. */
    CUmemoryPool cur1 = NULL;
    CUresult r_get1 = cuDeviceGetMemPool(&cur1, dev);
    long long other_used0 = r_get1 == CUDA_SUCCESS ? attr(cur1, CU_MEMPOOL_ATTR_USED_MEM_CURRENT) : -1;
    long long priv_used0 = attr(pool, CU_MEMPOOL_ATTR_USED_MEM_CURRENT);
    CUdeviceptr op = 0;
    CUresult r_other = cuMemAllocAsync(&op, 16 * MiB, NULL);
    cuCtxSynchronize();
    long long other_used1 = r_get1 == CUDA_SUCCESS ? attr(cur1, CU_MEMPOOL_ATTR_USED_MEM_CURRENT) : -1;
    long long priv_used1 = attr(pool, CU_MEMPOOL_ATTR_USED_MEM_CURRENT);
    if (r_other == CUDA_SUCCESS) cuMemFreeAsync(op, NULL);
    cuCtxSynchronize();
    int same_current = r_get0 == r_get1 && cur0 == cur1;
    int current_is_default = r_get0 == CUDA_SUCCESS && r_def0 == CUDA_SUCCESS ? cur0 == def0 : -1;

    printf("RESULT tag=%s layout=%s maxsize_mib=%zu threshold=%s ctxfree_create=%s get_current=%s get_default=%s "
           "new_bytes_get_current=%llu new_bytes_get_default=%llu current_is_default=%d create=%s "
           "zero=%s zero_ptr=0x%llx zero_free=%s zero_sync=%s "
           "fill_mib=%zu fill_error=%s abort=%d sync_after_oom=%s reserved_high=%lld used_high=%lld "
           "rss_delta_at_cap_kib=%lld swap_delta_at_cap_kib=%lld sync_after_free=%s reserved_after_sync=%lld "
           "rss_delta_after_sync_kib=%lld recover=%s recover_sync=%s trim=%s reserved_after_trim=%lld "
           "rss_delta_after_trim_kib=%lld same_current=%d other_alloc=%s other_used_delta=%lld private_used_delta=%lld\n",
           tag, layout, maxsize_mib, thr_max ? "max" : "0", nm(r_ctxfree), nm(r_get0), nm(r_def0), nb_get, nb_def,
           current_is_default, nm(r_create), nm(r_zero), zero_ptr, nm(r_zero_free), nm(r_zero_sync), n, nm(r_fill),
           aborted, nm(r_sync_after_oom), res_high, used_high, rss_at_cap - rss0, swap_at_cap - swap0,
           nm(r_sync_free), res_after_sync, rss_after_sync - rss0, nm(r_recover), nm(r_sync_recover), nm(r_trim),
           res_after_trim, rss_after_trim - rss0, same_current, nm(r_other),
           other_used1 >= 0 && other_used0 >= 0 ? other_used1 - other_used0 : -1, priv_used1 - priv_used0);
    free(held);
    cuMemPoolDestroy(pool);
    cuDevicePrimaryCtxRelease(dev);
    return 0;
}
