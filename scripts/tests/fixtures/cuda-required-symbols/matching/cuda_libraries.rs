pub(crate) const REQUIRED_CUBLAS_SYMBOLS: [&str; 3] = [
    "cublasCreate_v2",
    "cublasDestroy_v2",
    "cublasSgemm_v2",
];

pub(crate) const REQUIRED_CURAND_SYMBOLS: [&str; 2] = [
    "curandCreateGenerator",
    "curandDestroyGenerator",
];
