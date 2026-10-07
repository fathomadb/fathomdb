/*
 * pool_gap.c - 0.8.28 pool study, protocol section 4.6 (contiguous need).
 *
 * Measures how much contiguous free GPU virtual address space an explicit
 * pool of a given maxSize needs, by leaving exactly one free extent of G bytes
 * and making every other free extent small, then creating the pool.
 * Derived from pool_va_repro.c --pre-reserve (fences) and the
 * pool_teardown.c hole-filling step.
 *
 * Build: gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include \
 *            pool_gap.c -o pool_gap -lcuda
 * Usage: pool_gap --maxsize MiB [--gap MiB] [--in-unit] [--no-fill]
 *                 [--spacing MiB] [--hold MiB] [--maps-dir DIR] [--tag S]
 *
 * Layout: no blockers before cuInit (the driver makes its unobstructed
 * reservation: one 61.36 GiB unit at 8 GiB). After the primary context is
 * current:
 *   - every driver reservation (anonymous ---p mapping >= 1 GiB in
 *     [8 GiB, 128 GiB)) is cut by 2 MiB cuMemAddressReserve fences every
 *     --spacing MiB (default 128), so no free GPU VA extent inside a
 *     reservation exceeds about --spacing MiB;
 *   - every unmapped hole of the window gets a 4 KiB PROT_NONE blocker every
 *     --spacing MiB, so no unmapped hole exceeds --spacing MiB;
 *   - except one extent of exactly --gap MiB: with --in-unit, a fence-free
 *     extent inside the largest driver reservation, starting 16 GiB into it
 *     and bounded by fences; otherwise (between units) an unmapped hole at the
 *     start of the largest unmapped hole, bounded by a blocker.
 *   With --gap 0 there is no exceptional extent (negative control).
 *   With --no-fill nothing is fenced or blocked (the unfenced control used to
 *   locate the control ceiling); --hold MiB first cuMemAlloc's that much
 *   device memory and keeps it (tests whether the ceiling tracks free memory).
 * Then: cuMemPoolCreate(maxSize) -> cuDeviceSetMemPool -> cuMemAllocAsync(4)
 * + cuMemFreeAsync + cuCtxSynchronize (the probe). /proc/self/maps is read
 * before and after creation; new_mmap_bytes is the address space mapped
 * after creation that no mapping covered before. The default pool is never queried.
 * Output: one RESULT line:
 *   RESULT gap_mib maxsize_mib in_unit create_rc set_rc probe_rc new_mmap_bytes
 *   [extra key=value fields]
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
#define WLO (8 * GiB)
#define WHI (128 * GiB)
#define MAXR 16384
#define FENCE (2 * MiB)

typedef struct { unsigned long long lo, hi; int anon_none; } range;
typedef struct { range r[MAXR]; int n; } rset;

static const char *nm(CUresult r) { const char *n = "?"; cuGetErrorName(r, &n); return n ? n : "?"; }

/* Every mapping (unmerged) from /proc/self/maps; anon_none marks anonymous
 * PROT_NONE private mappings (inode 0, ---p). */
static void snap_all(rset *s) {
    FILE *f = fopen("/proc/self/maps", "r");
    char line[1024];
    s->n = 0;
    while (f && fgets(line, sizeof line, f) && s->n < MAXR) {
        unsigned long long lo, hi, off;
        char perms[8], dev[16];
        unsigned long inode;
        if (sscanf(line, "%llx-%llx %7s %llx %15s %lu", &lo, &hi, perms, &off, dev, &inode) < 6) continue;
        s->r[s->n].lo = lo;
        s->r[s->n].hi = hi;
        s->r[s->n].anon_none = !strcmp(perms, "---p") && inode == 0;
        s->n++;
    }
    if (f) fclose(f);
}

static void save_maps(const char *dir, const char *stage) {
    if (!dir) return;
    char path[4096];
    snprintf(path, sizeof path, "%s/maps-%d-%s.txt", dir, (int)getpid(), stage);
    FILE *in = fopen("/proc/self/maps", "r"), *out = fopen(path, "w");
    char buf[8192];
    size_t k;
    while (in && out && (k = fread(buf, 1, sizeof buf, in)) > 0) fwrite(buf, 1, k, out);
    if (in) fclose(in);
    if (out) fclose(out);
}

/* Unmapped holes of the window, in address order. */
static int holes(const rset *s, range *out, int max) {
    unsigned long long prev = WLO;
    int n = 0;
    for (int i = 0; i < s->n && n < max; i++) {
        unsigned long long lo = s->r[i].lo, hi = s->r[i].hi;
        if (hi <= WLO || lo >= WHI) continue;
        if (lo > prev) { out[n].lo = prev; out[n].hi = lo; n++; }
        if (hi > prev) prev = hi;
    }
    if (prev < WHI && n < max) { out[n].lo = prev; out[n].hi = WHI; n++; }
    return n;
}

