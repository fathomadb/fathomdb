/*
 * pool_teardown.c - does the Tegra default memory pool stay obtainable after
 * the context that obtained it is torn down and the address space is then
 * fragmented?
 *
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include pool_teardown.c -o pool_teardown -lcuda
 * Usage: ./pool_teardown primary|nonprimary [ADDR,ADDR,...|-] [OPTION...]
 *   ADDRs (hex, or "-" for none): 4 KiB PROT_NONE pages mapped before cuInit to shape the
 *   driver's reservations, e.g. the free-gap layout
 *   0x800000000,0xe00000000,0x1400000000,0x1a00000000,0x1f40001000 forces
 *   15.34 GiB reservation quarters and leaves one 21 GiB hole, so the default
 *   pool is created by a new mmap into that hole rather than inside a
 *   reservation.
 *
 * Sequence, in an unobstructed process:
 *   1. cuInit; snapshot the [8 GiB, 128 GiB) window (W0).
 *   2. primary: cuDevicePrimaryCtxRetain + set current;
 *      nonprimary: cuCtxCreate.
 *      Query the default pool and cuMemAllocAsync(4) (+ free, sync).
 *      Snapshot the window (W1).
 *   3. primary: cuDevicePrimaryCtxRelease (refcount 0; checked with
 *      cuDevicePrimaryCtxGetState); nonprimary: cuCtxDestroy. Snapshot (W2).
 *   4. Map 4 KiB PROT_NONE blockers every 4 GiB through every range that was
 *      mapped in W1 and not in W2, then the three standard blockers at 38, 68
 *      and 98 GiB (MAP_FIXED_NOREPLACE; an occupied address is reported, not
 *      forced), then a blocker every 4 GiB through every remaining unmapped
 *      hole, so no hole of 4 GiB or more is left in the window and the default
 *      pool could not be re-created anywhere outside existing mappings.
 *      Snapshot (W3) and report the largest unmapped hole.
 *   5. Retain / create again, query the default pool (and whether the handle
 *      is the same), cuMemAllocAsync(4), and cuMemAlloc(4).
 *   An explicit pool (cuMemPoolCreate, maxSize 1 GiB) is created in step 2 and
 *   allocated from again in step 5.
 * OPTIONs:
 *   explicit=keep     (default) the explicit pool above stays alive across
 *                     teardown;
 *   explicit=none     no explicit pool is ever created, so nothing but the
 *                     default pool can hold window mappings across teardown;
 *   explicit=destroy  the explicit pool is created and allocated from in step
 *                     2, then destroyed with cuMemPoolDestroy before teardown.
 *   fill-before-pool  a control: right after the context exists (step 2) and
 *                     before the first cuDeviceGetDefaultMemPool, map a
 *                     blocker every 4 GiB through every unmapped hole of the
 *                     window. The default pool then has no room for the range
 *                     it maps lazily, which shows that blockers placed after
 *                     cuInit are able to make it unavailable.
 * A summary line "RESULT ..." is printed last.
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif
#define GiB (1ULL << 30)
#define MiB (1ULL << 20)
#define WLO (8 * GiB)
#define WHI (128 * GiB)
#define MAXR 4096

typedef struct { unsigned long long lo, hi; } range;
typedef struct { range r[MAXR]; int n; } rset;

static const char *nm(CUresult r) { const char *n = "?"; cuGetErrorName(r, &n); return n; }
#define P(what, expr) ({ CUresult r_p_ = (expr); printf("%-34s -> %d (%s)\n", what, (int)r_p_, nm(r_p_)); r_p_; })

/* Merged union of every mapping intersecting the window. */
static void snap(rset *s) {
    FILE *f = fopen("/proc/self/maps", "r");
    char line[512];
    s->n = 0;
    while (f && fgets(line, sizeof line, f)) {
        unsigned long long lo, hi;
        if (sscanf(line, "%llx-%llx", &lo, &hi) != 2) continue;
        if (hi <= WLO || lo >= WHI) continue;
        if (lo < WLO) lo = WLO;
        if (hi > WHI) hi = WHI;
        if (s->n && lo <= s->r[s->n - 1].hi) {
            if (hi > s->r[s->n - 1].hi) s->r[s->n - 1].hi = hi;
        } else if (s->n < MAXR) {
            s->r[s->n].lo = lo; s->r[s->n].hi = hi; s->n++;
        }
    }
    if (f) fclose(f);
}

