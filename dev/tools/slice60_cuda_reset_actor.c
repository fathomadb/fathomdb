/* Qualification-only actor. Never part of a FathomDB distributed artifact. */
#include <cuda.h>
#include <dlfcn.h>
#include <inttypes.h>
#include <node_api.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void *driver_handle;

static napi_value refuse(napi_env env, const char *precondition) {
    char message[192];
    snprintf(message, sizeof(message), "CUDA coordination actor-precondition=%s", precondition);
    napi_throw_error(env, "SLICE60_ACTOR_REFUSED", message);
    return NULL;
}

static napi_value native_failure(napi_env env, const char *phase, CUresult status,
                                 bool release_attempted, CUresult release_status) {
    char message[256];
    if (release_attempted) {
        snprintf(message, sizeof(message), "CUDA coordination phase=%s actual-status=%u balanced-release-actual-status=%u",
                 phase, (unsigned)status, (unsigned)release_status);
    } else {
        snprintf(message, sizeof(message), "CUDA coordination phase=%s actual-status=%u balanced-release=not-acquired", phase, (unsigned)status);
    }
    napi_throw_error(env, "SLICE60_ACTOR_NATIVE_FAILURE", message);
    return NULL;
}

static napi_value coordinate(napi_env env, napi_callback_info info) {
    napi_value argument, output;
    size_t argc = 1;
    bool reset = false;
    const char *mode = getenv("FATHOMDB_COORDINATED_RESET");
    const char *live = getenv("FATHOMDB_REQUIRE_LIVE");
    if (napi_get_cb_info(env, info, &argc, &argument, NULL, NULL) != napi_ok || argc != 1 ||
        napi_get_value_bool(env, argument, &reset) != napi_ok || !mode || !live || strcmp(live, "1") ||
        (reset ? (strcmp(mode, "reset-refuse") && strcmp(mode, "reset-drop")) : strcmp(mode, "healthy"))) {
        napi_throw_error(env, "SLICE60_ACTOR_MODE", "explicit leased healthy or separately authorized reset mode required");
        return NULL;
    }
    if (!driver_handle) driver_handle = dlopen("libcuda.so.1", RTLD_NOW | RTLD_LOCAL);
    if (!driver_handle) return refuse(env, "driver-library-unavailable");
#define LOAD(name, symbol) \
    __typeof__(&name) name##_fn = (__typeof__(&name))dlsym(driver_handle, symbol); \
    if (!name##_fn) return refuse(env, "missing-symbol-" symbol)
    LOAD(cuDeviceGet, "cuDeviceGet");
    LOAD(cuDeviceGetUuid, "cuDeviceGetUuid_v2");
    LOAD(cuDevicePrimaryCtxGetState, "cuDevicePrimaryCtxGetState");
    LOAD(cuDevicePrimaryCtxRetain, "cuDevicePrimaryCtxRetain");
    LOAD(cuDevicePrimaryCtxRelease, "cuDevicePrimaryCtxRelease_v2");
    LOAD(cuDevicePrimaryCtxReset, "cuDevicePrimaryCtxReset_v2");
    LOAD(cuCtxGetCurrent, "cuCtxGetCurrent");
    LOAD(cuCtxSetCurrent, "cuCtxSetCurrent");
    LOAD(cuCtxGetId, "cuCtxGetId");
    LOAD(cuCtxSynchronize, "cuCtxSynchronize");
#undef LOAD
    CUdevice device;
    CUuuid uuid;
    CUcontext context = NULL, previous = NULL;
    unsigned int flags = 0;
    int active = 0;
    unsigned long long id = 0;
    CUresult status;
    const char *phase = NULL, *precondition = NULL;
    CUresult released = CUDA_SUCCESS;
    bool retained = false;
#define CHECK(call, label) do { status = (call); if (status != CUDA_SUCCESS) { phase = (label); goto finish; } } while (0)
    CHECK(cuDeviceGet_fn(&device, 0), "device");
    CHECK(cuDeviceGetUuid_fn(&uuid, device), "uuid");
    CHECK(cuDevicePrimaryCtxGetState_fn(device, &flags, &active), "primary-state");
    if (!active) return refuse(env, "primary-inactive-before-retain");
    CHECK(cuCtxGetCurrent_fn(&previous), "previous-context");
    CHECK(cuDevicePrimaryCtxRetain_fn(&context, device), "retain-existing-primary");
    retained = true;
    CHECK(cuCtxGetId_fn(context, &id), "retained-context-id");
    if (previous && previous != context) {
        precondition = "prior-context-mismatch";
        goto finish;
    }
    CHECK(cuCtxSetCurrent_fn(context), "bind-existing-primary");
    CHECK(cuCtxSynchronize_fn(), "all-streams-synchronize");
    if (reset) {
        /* All application owners are retained, drained and quiescent. After
           this attempt never restore a potentially stale context. */
        CHECK(cuDevicePrimaryCtxReset_fn(device), "process-primary-reset");
    } else {
        CHECK(cuCtxSetCurrent_fn(previous), "healthy-restore-prior");
    }
finish:
    if (retained) {
        released = cuDevicePrimaryCtxRelease_fn(device);
        if (!phase && released != CUDA_SUCCESS) {
            status = released;
            phase = "balanced-primary-release";
        }
    }
#undef CHECK
    if (phase) return native_failure(env, phase, status, retained, released);
    if (precondition) return refuse(env, precondition);
    char uuid_text[41] = "GPU-";
    size_t offset = 4;
    for (size_t i = 0; i < 16; ++i) {
        if (i == 4 || i == 6 || i == 8 || i == 10) uuid_text[offset++] = '-';
        snprintf(uuid_text + offset, sizeof(uuid_text) - offset, "%02x", (unsigned int)(unsigned char)uuid.bytes[i]);
        offset += 2;
    }
    char report[256];
    snprintf(report, sizeof(report), "{\"context_id\":\"%" PRIu64 "\",\"uuid\":\"%s\",\"mode\":\"%s\",\"balanced_release\":true}", (uint64_t)id, uuid_text, mode);
    if (napi_create_string_utf8(env, report, NAPI_AUTO_LENGTH, &output) != napi_ok) return NULL;
    return output;
}

static napi_value initialize(napi_env env, napi_value exports) {
    napi_value function;
    if (napi_create_function(env, "coordinate", NAPI_AUTO_LENGTH, coordinate, NULL, &function) != napi_ok ||
        napi_set_named_property(env, exports, "coordinate", function) != napi_ok) return NULL;
    return exports;
}
NAPI_MODULE(NODE_GYP_MODULE_NAME, initialize)
