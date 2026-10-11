/* Does a zero-byte allocation succeed on each path? (cudarc CudaStream::null
 * and Candle zero-element tensors reach malloc_sync(0) / malloc_async(0).) */
#include <cuda.h>
#include <stdio.h>

static const char *name(CUresult r) {
  const char *s = NULL;
  cuGetErrorName(r, &s);
  return s ? s : "?";
}

int main(void) {
  CUdevice dev;
  CUcontext ctx;
  CUdeviceptr p = 0;
  CUmemoryPool pool;
  cuInit(0);
  cuDeviceGet(&dev, 0);
  cuDevicePrimaryCtxRetain(&ctx, dev);
  cuCtxSetCurrent(ctx);
  CUresult rp = cuDeviceGetDefaultMemPool(&pool, dev);
  printf("default_pool=%s\n", name(rp));
  CUresult r = cuMemAlloc(&p, 0);
  printf("cuMemAlloc(0)=%s ptr=%#llx\n", name(r), (unsigned long long)p);
  if (r == CUDA_SUCCESS) printf("cuMemFree(ptr)=%s\n", name(cuMemFree(p)));
  printf("cuMemFree(0)=%s\n", name(cuMemFree(0)));
  p = 0;
  r = cuMemAllocAsync(&p, 0, 0);
  printf("cuMemAllocAsync(0)=%s ptr=%#llx\n", name(r), (unsigned long long)p);
  if (r == CUDA_SUCCESS) printf("cuMemFreeAsync(ptr)=%s\n", name(cuMemFreeAsync(p, 0)));
  printf("sync=%s\n", name(cuStreamSynchronize(0)));
  cuDevicePrimaryCtxRelease(dev);
  return 0;
}
