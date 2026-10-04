#include <cuda.h>
#include <stdio.h>
int main(void) {
  CUdevice device; CUcontext context; size_t free_bytes = 0, total_bytes = 0;
  CUresult rc = cuInit(0); if (rc != CUDA_SUCCESS) { printf("cuInit=%d\n", rc); return 1; }
  rc = cuDeviceGet(&device, 0); if (rc != CUDA_SUCCESS) { printf("cuDeviceGet=%d\n", rc); return 1; }
  rc = cuDevicePrimaryCtxRetain(&context, device); if (rc != CUDA_SUCCESS) { printf("cuDevicePrimaryCtxRetain=%d\n", rc); return 1; }
  rc = cuCtxSetCurrent(context); if (rc != CUDA_SUCCESS) { printf("cuCtxSetCurrent=%d\n", rc); return 1; }
  rc = cuMemGetInfo(&free_bytes, &total_bytes);
  printf("cuMemGetInfo=%d free=%zu total=%zu\n", rc, free_bytes, total_bytes);
  cuDevicePrimaryCtxRelease(device);
  return rc == CUDA_SUCCESS ? 0 : 1;
}
