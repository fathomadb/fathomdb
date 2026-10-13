//! D-18: the CUDA libraries a CUDA device needs are checked before the
//! device is built (0.8.28 Slice 31, design § 9.1).
//!
//! Candle's device construction loads cuBLAS and cuRAND and panics when one
//! is missing, and cudarc resolves each library symbol lazily, panicking at
//! its first call. This module refuses, before any GPU work, a library that
//! cannot be loaded for the built CUDA major, a cuBLAS that reports another
//! major, or a library that lacks a symbol FathomDB's CUDA path will call,
//! as a typed [`CudaLibraryRefusal`]. The record, its message and the
//! required symbol sets are host-independent; the loading checks live in the
//! CUDA driver part.

use std::fmt;

/// The cuBLAS functions FathomDB's CUDA path can call: every `sys::cublas*`
/// function cudarc's `cublas/result.rs` calls (Candle reaches cuBLAS only
/// through that layer). `scripts/check-cuda-required-symbols.py` keeps this
/// list equal to the resolved cudarc source.
#[cfg_attr(not(any(feature = "embed-cuda", feature = "rerank-cuda")), allow(dead_code))]
pub(crate) const REQUIRED_CUBLAS_SYMBOLS: [&str; 13] = [
    "cublasCreate_v2",
    "cublasDestroy_v2",
    "cublasSetStream_v2",
    "cublasSgemm_v2",
    "cublasDgemm_v2",
    "cublasSgemv_v2",
    "cublasDgemv_v2",
    "cublasSasum_v2",
    "cublasDasum_v2",
    "cublasGemmEx",
    "cublasGemmStridedBatchedEx",
    "cublasSgemmStridedBatched",
    "cublasDgemmStridedBatched",
];

/// The cuRAND functions FathomDB's CUDA path can call: every `sys::curand*`
/// function cudarc's `curand/result.rs` calls. Kept equal to the resolved
/// cudarc source by `scripts/check-cuda-required-symbols.py`.
#[cfg_attr(not(any(feature = "embed-cuda", feature = "rerank-cuda")), allow(dead_code))]
pub(crate) const REQUIRED_CURAND_SYMBOLS: [&str; 14] = [
    "curandCreateGenerator",
    "curandDestroyGenerator",
    "curandSetStream",
    "curandSetPseudoRandomGeneratorSeed",
    "curandSetGeneratorOffset",
    "curandGenerate",
    "curandGenerateLongLong",
    "curandGenerateUniform",
    "curandGenerateUniformDouble",
    "curandGenerateNormal",
    "curandGenerateNormalDouble",
    "curandGenerateLogNormal",
    "curandGenerateLogNormalDouble",
    "curandGeneratePoisson",
];

/// The remedy for a missing (or too old) CUDA 12 cuBLAS or cuRAND. The CUDA
/// library loader does not search Python's site-packages on Linux, so the
/// wheel directories must be put on `LD_LIBRARY_PATH`.
const CUDA_12_WHEEL_REMEDY: &str = "install the CUDA 12 cuBLAS and cuRAND wheels \
(`pip install nvidia-cublas-cu12 nvidia-curand-cu12`) and put both library directories on \
`LD_LIBRARY_PATH`, for example `export LD_LIBRARY_PATH=\"$(python3 -c 'import nvidia.cublas as m; \
print(m.__path__[0])')/lib:$(python3 -c 'import nvidia.curand as m; print(m.__path__[0])')/lib:\
$LD_LIBRARY_PATH\"`; the CUDA library loader does not search Python's site-packages on Linux";

/// Why a CUDA library was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CudaLibraryRefusalReason {
    /// No candidate file of the built CUDA major could be loaded.
    Missing,
    /// The loaded file reports a major other than the built one.
    WrongMajor,
    /// The loaded file lacks a symbol FathomDB's CUDA path calls.
    MissingSymbol,
}

impl CudaLibraryRefusalReason {
    /// Stable lower-snake-case reason for bindings and diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::WrongMajor => "wrong_major",
            Self::MissingSymbol => "missing_symbol",
        }
    }
}

