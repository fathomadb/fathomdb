# Introducing CUDA 13.2 and Deprecating CUDA 12.8 (Release 2.12)

**URL:** <https://dev-discuss.pytorch.org/t/introducing-cuda-13-2-and-deprecating-cuda-12-8-release-2-12/3337>\
**Category:** release/packaging\
**Created:** [April 1, 2026, 2:58pm UTC](https://dev-discuss.pytorch.org/t/introducing-cuda-13-2-and-deprecating-cuda-12-8-release-2-12/3337 "2026-04-01T14:58:55Z")\
**Posts on this page:** 1\
**Page:** 1

<div class="post-metadata">

**Author:** ![atalman](https://yyz2.discourse-cdn.com/flex036/user_avatar/dev-discuss.pytorch.org/atalman/32/1857_2.png) [@atalman](https://dev-discuss.pytorch.org/u/atalman)\
**Post date:** [April 1, 2026, 2:58pm UTC](https://dev-discuss.pytorch.org/t/introducing-cuda-13-2-and-deprecating-cuda-12-8-release-2-12/3337/1 "2026-04-01T14:58:55Z")

</div>

What’s changing

Starting with the PyTorch 2.12 release, we are updating the CUDA support matrix:

- CUDA 13.2 is being introduced as an experimental build. Nightly builds are already available  
on [download.pytorch.org](http://download.pytorch.org) and can be installed via:  
`pip install torch --index-url https://download.pytorch.org/whl/nightly/cu132`  
This enables early adopters to test against the latest CUDA toolkit and take advantage of new features, including expanded Blackwell architecture support.
- CUDA 12.8 is being deprecated and removed from CI/CD pipelines and binary build matrices. It will no longer be available as a prebuilt binary. Removal from CI/CD pipelines will be completed the week of April 6–10, 2026.
- CUDA 13.0 remains the stable variant published to PyPI (both x86\_64 and aarch64).
- CUDA 12.6 remains available as the legacy build via [https://download.pytorch.org](https://download.pytorch.org), preserving support for older GPU architectures (Maxwell, Pascal, Volta) that are not supported by CUDA 13.x.

The full support matrix for Release 2.12:

```auto
│ CUDA │ Role │ PyPI │ download.pytorch.org │ GPU Architectures (Linux x86_64) │
│ Version │ │ │ │ │   
├──────────┼──────────────┼──────┼──────────────────────┼───────────────────────────────────┤
│ │ │ │ │ Maxwell (5.0), Pascal (6.0), │
│ 12.6.3 │ Legacy │ No │ Yes │ Volta (7.0), Turing (7.5), Ampere │   
│ │ │ │ │ (8.0, 8.6), Hopper (9.0) │
├──────────┼──────────────┼──────┼──────────────────────┼───────────────────────────────────┤   
│ │ │ │ │ Turing (7.5), Ampere (8.0, 8.6), │
│ 13.0.x │ Stable │ Yes │ Yes │ Hopper (9.0), Blackwell (10.0, │   
│ │ │ │ │ 12.0) │
├──────────┼──────────────┼──────┼──────────────────────┼───────────────────────────────────┤   
│ │ │ │ │ Turing (7.5), Ampere (8.0, 8.6), │   
│ 13.2.x │ Experimental │ No │ Yes │ Hopper (9.0), Blackwell (10.0, │
│ │ │ │ │ 12.0) │   
└──────────┴──────────────┴──────┴──────────────────────┴───────────────────────────────────┘

```

For Linux aarch64, CUDA 13.0.x and 13.2.x add Blackwell support (SM 10.0, 11.0, 12.0) alongside Ampere (8.0) and Hopper (9.0).

Why we’re doing this

- CUDA 12.8 no longer serves a distinct role. With CUDA 13.0 established as stable and CUDA 12.6 retained for legacy GPU support, the intermediate 12.8 version adds maintenance burden without clear user benefit. Removing it simplifies the build matrix and CI infrastructure.
- CUDA 13.2 brings forward compatibility. Adding an experimental tier lets users and library authors test against the newest toolkit early, catch compatibility issues before they reach stable, and take advantage of the latest Blackwell optimizations.

Tracking / details

The full RFC with architecture tables, cuDNN versions, and implementation tasks is tracked in: [[RFC] CUDA support matrix for Release 2.12 - introduce CUDA 13.2 as experimental. · Issue #178665 · pytorch/pytorch · GitHub](https://github.com/pytorch/pytorch/issues/178665).

If you have concerns about the CUDA 12.8 removal or want to discuss the 13.2 experimental builds, please comment on the RFC issue.

— Team PyTorch
