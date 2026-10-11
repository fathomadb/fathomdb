# Notice: CUDA 12.6 wheels will no longer be published from PyTorch 2.15 (drops Maxwell / Pascal / Volta)

**URL:** <https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432>\
**Category:** release/packaging\
**Created:** [September 4, 2026, 3:13pm UTC](https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432 "2026-09-04T15:13:08Z")\
**Posts on this page:** 1\
**Page:** 1

<div class="post-metadata">

**Author:** ![atalman](https://yyz2.discourse-cdn.com/flex036/user_avatar/dev-discuss.pytorch.org/atalman/32/1857_2.png) [@atalman](https://dev-discuss.pytorch.org/u/atalman)\
**Post date:** [September 4, 2026, 3:13pm UTC](https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432/1 "2026-09-04T15:13:08Z")

</div>

Heads-up that **PyTorch will stop publishing CUDA 12.6 binaries in 2.15**. Since CUDA 12.8 and 12.9 were already dropped, this means **PyTorch 2.14 is the last release to ship any CUDA 12.x wheels**. From 2.15, CUDA 12.6 is a **build-from-source-only** configuration.

CUDA 12.6 is removed from **CD** entirely — no 12.6 wheels or libtorch archives will be built or uploaded to PyPI or [download.pytorch.org](http://download.pytorch.org). We are keeping **one CUDA 12.6 build in CI** so that compilation against CUDA 12.x stays exercised and we notice if it breaks.

> **This starts next week.** CUDA 12.6 builds are being removed from **nightly** on **Monday 7 September 2026**.

Tracking RFC: [[RFC] CUDA support matrix for Release 2.15 — deprecate CUDA 12.6 (drops Maxwell / Pascal / Volta) · Issue #190385 · pytorch/pytorch · GitHub](https://github.com/pytorch/pytorch/issues/190385)

## What this means for GPU support

CUDA 13.x does not support compute capabilities below Turing (sm\_75), and CUDA 12.6 was the only remaining build covering them. Dropping the published binaries ends prebuilt support for:

| Architecture | Compute capability | Status after 2.15 |
| --- | --- | --- |
| **Maxwell** | sm\_50 | **Dropped** |
| **Pascal** | sm\_60 | **Dropped** |
| **Volta** | sm\_70 | **Dropped** |
| Turing | sm\_75 | Retained |
| Ampere | sm\_80, sm\_86 | Retained |
| Hopper | sm\_90 | Retained |
| Blackwell | sm\_100, sm\_120 | Retained |

If you are on a Maxwell, Pascal, or Volta GPU, **PyTorch 2.14 is the last release with a prebuilt wheel for you**. From 2.15 onward you will need to either stay on 2.14 (or earlier), or **compile PyTorch from source** against a CUDA 12.x toolkit.

**Linux aarch64 loses nothing.** The CUDA 12.6 aarch64 build covered Ampere and Hopper, both still covered by the CUDA 13.x builds, which additionally add Blackwell.

## What remains

CUDA 13.x builds continue and cover Turing (7.5) through Blackwell. The exact set of published CUDA versions changes from release to release, so check the getting-started page for the current list rather than pinning to what is written here: [https://pytorch.org/get-started/locally/](https://pytorch.org/get-started/locally/)

## Installing 2.14 today

If you need CUDA 12.6, pin to 2.14 while it is current:

```auto
pip3 install torch==2.14.0 torchvision==0.29.0 --index-url https://download.pytorch.org/whl/cu126

```

## Building from source for CUDA 12.6

This is the supported path for pre-Turing GPUs from 2.15 onward. Install a CUDA 12.6 toolkit, then build with the architectures you need:

```auto
git clone --recursive https://github.com/pytorch/pytorch
cd pytorch
export TORCH_CUDA_ARCH_LIST="5.0;6.0;7.0" # Maxwell / Pascal / Volta, adjust to your GPU
python -m pip install -r requirements.txt
python setup.py develop

```

Full instructions: [GitHub - pytorch/pytorch: Tensors and Dynamic neural networks in Python with strong GPU acceleration · GitHub](https://github.com/pytorch/pytorch#from-source)

Because one CUDA 12.6 build is retained in CI, breakages in the 12.x compile path should surface on our side rather than yours. If you do hit one, please file an issue — that build exists precisely so this configuration stays buildable.

## Timeline

- **2.14 (released)** — CUDA 12.6 shipped as a legacy published build; deprecation announced in the release notes. These binaries remain available.
- **Mon 7 September 2026** — CUDA 12.6 removed from the **nightly** CD builds. No further 12.6 nightlies published.
- **2.15** — CUDA 12.6 removed from CD entirely; no wheels or libtorch archives built or published. One CUDA 12.6 build stays in CI as a compile check.

## Feedback

If this breaks a workflow you depend on, please comment on the RFC — [[RFC] CUDA support matrix for Release 2.15 — deprecate CUDA 12.6 (drops Maxwell / Pascal / Volta) · Issue #190385 · pytorch/pytorch · GitHub](https://github.com/pytorch/pytorch/issues/190385) — before the 2.15 branch cut. We are particularly interested in hearing from users still on Volta (V100), which is the most widely deployed of the three affected architectures.

cc @albanD @atalman @ngimel@malfet @tinglvv @nWEIdia @ptrblck @eqy