/// A CUDA library the device needs was refused before any GPU work; the
/// error kind is `cuda_library_unavailable`.
///
/// Non-exhaustive: build one with [`CudaLibraryRefusal::new`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaLibraryRefusal {
    /// The library: `cublas` or `curand`.
    pub library: &'static str,
    /// Why it was refused.
    pub reason: CudaLibraryRefusalReason,
    /// The loaded file, when one was found.
    pub path: Option<Box<str>>,
    /// The version the loaded file reports, when one was found.
    pub version: Option<Box<str>>,
    /// The library's own major that the build needs: 12 for cuBLAS and 10
    /// for cuRAND (`libcurand.so.10`) in a CUDA 12 build.
    pub required_major: u32,
    /// Each candidate name tried, with why it was not used.
    pub tried: Vec<(String, String)>,
    /// The required symbols the loaded file lacks (empty unless
    /// [`CudaLibraryRefusalReason::MissingSymbol`]), in the required-set order.
    pub missing_symbols: Vec<String>,
}

impl CudaLibraryRefusal {
    /// A refusal record.
    #[must_use]
    pub fn new(
        library: &'static str,
        reason: CudaLibraryRefusalReason,
        path: Option<String>,
        version: Option<String>,
        required_major: u32,
        tried: Vec<(String, String)>,
        missing_symbols: Vec<String>,
    ) -> Self {
        Self {
            library,
            reason,
            path: path.map(String::into_boxed_str),
            version: version.map(String::into_boxed_str),
            required_major,
            tried,
            missing_symbols,
        }
    }

    /// The stable error kind of every refusal.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        "cuda_library_unavailable"
    }
}

impl fmt::Display for CudaLibraryRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CUDA library {} is unusable ({})", self.library, self.reason.as_str())?;
        if let Some(path) = &self.path {
            write!(formatter, ": {path}")?;
        }
        if let Some(version) = &self.version {
            write!(formatter, " version {version}")?;
        }
        write!(formatter, " (required major {})", self.required_major)?;
        if !self.tried.is_empty() {
            let tried = self
                .tried
                .iter()
                .map(|(name, why)| format!("{name} ({why})"))
                .collect::<Vec<_>>()
                .join(", ");
            write!(formatter, "; tried {tried}")?;
        }
        if !self.missing_symbols.is_empty() {
            write!(formatter, "; missing symbols: {}", self.missing_symbols.join(", "))?;
        }
        match self.reason {
            CudaLibraryRefusalReason::Missing => {
                write!(formatter, ". Remedy: {CUDA_12_WHEEL_REMEDY}")
            }
            CudaLibraryRefusalReason::MissingSymbol => write!(
                formatter,
                ". Remedy: install a newer CUDA 12 copy of {}: {CUDA_12_WHEEL_REMEDY}",
                self.library
            ),
            CudaLibraryRefusalReason::WrongMajor => write!(
                formatter,
                ". Remedy: remove {} from the library search path (`LD_LIBRARY_PATH`) so the \
                 loader finds a copy with major {}",
                self.path.as_deref().unwrap_or("the file opened"),
                self.required_major
            ),
        }
    }
}

impl std::error::Error for CudaLibraryRefusal {}

/// One CUDA library the process loaded, as `CudaDeviceInfo::cuda_libraries`
/// and `doctor cuda-libraries` report it.
///
/// Non-exhaustive: build one with [`CudaLibraryReport::new`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaLibraryReport {
    /// The library: `driver`, `cublas`, `curand`, `cublasLt` or `nvrtc`.
    pub library: String,
    /// The candidate name that was opened.
    pub opened_as: String,
    /// The canonical path of the loaded file, comparable with `/proc/self/maps`.
    pub path: Option<String>,
    /// The version the library reports.
    pub version: Option<String>,
    /// The CUDA major the build checked it against.
    pub built_major: u32,
}

impl CudaLibraryReport {
    /// A loaded-library record.
    #[must_use]
    pub fn new(
        library: impl Into<String>,
        opened_as: impl Into<String>,
        path: Option<String>,
        version: Option<String>,
        built_major: u32,
    ) -> Self {
        Self { library: library.into(), opened_as: opened_as.into(), path, version, built_major }
    }
}

/// The names of `set` for which `is_present` is false, in set order.
#[cfg_attr(not(any(feature = "embed-cuda", feature = "rerank-cuda", test)), allow(dead_code))]
pub(crate) fn missing_required_symbols(
    set: &[&str],
    mut is_present: impl FnMut(&str) -> bool,
) -> Vec<String> {
    set.iter().filter(|name| !is_present(name)).map(|name| (*name).to_owned()).collect()
}

