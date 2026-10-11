/*
 * minimal_repro_runtime.c - same as minimal_repro.c but via the CUDA Runtime API
 * (cudaMallocAsync on the default pool).
 *
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include minimal_repro_runtime.c -o minimal_repro_runtime \
 *            -L/usr/local/cuda/lib64 -lcudart
 * Run:   ./minimal_repro_runtime [control]
 */
#define _GNU_SOURCE
#include <cuda_runtime_api.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

int main(int argc, char **argv) {
    if (!(argc > 1 && strcmp(argv[1], "control") == 0)) {
        const unsigned long long gib = 1ULL << 30;
        const unsigned long long at[3] = {38 * gib, 68 * gib, 98 * gib};
        for (int i = 0; i < 3; i++)
            printf("mmap 4 KiB PROT_NONE at %#llx -> %p\n", at[i],
                   mmap((void *)at[i], 4096, PROT_NONE,
                        MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0));
    }
    void *p = NULL;
    cudaError_t e = cudaMalloc(&p, 4);
    printf("cudaMalloc(4)          -> %d (%s)\n", (int)e, cudaGetErrorName(e));
    if (e == cudaSuccess) cudaFree(p);
    p = NULL;
    e = cudaMallocAsync(&p, 4, 0);
    printf("cudaMallocAsync(4, 0)  -> %d (%s)\n", (int)e, cudaGetErrorName(e));
    if (e == cudaSuccess) {
        cudaFreeAsync(p, 0);
        cudaError_t s = cudaStreamSynchronize(0);
        printf("cudaStreamSynchronize  -> %d (%s)\n", (int)s, cudaGetErrorName(s));
    }
    return e == cudaSuccess ? 0 : 1;
}