static unsigned long long total(const rset *s) {
    unsigned long long t = 0;
    for (int i = 0; i < s->n; i++) t += s->r[i].hi - s->r[i].lo;
    return t;
}

static unsigned long long largest_piece(const rset *s) {
    unsigned long long m = 0;
    for (int i = 0; i < s->n; i++) if (s->r[i].hi - s->r[i].lo > m) m = s->r[i].hi - s->r[i].lo;
    return m;
}

static unsigned long long largest_hole(const rset *s) {
    unsigned long long prev = WLO, m = 0;
    for (int i = 0; i < s->n; i++) {
        if (s->r[i].lo > prev && s->r[i].lo - prev > m) m = s->r[i].lo - prev;
        prev = s->r[i].hi;
    }
    if (WHI > prev && WHI - prev > m) m = WHI - prev;
    return m;
}

static void show(const char *label, const rset *s) {
    printf("%s: %d ranges, mapped=%.2f GiB, largest_piece=%.2f GiB, largest_hole=%.2f GiB\n",
           label, s->n, total(s) / (double)GiB, largest_piece(s) / (double)GiB, largest_hole(s) / (double)GiB);
    for (int i = 0; i < s->n; i++)
        if (s->r[i].hi - s->r[i].lo >= GiB)
            printf("  [%#llx, %#llx) %.2f GiB\n", s->r[i].lo, s->r[i].hi, (s->r[i].hi - s->r[i].lo) / (double)GiB);
}

/* a minus b, both merged and sorted. */
static void diff(const rset *a, const rset *b, rset *out) {
    out->n = 0;
    for (int i = 0; i < a->n; i++) {
        unsigned long long lo = a->r[i].lo, hi = a->r[i].hi;
        for (int j = 0; j < b->n && lo < hi; j++) {
            if (b->r[j].hi <= lo || b->r[j].lo >= hi) continue;
            if (b->r[j].lo > lo && out->n < MAXR) { out->r[out->n].lo = lo; out->r[out->n].hi = b->r[j].lo; out->n++; }
            lo = b->r[j].hi;
        }
        if (lo < hi && out->n < MAXR) { out->r[out->n].lo = lo; out->r[out->n].hi = hi; out->n++; }
    }
}