/// The outcome of `doctor cuda-libraries`: the build's CUDA major, every
/// library the process loaded, and the refusal if one was found.
///
/// Non-exhaustive: build one with [`CudaLibrariesDiagnostic::new`].
#[doc(hidden)]
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaLibrariesDiagnostic {
    /// The CUDA major the libraries were checked against.
    pub built_major: u32,
    /// Each library loaded, as the device report lists them.
    pub libraries: Vec<CudaLibraryReport>,
    /// The refusal, when a needed library is unusable.
    pub refusal: Option<CudaLibraryRefusal>,
    /// The CUDA libraries mapped into this process (`/proc/self/maps`).
    pub maps: Vec<String>,
}

impl CudaLibrariesDiagnostic {
    /// A diagnostic record.
    #[must_use]
    pub fn new(
        built_major: u32,
        libraries: Vec<CudaLibraryReport>,
        refusal: Option<CudaLibraryRefusal>,
        maps: Vec<String>,
    ) -> Self {
        Self { built_major, libraries, refusal, maps }
    }
}

#[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
mod driver {
    use std::path::Path;
    use std::sync::Once;

    use candle_core::cuda::cudarc::{self, LibLoadError, LoadedLibrary, RejectReason};

    use super::{
        missing_required_symbols, CudaLibrariesDiagnostic, CudaLibraryRefusal,
        CudaLibraryRefusalReason, CudaLibraryReport, REQUIRED_CUBLAS_SYMBOLS,
        REQUIRED_CURAND_SYMBOLS,
    };

    /// A library's own major in a build for CUDA `cuda_major`: cuRAND keeps
    /// soname major 10 (`libcurand.so.10`) across CUDA 12 and 13.
    fn library_major(library: &str, cuda_major: u32) -> u32 {
        if library == "curand" {
            10
        } else {
            cuda_major
        }
    }

    /// Where the dynamic loader finds `name` first: the first
    /// `LD_LIBRARY_PATH` directory holding it (canonical path), if any.
    fn on_library_path(name: &str) -> Option<String> {
        std::env::var("LD_LIBRARY_PATH")
            .ok()?
            .split(':')
            .filter(|dir| !dir.is_empty())
            .map(|dir| Path::new(dir).join(name))
            .find(|candidate| candidate.is_file())
            .and_then(|candidate| candidate.canonicalize().ok())
            .map(|path| path.display().to_string())
    }

    /// Map the fork's load error to a refusal: `WrongMajor` when a candidate
    /// was found and reported another major, `Missing` otherwise.
    pub(crate) fn refusal_from_lib_load_error(error: &LibLoadError) -> CudaLibraryRefusal {
        let wrong = error.tried.iter().find_map(|(name, reason)| match reason {
            RejectReason::WrongVersion(version)
            | RejectReason::AlreadyLoadedWrongVersion(version)
            | RejectReason::WrongMajorDependency { version, .. } => {
                Some((name, version.to_string()))
            }
            _ => None,
        });
        let (reason, path, version) = match wrong {
            Some((name, version)) => {
                (CudaLibraryRefusalReason::WrongMajor, on_library_path(name), Some(version))
            }
            None => (CudaLibraryRefusalReason::Missing, None, None),
        };
        CudaLibraryRefusal::new(
            error.library,
            reason,
            path,
            version,
            library_major(error.library, error.cuda_major),
            error.tried.iter().map(|(name, why)| (name.clone(), why.to_string())).collect(),
            Vec::new(),
        )
    }

    fn report(library: &str, loaded: &LoadedLibrary) -> CudaLibraryReport {
        CudaLibraryReport::new(
            library,
            loaded.opened_as.clone(),
            loaded.path.clone(),
            loaded.version.map(|version| version.to_string()),
            loaded.cuda_major,
        )
    }

    fn loaded_refusal(
        loaded: &LoadedLibrary,
        reason: CudaLibraryRefusalReason,
        missing_symbols: Vec<String>,
    ) -> CudaLibraryRefusal {
        CudaLibraryRefusal::new(
            loaded.library,
            reason,
            loaded.path.clone(),
            loaded.version.map(|version| version.to_string()),
            library_major(loaded.library, loaded.cuda_major),
            vec![(loaded.opened_as.clone(), "opened".to_owned())],
            missing_symbols,
        )
    }

