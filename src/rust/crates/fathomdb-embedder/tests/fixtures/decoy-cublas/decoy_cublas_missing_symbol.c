/*
 * 0.8.28 Slice 31 (R12b, CUDA-11, owner ruling Q26): a decoy CUDA 12 cuBLAS,
 * installed as both libcublas.so.12 and libcublas.so, that reports 12.6.4 and
 * exports every name of REQUIRED_CUBLAS_SYMBOLS as a stub returning
 * CUBLAS_STATUS_SUCCESS (cublasCreate_v2 hands out a non-null handle) except
 * cublasSgemmStridedBatched. FathomDB must refuse it, naming that symbol,
 * before any device is built.
 */
typedef int cublasStatus_t;
typedef enum { MAJOR_VERSION = 0, MINOR_VERSION = 1, PATCH_LEVEL = 2 } libraryPropertyType;

static int decoy_handle;

cublasStatus_t cublasGetProperty(libraryPropertyType type, int *value) {
    switch (type) {
    case MAJOR_VERSION: *value = 12; break;
    case MINOR_VERSION: *value = 6; break;
    default: *value = 4; break;
    }
    return 0;
}

cublasStatus_t cublasCreate_v2(void **handle) {
    *handle = &decoy_handle;
    return 0;
}

cublasStatus_t cublasDestroy_v2(void) { return 0; }

cublasStatus_t cublasSetStream_v2(void) { return 0; }

cublasStatus_t cublasSgemm_v2(void) { return 0; }

cublasStatus_t cublasDgemm_v2(void) { return 0; }

cublasStatus_t cublasSgemv_v2(void) { return 0; }

cublasStatus_t cublasDgemv_v2(void) { return 0; }

cublasStatus_t cublasSasum_v2(void) { return 0; }

cublasStatus_t cublasDasum_v2(void) { return 0; }

cublasStatus_t cublasGemmEx(void) { return 0; }

cublasStatus_t cublasGemmStridedBatchedEx(void) { return 0; }

cublasStatus_t cublasDgemmStridedBatched(void) { return 0; }
