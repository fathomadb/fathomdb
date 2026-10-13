#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_reason_strings_are_missing_wrong_major_and_missing_symbol() {
        assert_eq!(CudaLibraryRefusalReason::Missing.as_str(), "missing");
        assert_eq!(CudaLibraryRefusalReason::WrongMajor.as_str(), "wrong_major");
        assert_eq!(CudaLibraryRefusalReason::MissingSymbol.as_str(), "missing_symbol");
    }

    #[test]
    fn the_refusal_message_names_library_file_version_and_major() {
        let refusal = CudaLibraryRefusal::new(
            "cublas",
            CudaLibraryRefusalReason::WrongMajor,
            Some("/opt/decoy/libcublas.so.12".to_owned()),
            Some("13.1.1".to_owned()),
            12,
            vec![("libcublas.so.12".to_owned(), "opened".to_owned())],
            Vec::new(),
        );
        let message = refusal.to_string();
        for needle in ["cublas", "wrong_major", "/opt/decoy/libcublas.so.12", "13.1.1", "major 12"]
        {
            assert!(message.contains(needle), "{needle:?} missing from {message:?}");
        }
        assert!(
            message.contains("remove"),
            "a wrong-major refusal says which file to remove: {message}"
        );
    }

    #[test]
    fn missing_required_symbols_lists_each_absent_name_in_set_order() {
        let set =
            ["cublasCreate_v2", "cublasSgemmStridedBatched", "cublasGemmEx", "cublasDestroy_v2"];
        let absent = ["cublasDestroy_v2", "cublasSgemmStridedBatched"];
        let missing = missing_required_symbols(&set, |name| !absent.contains(&name));
        assert_eq!(
            missing,
            vec!["cublasSgemmStridedBatched".to_owned(), "cublasDestroy_v2".to_owned()]
        );
        assert!(missing_required_symbols(&set, |_| true).is_empty());
        assert_eq!(REQUIRED_CUBLAS_SYMBOLS.len(), 13);
        assert_eq!(REQUIRED_CURAND_SYMBOLS.len(), 14);
    }

    #[test]
    fn a_missing_symbol_refusal_names_the_symbols_and_the_remedy() {
        let refusal = CudaLibraryRefusal::new(
            "cublas",
            CudaLibraryRefusalReason::MissingSymbol,
            Some("/opt/old/libcublas.so.12".to_owned()),
            Some("12.0.1".to_owned()),
            12,
            Vec::new(),
            vec!["cublasSgemmStridedBatched".to_owned()],
        );
        assert_eq!(refusal.missing_symbols, vec!["cublasSgemmStridedBatched".to_owned()]);
        let message = refusal.to_string();
        for needle in [
            "missing_symbol",
            "cublasSgemmStridedBatched",
            "/opt/old/libcublas.so.12",
            "pip install nvidia-cublas-cu12 nvidia-curand-cu12",
            "nvidia.cublas",
            "nvidia.curand",
            "LD_LIBRARY_PATH",
        ] {
            assert!(message.contains(needle), "{needle:?} missing from {message:?}");
        }
    }
}
