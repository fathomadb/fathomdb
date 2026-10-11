# Transitioning PyPI CUDA wheels to CUDA 13.0 as the stable (Release 2.11)

**URL:** <https://dev-discuss.pytorch.org/t/transitioning-pypi-cuda-wheels-to-cuda-13-0-as-the-stable-release-2-11/3325>\
**Category:** release/packaging\
**Created:** [March 11, 2026, 10:04pm UTC](https://dev-discuss.pytorch.org/t/transitioning-pypi-cuda-wheels-to-cuda-13-0-as-the-stable-release-2-11/3325 "2026-03-11T22:04:29Z")\
**Posts on this page:** 1\
**Page:** 1

<div class="post-metadata">

**Author:** ![atalman](https://yyz2.discourse-cdn.com/flex036/user_avatar/dev-discuss.pytorch.org/atalman/32/1857_2.png) [@atalman](https://dev-discuss.pytorch.org/u/atalman)\
**Post date:** [March 11, 2026, 10:04pm UTC](https://dev-discuss.pytorch.org/t/transitioning-pypi-cuda-wheels-to-cuda-13-0-as-the-stable-release-2-11/3325/1 "2026-03-11T22:04:29Z")

</div>

Hi everyone — we’re planning to transition CUDA 13.0 to be the stable CUDA runtime/toolkit for the PyTorch wheels published to PyPI for the 2.11 release series.

## What’s changing

- CUDA 13.0 will become the “stable” CUDA variant for PyTorch wheels published on PyPI (starting with the 2.11 release line).
- We will be publishing both x86\_64 and aarch64 CUDA 13.0 binaries to PyPI.
- CUDA 12.6 and 12.8 binaries will continue to be available to download via [download.pytorch.org](http://download.pytorch.org).

## Why we’re doing this

- Align the “stable” published wheels with the current CUDA/toolchain direction for the 2.11 timeframe.
- Reduce ambiguity for users choosing “the” default CUDA build on PyPI, and simplify ecosystem expectations around the recommended baseline.

## Tracking / details

We’re tracking the proposal and support matrix discussion here:

- [[RFC] Cuda support matrix for Release 2.11. Make CUDA 13.0 as stable](https://github.com/pytorch/pytorch/issues/172663)

Cheers,  
Team PyTorch