    fn check_cublas() -> Result<LoadedLibrary, CudaLibraryRefusal> {
        // SAFETY: loads and version-checks the cuBLAS shared library only;
        // no cuBLAS function is called.
        unsafe { cudarc::cublas::sys::try_culib() }
            .map_err(|error| refusal_from_lib_load_error(&error))?;
        let loaded =
            cudarc::cublas::sys::loaded_library().expect("cuBLAS is loaded after try_culib");
        // The fork version-checks every candidate, but a file that reports
        // another major must never reach a device even if it got through.
        if loaded.version.is_some_and(|version| version.major != loaded.cuda_major) {
            return Err(loaded_refusal(&loaded, CudaLibraryRefusalReason::WrongMajor, Vec::new()));
        }
        let missing = missing_required_symbols(&REQUIRED_CUBLAS_SYMBOLS, |name| {
            // SAFETY: looks the symbol up in the already-loaded library; it
            // is not called.
            unsafe { cudarc::cublas::sys::is_symbol_present(name) }.unwrap_or(false)
        });
        if !missing.is_empty() {
            return Err(loaded_refusal(&loaded, CudaLibraryRefusalReason::MissingSymbol, missing));
        }
        Ok(loaded)
    }

    fn check_curand() -> Result<LoadedLibrary, CudaLibraryRefusal> {
        // SAFETY: loads and version-checks the cuRAND shared library only;
        // no cuRAND function is called.
        unsafe { cudarc::curand::sys::try_culib() }
            .map_err(|error| refusal_from_lib_load_error(&error))?;
        let loaded =
            cudarc::curand::sys::loaded_library().expect("cuRAND is loaded after try_culib");
        let missing = missing_required_symbols(&REQUIRED_CURAND_SYMBOLS, |name| {
            // SAFETY: looks the symbol up in the already-loaded library; it
            // is not called.
            unsafe { cudarc::curand::sys::is_symbol_present(name) }.unwrap_or(false)
        });
        if !missing.is_empty() {
            return Err(loaded_refusal(&loaded, CudaLibraryRefusalReason::MissingSymbol, missing));
        }
        Ok(loaded)
    }

    /// The one stderr line a process writes for a refusal (owner ruling Q24).
    fn refusal_line(refusal: &CudaLibraryRefusal) -> String {
        let tried: Vec<serde_json::Value> = refusal
            .tried
            .iter()
            .map(|(name, reason)| serde_json::json!({"name": name, "reason": reason}))
            .collect();
        let record = serde_json::json!({
            "kind": refusal.kind(),
            "library": refusal.library,
            "reason": refusal.reason.as_str(),
            "path": refusal.path,
            "version": refusal.version,
            "required_major": refusal.required_major,
            "tried": tried,
            "missing_symbols": refusal.missing_symbols,
            "message": refusal.to_string(),
        });
        format!("fathomdb-cuda-libraries {record}")
    }

    /// Check, before any CUDA device is built, that cuBLAS and cuRAND (the
    /// libraries Candle's device construction loads) are usable for the
    /// built CUDA major and export every required symbol. A refusal writes
    /// one stderr line per process.
    pub(crate) fn require_usable_cuda_libraries() -> Result<(), CudaLibraryRefusal> {
        static REFUSAL_LINE: Once = Once::new();
        let outcome = check_cublas().and_then(|_| check_curand()).map(|_| ());
        if let Err(refusal) = &outcome {
            REFUSAL_LINE.call_once(|| eprintln!("{}", refusal_line(refusal)));
        }
        outcome
    }

    /// Every CUDA library this process has loaded: the driver, cuBLAS and
    /// cuRAND, and cuBLASLt or NVRTC only when cudarc loaded them.
    pub(crate) fn cuda_library_reports() -> Vec<CudaLibraryReport> {
        [
            ("driver", cudarc::driver::sys::loaded_library()),
            ("cublas", cudarc::cublas::sys::loaded_library()),
            ("cublasLt", cudarc::cublaslt::sys::loaded_library()),
            ("curand", cudarc::curand::sys::loaded_library()),
            ("nvrtc", cudarc::nvrtc::sys::loaded_library()),
        ]
        .into_iter()
        .filter_map(|(library, loaded)| loaded.map(|loaded| report(library, &loaded)))
        .collect()
    }

    /// `doctor cuda-libraries`: load the driver, run the device check and
    /// report what loaded. It always reports; a refusal is in the result.
    pub fn cuda_libraries_diagnostic() -> CudaLibrariesDiagnostic {
        // `is_culib_present` only tries to open the driver; `culib` keeps it
        // loaded, which is what `loaded_library` reports.
        // SAFETY: locates and loads the CUDA driver library only; no driver
        // function is called, and `culib` runs only when it can succeed.
        if unsafe { cudarc::driver::sys::is_culib_present() } {
            let _ = unsafe { cudarc::driver::sys::culib() };
        }
        let refusal = require_usable_cuda_libraries().err();
        let libraries = cuda_library_reports();
        let built_major = cudarc::cublas::sys::loaded_library()
            .or_else(cudarc::driver::sys::loaded_library)
            .map_or(12, |loaded| loaded.cuda_major);
        let built_major = refusal
            .as_ref()
            .filter(|refusal| refusal.library == "cublas")
            .map_or(built_major, |refusal| refusal.required_major);
        let maps = std::fs::read_to_string("/proc/self/maps")
            .map(|maps| crate::cuda_pool_policy::cuda_libraries_in_maps(&maps))
            .unwrap_or_default();
        CudaLibrariesDiagnostic::new(built_major, libraries, refusal, maps)
    }
}