static int block(unsigned long long at) {
    void *p = mmap((void *)at, 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    return p == (void *)at;
}

/* A blocker every 4 GiB through every unmapped hole of the window. */
static const char *explicit_outcome(int created, CUresult r, const char *mode) {
    if (!created) return "not_created";
    if (strcmp(mode, "destroy") == 0 && r == CUDA_ERROR_UNKNOWN) return "destroyed";
    return nm(r);
}

static int fill_holes(void) {
    rset holes_src; snap(&holes_src);
    int fill = 0;
    unsigned long long prev = WLO;
    for (int i = 0; i <= holes_src.n; i++) {
        unsigned long long lo = prev, hi = i < holes_src.n ? holes_src.r[i].lo : WHI;
        for (unsigned long long a = lo; a + 4096 <= hi; a += 4 * GiB) fill += block(a);
        if (i < holes_src.n) prev = holes_src.r[i].hi;
    }
    return fill;
}

int main(int argc, char **argv) {
    int primary = !(argc > 1 && strcmp(argv[1], "nonprimary") == 0);
    const char *explicit_mode = "keep";
    int fill_before_pool = 0;
    for (int i = 3; i < argc; i++) {
        if (strncmp(argv[i], "explicit=", 9) == 0) explicit_mode = argv[i] + 9;
        else if (strcmp(argv[i], "fill-before-pool") == 0) fill_before_pool = 1;
        else { fprintf(stderr, "unknown option %s\n", argv[i]); return 2; }
    }
    if (strcmp(explicit_mode, "keep") && strcmp(explicit_mode, "none") && strcmp(explicit_mode, "destroy")) {
        fprintf(stderr, "explicit= must be keep, none or destroy\n");
        return 2;
    }
    rset w0, w1, w2, w3, freed;
    CUdevice dev;
    CUcontext ctx;
    CUmemoryPool pool1 = NULL, pool2 = NULL;
    CUdeviceptr p;

    printf("mode=%s explicit=%s fill_before_pool=%d\n", primary ? "primary" : "nonprimary", explicit_mode, fill_before_pool);
    int pre = 0;
    if (argc > 2 && strcmp(argv[2], "-") != 0) {
        char *list = strdup(argv[2]);
        for (char *t = strtok(list, ","); t; t = strtok(NULL, ",")) {
            unsigned long long at = strtoull(t, NULL, 16);
            int ok = block(at);
            printf("pre-cuInit blocker at %#llx -> %s\n", at, ok ? "mapped" : "REFUSED");
            pre += ok;
        }
        free(list);
    }
    P("cuInit", cuInit(0));
    P("cuDeviceGet", cuDeviceGet(&dev, 0));
    snap(&w0); show("W0 after cuInit", &w0);

    if (primary) {
        P("cuDevicePrimaryCtxRetain", cuDevicePrimaryCtxRetain(&ctx, dev));
        P("cuCtxSetCurrent", cuCtxSetCurrent(ctx));
    } else {
        P("cuCtxCreate", cuCtxCreate(&ctx, 0, dev));
    }
    int early_fill = fill_before_pool ? fill_holes() : 0;
    if (fill_before_pool) {
        rset wf; snap(&wf);
        printf("blockers placed before the first default-pool query=%d\n", early_fill);
        show("WF after early hole filling", &wf);
    }
    CUresult pool_before = P("cuDeviceGetDefaultMemPool", cuDeviceGetDefaultMemPool(&pool1, dev));
    CUresult async_before = P("cuMemAllocAsync(4)", cuMemAllocAsync(&p, 4, NULL));
    if (async_before == CUDA_SUCCESS) {
        P("cuMemFreeAsync", cuMemFreeAsync(p, NULL));
        P("cuCtxSynchronize", cuCtxSynchronize());
    }
    CUmemPoolProps props;
    memset(&props, 0, sizeof props);
    props.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    props.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    props.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    props.location.id = (int)dev;
    props.maxSize = GiB;
    CUmemoryPool explicit_pool = NULL;
    CUresult explicit_create = CUDA_ERROR_NOT_PERMITTED;
    if (strcmp(explicit_mode, "none") != 0)
        explicit_create = P("cuMemPoolCreate(maxSize 1 GiB)", cuMemPoolCreate(&explicit_pool, &props));
    CUresult explicit_before = CUDA_ERROR_UNKNOWN;
    if (explicit_create == CUDA_SUCCESS) {
        explicit_before = P("cuMemAllocFromPoolAsync(4)", cuMemAllocFromPoolAsync(&p, 4, explicit_pool, NULL));
        if (explicit_before == CUDA_SUCCESS) { cuMemFreeAsync(p, NULL); cuCtxSynchronize(); }
    }
    if (explicit_create == CUDA_SUCCESS && strcmp(explicit_mode, "destroy") == 0) {
        P("cuMemPoolDestroy(explicit)", cuMemPoolDestroy(explicit_pool));
        explicit_pool = NULL;
    }
    snap(&w1); show("W1 after pool + async alloc", &w1);

    if (primary) {
        P("cuDevicePrimaryCtxRelease", cuDevicePrimaryCtxRelease(dev));
        unsigned int flags = 0; int active = -1;
        P("cuDevicePrimaryCtxGetState", cuDevicePrimaryCtxGetState(dev, &flags, &active));
        printf("primary context active after release: %d\n", active);
    } else {
        P("cuCtxDestroy", cuCtxDestroy(ctx));
    }
    snap(&w2); show("W2 after teardown", &w2);
    diff(&w1, &w2, &freed);
    rset added01; diff(&w1, &w0, &added01);
    printf("window ranges added by context+pool (W1-W0): %d, %.2f GiB\n", added01.n, total(&added01) / (double)GiB);
    printf("window ranges unmapped by teardown (W1-W2): %d, %.2f GiB\n", freed.n, total(&freed) / (double)GiB);

    int placed = 0, refused = 0;
    for (int i = 0; i < freed.n; i++) {
        for (unsigned long long a = (freed.r[i].lo + 4095) & ~4095ULL; a < freed.r[i].hi; a += 4 * GiB) {
            if (block(a)) placed++; else refused++;
        }
    }
    const unsigned long long std_at[3] = {38 * GiB, 68 * GiB, 98 * GiB};
    int std_ok = 0;
    for (int i = 0; i < 3; i++) std_ok += block(std_at[i]);
    printf("blockers in freed ranges: placed=%d refused=%d; standard blockers placed=%d/3\n", placed, refused, std_ok);
    int fill = fill_holes();
    printf("hole-filling blockers placed=%d\n", fill);
    snap(&w3); show("W3 after blockers", &w3);

    if (primary) {
        P("cuDevicePrimaryCtxRetain (again)", cuDevicePrimaryCtxRetain(&ctx, dev));
        P("cuCtxSetCurrent", cuCtxSetCurrent(ctx));
    } else {
        P("cuCtxCreate (again)", cuCtxCreate(&ctx, 0, dev));
    }
    CUresult pool_after = P("cuDeviceGetDefaultMemPool", cuDeviceGetDefaultMemPool(&pool2, dev));
    printf("same default pool handle: %s\n", pool_after == CUDA_SUCCESS && pool1 == pool2 ? "yes" : "no");
    CUresult async_after = P("cuMemAllocAsync(4)", cuMemAllocAsync(&p, 4, NULL));
    if (async_after == CUDA_SUCCESS) {
        P("cuMemFreeAsync", cuMemFreeAsync(p, NULL));
        P("cuCtxSynchronize", cuCtxSynchronize());
    }
    CUresult sync_after = P("cuMemAlloc(4)", cuMemAlloc(&p, 4));
    if (sync_after == CUDA_SUCCESS) cuMemFree(p);
    CUresult explicit_after = CUDA_ERROR_UNKNOWN;
    if (explicit_create == CUDA_SUCCESS && explicit_pool != NULL) {
        explicit_after = P("cuMemAllocFromPoolAsync(4) again", cuMemAllocFromPoolAsync(&p, 4, explicit_pool, NULL));
        if (explicit_after == CUDA_SUCCESS) { cuMemFreeAsync(p, NULL); cuCtxSynchronize(); }
    }
    rset w4; snap(&w4); show("W4 after re-retain", &w4);

    printf("RESULT pre_blockers=%d mode=%s explicit=%s fill_before_pool=%d pool_before=%s async_before=%s added_GiB=%.2f freed_GiB=%.2f freed_blockers=%d std_blockers=%d "
           "hole_after_blockers_GiB=%.2f pool_after=%s same_pool=%d async_after=%s sync_after=%s explicit_before=%s explicit_after=%s\n",
           pre, primary ? "primary" : "nonprimary", explicit_mode, fill_before_pool, nm(pool_before), nm(async_before), total(&added01) / (double)GiB,
           total(&freed) / (double)GiB, placed, std_ok, largest_hole(&w3) / (double)GiB, nm(pool_after),
           pool_after == CUDA_SUCCESS && pool1 == pool2, nm(async_after), nm(sync_after),
           explicit_outcome(explicit_create == CUDA_SUCCESS, explicit_before, "keep"),
           explicit_outcome(explicit_create == CUDA_SUCCESS, explicit_after, explicit_mode));
    if (primary) cuDevicePrimaryCtxRelease(dev); else cuCtxDestroy(ctx);
    return 0;
}
