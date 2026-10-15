/*
 * 0.8.28 Slice 31 (R12, CUDA-04): a decoy cuBLAS installed as both
 * libcublas.so.12 and libcublas.so. Its version query reports 13.1.1 and it
 * exports nothing else, so a loader that accepted it would fail at the first
 * real cuBLAS call. FathomDB must refuse it before any GPU work.
 */
typedef int cublasStatus_t;
typedef enum { MAJOR_VERSION = 0, MINOR_VERSION = 1, PATCH_LEVEL = 2 } libraryPropertyType;

cublasStatus_t cublasGetProperty(libraryPropertyType type, int *value) {
    switch (type) {
    case MAJOR_VERSION: *value = 13; break;
    case MINOR_VERSION: *value = 1; break;
    default: *value = 1; break;
    }
    return 0;
}