#[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
pub use driver::cuda_libraries_diagnostic;
#[cfg(all(test, any(feature = "embed-cuda", feature = "rerank-cuda")))]
use driver::refusal_from_lib_load_error;
#[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
pub(crate) use driver::{cuda_library_reports, require_usable_cuda_libraries};

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    struct RefusingProvider(CudaLibraryRefusal);

    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    impl crate::CudaProvider for RefusingProvider {
        fn enumerate_visible_cuda_devices(
            &mut self,
        ) -> Result<Vec<crate::CudaVisibleDevice>, crate::CudaProbeError> {
            Ok(vec![crate::CudaVisibleDevice {
                visible_ordinal: 0,
                uuid: "GPU-d18".to_owned(),
                name: "d18".to_owned(),
                compute_capability: None,
            }])
        }

        fn probe_cuda(
            &mut self,
            _ordinal: usize,
        ) -> Result<crate::CudaDeviceInfo, crate::CudaProbeError> {
            Err(crate::CudaProbeError::LibraryUnavailable(self.0.clone()))
        }
    }

    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    #[test]
    fn a_lib_load_error_for_a_missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy() {
        use candle_core::cuda::cudarc::{LibLoadError, RejectReason};
        let error = LibLoadError {
            library: "cublas",
            cuda_major: 12,
            tried: vec![
                ("libcublas.so".to_owned(), RejectReason::NotFound),
                ("libcublas.so.12".to_owned(), RejectReason::NotFound),
            ],
        };
        let refusal = refusal_from_lib_load_error(&error);
        assert_eq!(refusal.reason, CudaLibraryRefusalReason::Missing);
        assert_eq!(refusal.required_major, 12);
        assert_eq!(refusal.path, None);
        let resolution = crate::resolve_embed_device_policy(
            crate::EmbedDevicePolicy::Cuda(0),
            true,
            &mut RefusingProvider(refusal.clone()),
        )
        .expect_err("a forced refusal is an error");
        assert_eq!(
            resolution,
            crate::DeviceResolutionError::CudaLibraryUnavailable { ordinal: 0, refusal }
        );
        let error = crate::EmbedDevicePolicyError::Resolution(resolution);
        assert_eq!(error.kind(), "cuda_library_unavailable");
        let message = error.to_string();
        for needle in [
            "pip install nvidia-cublas-cu12 nvidia-curand-cu12",
            "nvidia.cublas",
            "nvidia.curand",
            "LD_LIBRARY_PATH",
        ] {
            assert!(message.contains(needle), "{needle:?} missing from {message:?}");
        }
    }

    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    #[test]
    fn a_lib_load_error_for_a_wrong_major_cublas_is_wrong_major() {
        use candle_core::cuda::cudarc::{LibLoadError, LibVersion, RejectReason};
        let thirteen = LibVersion { major: 13, minor: 1, patch: Some(1) };
        let error = LibLoadError {
            library: "cublas",
            cuda_major: 12,
            tried: vec![
                ("libcublas.so.12".to_owned(), RejectReason::WrongVersion(thirteen)),
                ("libcublas.so".to_owned(), RejectReason::NotFound),
            ],
        };
        let refusal = refusal_from_lib_load_error(&error);
        assert_eq!(refusal.reason, CudaLibraryRefusalReason::WrongMajor);
        assert_eq!(refusal.version.as_deref(), Some("13.1.1"));
        assert_eq!(refusal.required_major, 12);
        assert_eq!(
            refusal.tried[0],
            ("libcublas.so.12".to_owned(), "wrong version 13.1.1".to_owned())
        );
        let curand = refusal_from_lib_load_error(&LibLoadError {
            library: "curand",
            cuda_major: 12,
            tried: vec![("libcurand.so.10".to_owned(), RejectReason::NotFound)],
        });
        assert_eq!(curand.required_major, 10, "cuRAND's own major in a CUDA 12 build");
    }

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
