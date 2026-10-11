/*
 * pool_va_repro.c - minimal CUDA Driver API reproducer: does a small anonymous
 * mapping placed in the address range the driver uses for its GPU VA
 * reservation make cuDeviceGetDefaultMemPool / cuMemAllocAsync fail with
 * CUDA_ERROR_OUT_OF_MEMORY on Jetson (Tegra, integrated GPU)?
 *
 * No dependency other than libcuda and libc/libpthread.
 *
 * Build:
 *   gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda/include \
 *       pool_va_repro.c -o pool_va_repro -lcuda -lpthread
 *
 * Usage (all options optional; no options = control run):
 *   --at ADDR[,ADDR...]   place one blocker mapping at each address (hex ok)
 *   --random N            place N blockers at random addresses ...
 *   --range LO:HI         ... drawn uniformly from [LO, HI)   (default 8G:128G)
 *   --seed S              ... with this PRNG seed               (default 1)
 *   --size BYTES          blocker size, suffix K/M/G allowed    (default 256K)
 *   --prot none|rw        blocker protection                    (default none)
 *   --thread              run all CUDA calls on a new pthread
 *   --pool-maxsize BYTES  also create a pool with this maxSize, allocate from it,
 *                         then cuDeviceSetMemPool it and retry cuMemAllocAsync
 *   --maps-dir DIR        save /proc/self/maps snapshots per stage in DIR
 *   --tag STR             copied into the RESULT line
 *   --pre-reserve A:S[,..] after context creation and before the default pool,
 *                         cuMemAddressReserve(size S, requested address A)
 *
 * Blockers are mmap(MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE) and are
 * created BEFORE cuInit. Addresses are aligned down to the blocker size
 * (min 4 KiB page).
 *
 * Output: one "STEP" line per call with the raw CUresult, a list of the large
 * (>= 1 GiB) anonymous PROT_NONE ("---p") mappings present after context
 * creation, and one final machine-readable "RESULT" line.
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <errno.h>
#include <inttypes.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

#define GiB ((uint64_t)1 << 30)
#define MiB ((uint64_t)1 << 20)
#define KiB ((uint64_t)1 << 10)
#define MAX_BLOCKERS 4096
#define MAX_PIECES 256

static uint64_t g_at[MAX_BLOCKERS];
static int g_nat;
static int g_nrandom;
static uint64_t g_lo = 8 * GiB, g_hi = 128 * GiB;
static uint64_t g_seed = 1;
static uint64_t g_bsize = 256 * KiB;
static int g_prot = PROT_NONE;
static int g_thread;
static uint64_t g_pool_maxsize;
static const char *g_maps_dir;
static const char *g_tag = "-";

static int g_placed, g_place_fail;

#define MAX_PRE 256
static uint64_t g_pre_addr[MAX_PRE], g_pre_size[MAX_PRE];
static int g_npre, g_pre_ok;

/* result summary */
static CUresult r_init, r_ctx, r_attr, r_meminfo, r_alloc, r_defpool, r_async,
    r_freeasync, r_poolcreate, r_poolcreate_ms, r_resv[4];
static CUresult r_setpool = CUDA_ERROR_NOT_PERMITTED, r_setpool_async = CUDA_ERROR_NOT_PERMITTED,
    r_setpool_async_big = CUDA_ERROR_NOT_PERMITTED;
static int attr_pools;
static size_t mem_free, mem_total;
static uint64_t max_piece, sum_pieces;
static int n_pieces;
static uint64_t piece_start[MAX_PIECES], piece_size[MAX_PIECES];

static uint64_t parse_size(const char *s) {
    char *end;
    uint64_t v = strtoull(s, &end, 0);
    switch (*end) {
    case 'k': case 'K': v *= KiB; break;
    case 'm': case 'M': v *= MiB; break;
    case 'g': case 'G': v *= GiB; break;
    case 't': case 'T': v *= GiB * 1024; break;
    default: break;
    }
    return v;
}

