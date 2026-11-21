"""Check pre-reset identity guards against an isolated synthetic driver ABI."""
import os
import pathlib
import subprocess
import tempfile
import unittest

SOURCE = pathlib.Path(__file__).resolve().parents[1] / "slice60_cuda_reset_actor.c"
DRIVER = r'''
#include <cuda.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static void mark(const char *value) {
    FILE *f = fopen(getenv("ACTOR_TRACE"), "a");
    if (!f) abort();
    fprintf(f, "%s\n", value); fclose(f);
}
CUresult cuDeviceGet(CUdevice *d, int ordinal) { *d=ordinal; return CUDA_SUCCESS; }
CUresult cuDeviceGetUuid_v2(CUuuid *u, CUdevice d) { (void)d; memset(u,0,sizeof(*u)); return CUDA_SUCCESS; }
CUresult cuDevicePrimaryCtxGetState(CUdevice d, unsigned *flags, int *active) { (void)d; *flags=0; *active=1; return CUDA_SUCCESS; }
CUresult cuDevicePrimaryCtxRetain(CUcontext *c, CUdevice d) { (void)d; mark("retain"); *c=(CUcontext)1; return CUDA_SUCCESS; }
CUresult cuDevicePrimaryCtxRelease_v2(CUdevice d) { (void)d; mark("release"); return CUDA_SUCCESS; }
CUresult cuDevicePrimaryCtxReset_v2(CUdevice d) { (void)d; mark("RESET"); return CUDA_SUCCESS; }
CUresult cuCtxGetCurrent(CUcontext *c) { *c=NULL; return CUDA_SUCCESS; }
CUresult cuCtxSetCurrent(CUcontext c) { (void)c; mark("bind"); return CUDA_SUCCESS; }
CUresult cuCtxGetId(CUcontext c, unsigned long long *id) { (void)c; *id=42; return CUDA_SUCCESS; }
CUresult cuCtxSynchronize(void) { mark("sync"); return CUDA_SUCCESS; }
'''


class ActorIdentity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory(prefix="slice60-actor-abi-")
        cls.root = pathlib.Path(cls.scratch.name)
        cuda = os.environ.get("SLICE60_CUDA_INCLUDE", "/usr/local/cuda-12.6/include")
        headers = os.environ["SLICE60_NODE_API_INCLUDE"]
        driver = cls.root / "driver.c"
        driver.write_text(DRIVER)
        subprocess.run(["gcc", "-Wall", "-Wextra", "-Werror", "-shared", "-fPIC", "-I"+cuda, str(driver), "-o", str(cls.root/"libcuda.so.1")], check=True)
        cls.actor = cls.root / "actor.node"
        subprocess.run(["gcc", "-Wall", "-Wextra", "-Werror", "-shared", "-fPIC", "-DNAPI_VERSION=8", "-I"+cuda, "-I"+headers, str(SOURCE), "-ldl", "-o", str(cls.actor)], check=True)

    @classmethod
    def tearDownClass(cls):
        cls.scratch.cleanup()

    def invoke(self, uuid, context):
        trace = self.root / (self.id().split(".")[-1]+".trace")
        env = dict(os.environ, LD_LIBRARY_PATH=str(self.root), ACTOR_TRACE=str(trace),
                   FATHOMDB_COORDINATED_RESET="reset-drop", FATHOMDB_REQUIRE_LIVE="1",
                   FATHOMDB_SELECTED_UUID=uuid, FATHOMDB_EXPECTED_CONTEXT_ID=context)
        script = "try { require(process.argv[1]).coordinate(true); } catch(e) { console.log(e.message); process.exitCode=23; }"
        result = subprocess.run([os.environ["SLICE60_NODE"], "-e", script, str(self.actor)], env=env, text=True, capture_output=True, check=False)
        return result, trace.read_text().splitlines() if trace.exists() else []

    def test_wrong_device_refuses_before_retain_or_reset(self):
        result, trace = self.invoke("GPU-11111111-0000-0000-0000-000000000000", "42")
        self.assertEqual(result.returncode, 23, result.stdout+result.stderr)
        self.assertIn("selected-uuid-mismatch", result.stdout)
        self.assertEqual(trace, [])

    def test_wrong_context_refuses_and_balances_before_bind_or_reset(self):
        result, trace = self.invoke("GPU-00000000-0000-0000-0000-000000000000", "43")
        self.assertEqual(result.returncode, 23, result.stdout+result.stderr)
        self.assertIn("expected-context-id-mismatch", result.stdout)
        self.assertEqual(trace, ["retain", "release"])

    def test_matching_identity_resets_once_and_balances_once(self):
        result, trace = self.invoke("GPU-00000000-0000-0000-0000-000000000000", "42")
        self.assertEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assertEqual(trace, ["retain", "bind", "sync", "RESET", "release"])


if __name__ == "__main__":
    unittest.main()