static unsigned long long largest(const range *r, int n, int *idx) {
    unsigned long long m = 0;
    for (int i = 0; i < n; i++)
        if (r[i].hi - r[i].lo > m) { m = r[i].hi - r[i].lo; if (idx) *idx = i; }
    return m;
}

static int block(unsigned long long at) {
    void *p = mmap((void *)at, 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    return p == (void *)at;
}

static int fences_placed, fences_moved, fences_failed;

static void fence(unsigned long long at) {
    CUdeviceptr va = 0;
    CUresult r = cuMemAddressReserve(&va, FENCE, 0, (CUdeviceptr)at, 0);
    if (r != CUDA_SUCCESS) { fences_failed++; return; }
    if (va != at) { cuMemAddressFree(va, FENCE); fences_moved++; return; }
    fences_placed++;
}

int main(int argc, char **argv) {
    size_t maxsize_mib = 0, gap_mib = 0, spacing_mib = 128, hold_mib = 0;
    int in_unit = 0, no_fill = 0;
    const char *maps_dir = NULL, *tag = "-";
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i], *v = i + 1 < argc ? argv[i + 1] : NULL;
        if (!strcmp(a, "--maxsize") && v) { maxsize_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--gap") && v) { gap_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--spacing") && v) { spacing_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--hold") && v) { hold_mib = strtoull(v, NULL, 0); i++; }
        else if (!strcmp(a, "--in-unit")) in_unit = 1;
        else if (!strcmp(a, "--no-fill")) no_fill = 1;
        else if (!strcmp(a, "--maps-dir") && v) { maps_dir = v; i++; }
        else if (!strcmp(a, "--tag") && v) { tag = v; i++; }
        else { fprintf(stderr, "unknown/incomplete option %s\n", a); return 2; }
    }
    if (!maxsize_mib || spacing_mib < 4 || (gap_mib && gap_mib % 2)) {
        fprintf(stderr, "--maxsize required; --spacing >= 4; --gap a multiple of 2 MiB\n");
        return 2;
    }
    if (hold_mib > 8192) { fprintf(stderr, "refusing --hold > 8 GiB\n"); return 2; }
    const unsigned long long spacing = spacing_mib * MiB, gap = gap_mib * MiB;

    CUdevice dev;
    CUcontext ctx;
    CUresult r_init = cuInit(0);
    if (r_init != CUDA_SUCCESS) { printf("RESULT tag=%s init=%s\n", tag, nm(r_init)); return 1; }
    int drv = 0;
    cuDriverGetVersion(&drv);
    cuDeviceGet(&dev, 0);
    CUresult r_ctx = cuDevicePrimaryCtxRetain(&ctx, dev);
    if (r_ctx != CUDA_SUCCESS) { printf("RESULT tag=%s ctx=%s\n", tag, nm(r_ctx)); return 1; }
    cuCtxSetCurrent(ctx);

    CUdeviceptr held = 0;
    CUresult r_hold = CUDA_SUCCESS;
    if (hold_mib) {
        r_hold = cuMemAlloc(&held, hold_mib * MiB);
        if (r_hold == CUDA_SUCCESS) cuMemsetD8(held, 0x5a, hold_mib * MiB);
    }
    size_t mem_free = 0, mem_total = 0;
    cuMemGetInfo(&mem_free, &mem_total);

    static rset s0, s1, s2;
    static range hs[MAXR], units[64];
    snap_all(&s0);
    save_maps(maps_dir, "0-after-ctx");
    int nunits = 0;
    for (int i = 0; i < s0.n && nunits < 64; i++)
        if (s0.r[i].anon_none && s0.r[i].hi - s0.r[i].lo >= GiB && s0.r[i].lo >= WLO && s0.r[i].hi <= WHI)
            units[nunits++] = s0.r[i];
    int ui = -1;
    unsigned long long unit_max = largest(units, nunits, &ui);

    unsigned long long hole_lo = 0, hole_hi = 0;
    int blockers = 0, hole_ok = -1;
    if (!no_fill) {
        int nh = holes(&s0, hs, MAXR), hi_idx = -1;
        largest(hs, nh, &hi_idx);
        if (gap && in_unit) {
            if (ui < 0 || units[ui].hi - units[ui].lo < 16 * GiB + gap + 2 * FENCE) {
                fprintf(stderr, "largest unit too small for the in-unit gap\n");
                return 2;
            }
            hole_lo = ((units[ui].lo + 16 * GiB) + FENCE - 1) & ~(FENCE - 1);
            hole_hi = hole_lo + gap;
        } else if (gap) {
            if (hi_idx < 0 || hs[hi_idx].hi - hs[hi_idx].lo < gap + 2 * 4096) {
                fprintf(stderr, "largest unmapped hole too small for the gap\n");
                return 2;
            }
            hole_lo = (hs[hi_idx].lo + 4095) & ~4095ULL;
            hole_hi = hole_lo + gap;
        }
        /* Fences through every driver reservation, plus exact bounds for an
         * in-unit gap; nothing inside [hole_lo - FENCE, hole_hi). */
        for (int u = 0; u < nunits; u++) {
            for (unsigned long long a = (units[u].lo + FENCE - 1) & ~(FENCE - 1); a + FENCE <= units[u].hi; a += spacing) {
                if (gap && in_unit && a + FENCE > hole_lo - FENCE && a < hole_hi + FENCE) continue;
                fence(a);
            }
        }
        if (gap && in_unit) {
            fence(hole_lo - FENCE);
            fence(hole_hi);
        }
        /* Blockers through every unmapped hole, except the between-units gap,
         * which is bounded by one blocker at its end. */
        for (int h = 0; h < nh; h++) {
            unsigned long long lo = (hs[h].lo + 4095) & ~4095ULL;
            if (gap && !in_unit && h == hi_idx) {
                blockers += block(hole_hi);
                lo = hole_hi + spacing;
            }
            for (unsigned long long a = lo; a + 4096 <= hs[h].hi; a += spacing) blockers += block(a);
        }
        /* The exceptional extent must be free: reserve it whole, then free. */
        if (gap) {
            CUdeviceptr va = 0;
            if (in_unit) {
                CUresult r = cuMemAddressReserve(&va, gap, 0, (CUdeviceptr)hole_lo, 0);
                hole_ok = r == CUDA_SUCCESS && va == hole_lo;
                if (r == CUDA_SUCCESS) cuMemAddressFree(va, gap);
            } else {
                void *p = mmap((void *)hole_lo, gap, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE | MAP_NORESERVE, -1, 0);
                hole_ok = p == (void *)hole_lo;
                if (p != MAP_FAILED) munmap(p, gap);
            }
        }
    }
    snap_all(&s1);
    save_maps(maps_dir, "1-before-create");
    int nh1 = holes(&s1, hs, MAXR);
    unsigned long long largest_unmapped = largest(hs, nh1, NULL);

    CUmemPoolProps props;
    memset(&props, 0, sizeof props);
    props.allocType = CU_MEM_ALLOCATION_TYPE_PINNED;
    props.handleTypes = CU_MEM_HANDLE_TYPE_NONE;
    props.location.type = CU_MEM_LOCATION_TYPE_DEVICE;
    props.location.id = (int)dev;
    props.maxSize = maxsize_mib * MiB;
    CUmemoryPool pool = NULL;
    CUresult r_create = cuMemPoolCreate(&pool, &props);
    CUresult r_set = CUDA_ERROR_NOT_PERMITTED, r_probe = CUDA_ERROR_NOT_PERMITTED, r_free = CUDA_ERROR_NOT_PERMITTED,
             r_sync = CUDA_ERROR_NOT_PERMITTED;
    if (r_create == CUDA_SUCCESS) {
        r_set = cuDeviceSetMemPool(dev, pool);
        if (r_set == CUDA_SUCCESS) {
            CUdeviceptr p = 0;
            r_probe = cuMemAllocAsync(&p, 4, NULL);
            if (r_probe == CUDA_SUCCESS) r_free = cuMemFreeAsync(p, NULL);
            r_sync = cuCtxSynchronize();
        }
    }
    snap_all(&s2);
    save_maps(maps_dir, "2-after-create");
    /* Address space mapped after creation that no mapping covered before
     * (VMA splits and permission changes inside existing mappings excluded). */
    unsigned long long new_bytes = 0;
    for (int i = 0; i < s2.n; i++) {
        unsigned long long lo = s2.r[i].lo, hi = s2.r[i].hi, covered = 0;
        for (int j = 0; j < s1.n; j++) {
            unsigned long long a = s1.r[j].lo > lo ? s1.r[j].lo : lo, b = s1.r[j].hi < hi ? s1.r[j].hi : hi;
            if (b > a) covered += b - a;
        }
        new_bytes += (hi - lo) - covered;
    }
    printf("RESULT tag=%s gap_mib=%zu maxsize_mib=%zu in_unit=%d create_rc=%s set_rc=%s probe_rc=%s new_mmap_bytes=%llu "
           "no_fill=%d spacing_mib=%zu units=%d unit_max_mib=%llu hole_lo=%#llx hole_ok=%d fences=%d fences_moved=%d "
           "fences_failed=%d blockers=%d largest_unmapped_mib=%llu free_rc=%s sync_rc=%s hold_mib=%zu hold_rc=%s "
           "mem_free=%zu mem_total=%zu driver=%d\n",
           tag, gap_mib, maxsize_mib, in_unit, nm(r_create), nm(r_set), nm(r_probe), new_bytes, no_fill, spacing_mib, nunits,
           unit_max / MiB, hole_lo, hole_ok, fences_placed, fences_moved, fences_failed, blockers, largest_unmapped / MiB,
           nm(r_free), nm(r_sync), hold_mib, nm(r_hold), mem_free, mem_total, drv);
    if (held) cuMemFree(held);
    cuDevicePrimaryCtxRelease(dev);
    return 0;
}