/* splitmix64: deterministic, libc-independent */
static uint64_t rng_next(uint64_t *s) {
    uint64_t z = (*s += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

static const char *errname(CUresult r) {
    const char *n = NULL;
    if (cuGetErrorName(r, &n) != CUDA_SUCCESS || !n) return "?";
    return n;
}

static void step(const char *name, CUresult r, const char *extra) {
    printf("STEP %-34s rc=%d (%s)%s%s\n", name, (int)r, errname(r),
           extra ? " " : "", extra ? extra : "");
    fflush(stdout);
}

static void save_maps(const char *stage) {
    if (!g_maps_dir) return;
    char path[4096];
    snprintf(path, sizeof path, "%s/maps-%d-%s.txt", g_maps_dir, (int)getpid(), stage);
    FILE *in = fopen("/proc/self/maps", "r");
    FILE *out = fopen(path, "w");
    if (in && out) {
        char buf[8192];
        size_t n;
        while ((n = fread(buf, 1, sizeof buf, in)) > 0) fwrite(buf, 1, n, out);
    }
    if (in) fclose(in);
    if (out) fclose(out);
}

/* Collect anonymous ---p mappings >= 1 GiB below 1 TiB (the driver's GPU VA
 * reservation; the process has none of these before cuInit). */
static void scan_pieces(const char *label, int record) {
    FILE *f = fopen("/proc/self/maps", "r");
    if (!f) { perror("maps"); return; }
    char line[1024];
    int n = 0;
    uint64_t mx = 0, sum = 0;
    printf("PIECES %s:", label);
    while (fgets(line, sizeof line, f)) {
        uint64_t a, b, off;
        char perms[8];
        unsigned long inode;
        char dev[16];
        int pathpos = 0;
        if (sscanf(line, "%" SCNx64 "-%" SCNx64 " %7s %" SCNx64 " %15s %lu %n",
                   &a, &b, perms, &off, dev, &inode, &pathpos) < 6)
            continue;
        if (strcmp(perms, "---p") != 0 || inode != 0) continue;
        if (b - a < GiB || a >= 1024 * GiB) continue;
        if (n < MAX_PIECES && record) { piece_start[n] = a; piece_size[n] = b - a; }
        printf(" 0x%" PRIx64 "+%.2fGiB", a, (double)(b - a) / GiB);
        n++;
        sum += b - a;
        if (b - a > mx) mx = b - a;
    }
    fclose(f);
    printf(" | n=%d sum=%.2fGiB max=%.2fGiB\n", n, (double)sum / GiB, (double)mx / GiB);
    if (record) { n_pieces = n; max_piece = mx; sum_pieces = sum; }
    fflush(stdout);
}

static void place_blockers(void) {
    uint64_t align = g_bsize < 4096 ? 4096 : g_bsize;
    /* align to a power of two <= size */
    uint64_t p2 = 4096;
    while (p2 * 2 <= align) p2 *= 2;
    uint64_t s = g_seed;
    int total = g_nat + g_nrandom;
    for (int i = 0; i < total; i++) {
        uint64_t addr;
        if (i < g_nat) {
            addr = g_at[i];
        } else {
            uint64_t span = g_hi - g_lo;
            addr = g_lo + rng_next(&s) % span;
        }
        addr &= ~(p2 - 1);
        void *p = mmap((void *)(uintptr_t)addr, g_bsize, g_prot,
                       MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE | MAP_NORESERVE,
                       -1, 0);
        if (p == MAP_FAILED || (uint64_t)(uintptr_t)p != addr) {
            printf("BLOCKER 0x%" PRIx64 " FAILED errno=%d (%s)\n", addr, errno, strerror(errno));
            if (p != MAP_FAILED) munmap(p, g_bsize);
            g_place_fail++;
            continue;
        }
        if (g_prot & PROT_WRITE) memset(p, 0xA5, 64); /* touch it */
        printf("BLOCKER 0x%" PRIx64 " size=0x%" PRIx64 " ok\n", addr, g_bsize);
        g_placed++;
    }
    fflush(stdout);
}

static void *cuda_body(void *unused) {
    (void)unused;
    char extra[256];
    CUdevice dev;
    CUcontext ctx = NULL;

    r_init = cuInit(0);
    step("cuInit", r_init, NULL);
    if (r_init != CUDA_SUCCESS) return NULL;
    save_maps("1-after-init");
    scan_pieces("after_cuInit", 0);

    int drv = 0;
    cuDriverGetVersion(&drv);
    printf("INFO cuDriverGetVersion=%d\n", drv);

    CUresult r = cuDeviceGet(&dev, 0);
    step("cuDeviceGet", r, NULL);
    r_ctx = cuDevicePrimaryCtxRetain(&ctx, dev);
    step("cuDevicePrimaryCtxRetain", r_ctx, NULL);
    if (r_ctx != CUDA_SUCCESS) return NULL;
    r = cuCtxSetCurrent(ctx);
    step("cuCtxSetCurrent", r, NULL);
    save_maps("2-after-ctx");
    scan_pieces("after_ctx", 1);

    r_attr = cuDeviceGetAttribute(&attr_pools, CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED, dev);
    snprintf(extra, sizeof extra, "value=%d", attr_pools);
    step("cuDeviceGetAttribute(POOLS_SUPPORTED)", r_attr, extra);

    r_meminfo = cuMemGetInfo(&mem_free, &mem_total);
    snprintf(extra, sizeof extra, "free=%zu total=%zu", mem_free, mem_total);
    step("cuMemGetInfo", r_meminfo, extra);

    CUdeviceptr dp = 0;
    r_alloc = cuMemAlloc(&dp, 4);
    snprintf(extra, sizeof extra, "ptr=0x%llx", (unsigned long long)dp);
    step("cuMemAlloc_v2(4)", r_alloc, extra);
    if (r_alloc == CUDA_SUCCESS) step("cuMemFree_v2", cuMemFree(dp), NULL);

    /* Optional: consume GPU VA inside the driver's reservation before the
     * default pool is created (cuMemAddressReserve at a requested address). */
    for (int i = 0; i < g_npre; i++) {
        CUdeviceptr va = 0;
        CUresult rr = cuMemAddressReserve(&va, g_pre_size[i], 0, (CUdeviceptr)g_pre_addr[i], 0);
        snprintf(extra, sizeof extra, "req=0x%" PRIx64 " size=0x%" PRIx64 " got=0x%llx%s",
                 g_pre_addr[i], g_pre_size[i], (unsigned long long)va,
                 (rr == CUDA_SUCCESS && va != g_pre_addr[i]) ? " MOVED" : "");
        step("pre:cuMemAddressReserve", rr, extra);
        if (rr == CUDA_SUCCESS && va == g_pre_addr[i]) g_pre_ok++;
    }

    CUmemoryPool defpool = NULL;
    r_defpool = cuDeviceGetDefaultMemPool(&defpool, dev);
    snprintf(extra, sizeof extra, "pool=%p", (void *)defpool);
    step("cuDeviceGetDefaultMemPool", r_defpool, extra);

    CUdeviceptr ap = 0;
    r_async = cuMemAllocAsync(&ap, 4, NULL);
    snprintf(extra, sizeof extra, "ptr=0x%llx", (unsigned long long)ap);
    step("cuMemAllocAsync(4, null stream)", r_async, extra);
    r_freeasync = CUDA_ERROR_NOT_PERMITTED; /* sentinel: not attempted */
    if (r_async == CUDA_SUCCESS) {
        r_freeasync = cuMemFreeAsync(ap, NULL);
        step("cuMemFreeAsync", r_freeasync, NULL);
        step("cuCtxSynchronize", cuCtxSynchronize(), NULL);
    }
    save_maps("3-after-pool");

    CUmemPoolProps props;
    memset(&props, 0, sizeof props);
    props.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    props.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    props.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    props.location.id = 0;
    CUmemoryPool pool = NULL;
    r_poolcreate = cuMemPoolCreate(&pool, &props);
    step("cuMemPoolCreate(maxSize=0)", r_poolcreate, NULL);
    if (r_poolcreate == CUDA_SUCCESS) cuMemPoolDestroy(pool);

    r_poolcreate_ms = CUDA_ERROR_NOT_PERMITTED;
    if (g_pool_maxsize) {
        props.maxSize = g_pool_maxsize;
        pool = NULL;
        r_poolcreate_ms = cuMemPoolCreate(&pool, &props);
        snprintf(extra, sizeof extra, "maxSize=%" PRIu64, g_pool_maxsize);
        step("cuMemPoolCreate(maxSize=N)", r_poolcreate_ms, extra);
        if (r_poolcreate_ms == CUDA_SUCCESS) {
            CUdeviceptr pp = 0;
            CUresult ra = cuMemAllocFromPoolAsync(&pp, 4, pool, NULL);
            step("cuMemAllocFromPoolAsync(4, pool)", ra, NULL);
            if (ra == CUDA_SUCCESS) cuMemFreeAsync(pp, NULL);
            cuCtxSynchronize();
            /* Workaround probe: make this pool the device's current pool so
             * plain cuMemAllocAsync uses it instead of the default pool. */
            r_setpool = cuDeviceSetMemPool(dev, pool);
            step("cuDeviceSetMemPool(explicit pool)", r_setpool, NULL);
            if (r_setpool == CUDA_SUCCESS) {
                const size_t sizes[2] = {4, 256u << 20};
                for (int k = 0; k < 2; k++) {
                    CUdeviceptr q = 0;
                    CUresult rq = cuMemAllocAsync(&q, sizes[k], NULL);
                    snprintf(extra, sizeof extra, "bytes=%zu ptr=0x%llx", sizes[k], (unsigned long long)q);
                    step("cuMemAllocAsync after SetMemPool", rq, extra);
                    if (k == 0) r_setpool_async = rq;
                    else r_setpool_async_big = rq;
                    if (rq == CUDA_SUCCESS) {
                        cuMemsetD8Async(q, 0x5a, sizes[k], NULL);
                        cuMemFreeAsync(q, NULL);
                    }
                }
                step("cuCtxSynchronize", cuCtxSynchronize(), NULL);
            }
        }
    }

    const uint64_t rs[4] = {2 * (uint64_t)MiB, 64 * (uint64_t)MiB, 1 * (uint64_t)GiB, 32 * (uint64_t)GiB};
    for (int i = 0; i < 4; i++) {
        CUdeviceptr va = 0;
        r_resv[i] = cuMemAddressReserve(&va, rs[i], 0, 0, 0);
        char name[64];
        snprintf(name, sizeof name, "cuMemAddressReserve(%" PRIu64 "MiB)", rs[i] / MiB);
        snprintf(extra, sizeof extra, "va=0x%llx", (unsigned long long)va);
        step(name, r_resv[i], extra);
        if (r_resv[i] == CUDA_SUCCESS) cuMemAddressFree(va, rs[i]);
    }

    cuDevicePrimaryCtxRelease(dev);
    return NULL;
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        const char *v = (i + 1 < argc) ? argv[i + 1] : NULL;
        if (!strcmp(a, "--at") && v) {
            char *dup = strdup(v), *tok, *save = NULL;
            for (tok = strtok_r(dup, ",", &save); tok && g_nat < MAX_BLOCKERS;
                 tok = strtok_r(NULL, ",", &save))
                g_at[g_nat++] = parse_size(tok);
            free(dup);
            i++;
        } else if (!strcmp(a, "--random") && v) { g_nrandom = atoi(v); i++; }
        else if (!strcmp(a, "--range") && v) {
            const char *c = strchr(v, ':');
            if (!c) { fprintf(stderr, "--range LO:HI\n"); return 2; }
            g_lo = parse_size(v);
            g_hi = parse_size(c + 1);
            i++;
        } else if (!strcmp(a, "--seed") && v) { g_seed = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--size") && v) { g_bsize = parse_size(v); i++; }
        else if (!strcmp(a, "--prot") && v) { g_prot = !strcmp(v, "rw") ? PROT_READ | PROT_WRITE : PROT_NONE; i++; }
        else if (!strcmp(a, "--thread")) g_thread = 1;
        else if (!strcmp(a, "--pool-maxsize") && v) { g_pool_maxsize = parse_size(v); i++; }
        else if (!strcmp(a, "--maps-dir") && v) { g_maps_dir = v; i++; }
        else if (!strcmp(a, "--tag") && v) { g_tag = v; i++; }
        else if (!strcmp(a, "--pre-reserve") && v) {
            char *dup = strdup(v), *tok, *save = NULL;
            for (tok = strtok_r(dup, ",", &save); tok && g_npre < MAX_PRE;
                 tok = strtok_r(NULL, ",", &save)) {
                char *c = strchr(tok, ':');
                if (!c) { fprintf(stderr, "--pre-reserve ADDR:SIZE[,...]\n"); return 2; }
                *c = 0;
                g_pre_addr[g_npre] = parse_size(tok);
                g_pre_size[g_npre] = parse_size(c + 1);
                g_npre++;
            }
            free(dup);
            i++;
        }
        else { fprintf(stderr, "unknown/incomplete option: %s\n", a); return 2; }
    }
    if (g_nrandom + g_nat > MAX_BLOCKERS) { fprintf(stderr, "too many blockers\n"); return 2; }

    printf("INFO pid=%d blockers_at=%d random=%d range=0x%" PRIx64 ":0x%" PRIx64
           " seed=%" PRIu64 " size=%" PRIu64 " prot=%s thread=%d\n",
           (int)getpid(), g_nat, g_nrandom, g_lo, g_hi, g_seed, g_bsize,
           g_prot == PROT_NONE ? "none" : "rw", g_thread);
    place_blockers();
    save_maps("0-before-init");
    scan_pieces("before_cuInit", 0);

    if (g_thread) {
        pthread_t t;
        pthread_create(&t, NULL, cuda_body, NULL);
        pthread_join(t, NULL);
    } else {
        cuda_body(NULL);
    }

    int pass = (r_defpool == CUDA_SUCCESS && r_async == CUDA_SUCCESS);
    printf("RESULT tag=%s outcome=%s placed=%d place_fail=%d init=%d ctx=%d attr_pools=%d "
           "free=%zu total=%zu alloc_sync=%d default_pool=%d alloc_async=%d free_async=%d "
           "pool_create=%d pool_create_maxsize=%d resv_2M=%d resv_64M=%d resv_1G=%d resv_32G=%d "
           "set_mempool=%d setpool_async_4=%d setpool_async_256M=%d pre_reserve_ok=%d/%d pieces=%d sum_gib=%.2f max_piece_gib=%.2f layout=",
           g_tag, pass ? "pass" : "FAIL", g_placed, g_place_fail, r_init, r_ctx, attr_pools,
           mem_free, mem_total, r_alloc, r_defpool, r_async, r_freeasync == CUDA_ERROR_NOT_PERMITTED ? -1 : (int)r_freeasync,
           r_poolcreate, r_poolcreate_ms == CUDA_ERROR_NOT_PERMITTED ? -1 : (int)r_poolcreate_ms,
           r_resv[0], r_resv[1], r_resv[2], r_resv[3],
           r_setpool == CUDA_ERROR_NOT_PERMITTED ? -1 : (int)r_setpool,
           r_setpool_async == CUDA_ERROR_NOT_PERMITTED ? -1 : (int)r_setpool_async,
           r_setpool_async_big == CUDA_ERROR_NOT_PERMITTED ? -1 : (int)r_setpool_async_big, g_pre_ok, g_npre, n_pieces,
           (double)sum_pieces / GiB, (double)max_piece / GiB);
    for (int i = 0; i < n_pieces && i < MAX_PIECES; i++)
        printf("%s0x%" PRIx64 "+%.2f", i ? "," : "", piece_start[i], (double)piece_size[i] / GiB);
    printf("\n");
    return pass ? 0 : 1;
}
