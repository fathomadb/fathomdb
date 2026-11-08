#define _GNU_SOURCE
#include "dwfl_bias_abi.h"

#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef Dwfl_Module *(*report_elf_fn)(Dwfl *, const char *, const char *, int,
                                    GElf_Addr, bool);
typedef Dwfl_Module *(*addrmodule_fn)(Dwfl *, Dwarf_Addr);
typedef const char *(*module_info_fn)(Dwfl_Module *, void ***, Dwarf_Addr *,
                                     Dwarf_Addr *, Dwarf_Addr *, Dwarf_Addr *,
                                     const char **, const char **);
static _Atomic uint64_t correction_count;
static _Atomic uint64_t registered_count;
static _Atomic uint64_t reused_count;

static void fail(const char *message) {
    fprintf(stderr, "dwfl bias: %s\n", message);
    _Exit(125);
}

static GElf_Addr address(const char *variable) {
    const char *text = getenv(variable);
    char *end;
    if (text == NULL || text[0] == '\0' || text[0] == '-') {
        fail(variable);
    }
    errno = 0;
    unsigned long long parsed = strtoull(text, &end, 0);
    if (errno != 0 || end == text || *end != '\0') {
        fail(variable);
    }
    return (GElf_Addr)parsed;
}

static void audit(const char *filename, GElf_Addr before, GElf_Addr after, bool reused) {
    const char *path = getenv("FATHOMDB_DWFL_AUDIT");
    if (path == NULL || path[0] == '\0') {
        fail("FATHOMDB_DWFL_AUDIT is required for corrections");
    }
    int fd = open(path, O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC, 0600);
    if (fd < 0) {
        fail("cannot open correction audit");
    }
    uint64_t count = atomic_fetch_add(&correction_count, 1) + 1;
    uint64_t action_count = atomic_fetch_add(reused ? &reused_count : &registered_count, 1) + 1;
    if (dprintf(fd, "count=%" PRIu64 " action=%s action_count=%" PRIu64 " bad=0x%" PRIx64
                " corrected=0x%" PRIx64 " filename=%s\n",
                count, reused ? "reused" : "registered", action_count,
                (uint64_t)before, (uint64_t)after, filename) < 0) {
        fail("cannot write correction audit");
    }
    if (close(fd) != 0) {
        fail("cannot close correction audit");
    }
}


static Dwfl_Module *existing_module(Dwfl *dwfl, const char *target, GElf_Addr corrected) {
    addrmodule_fn find = (addrmodule_fn)dlsym(RTLD_NEXT, "dwfl_addrmodule");
    module_info_fn info = (module_info_fn)dlsym(RTLD_NEXT, "dwfl_module_info");
    if (find == NULL || info == NULL) {
        fail("RTLD_NEXT native module lookup resolution failed");
    }
    Dwfl_Module *module = find(dwfl, corrected);
    if (module == NULL) {
        return NULL;
    }
    Dwarf_Addr start = 0, end = 0;
    const char *mainfile = NULL;
    if (info(module, NULL, &start, &end, NULL, NULL, &mainfile, NULL) != NULL
        && start == corrected && end > start && mainfile != NULL
        && strcmp(mainfile, target) == 0) {
        return module;
    }
    return NULL;
}

Dwfl_Module *dwfl_report_elf(Dwfl *dwfl, const char *name,
                           const char *file_name, int fd,
                           GElf_Addr base, bool add_p_vaddr) {
    int incoming_errno = errno;
    report_elf_fn next = (report_elf_fn)dlsym(RTLD_NEXT, "dwfl_report_elf");
    if (next == NULL) {
        fail("RTLD_NEXT dwfl_report_elf resolution failed");
    }
    const char *target = getenv("FATHOMDB_DWFL_TARGET");
    if (!add_p_vaddr && file_name != NULL && target != NULL
        && strcmp(file_name, target) == 0
        && base == address("FATHOMDB_DWFL_BAD_BASE")) {
        GElf_Addr corrected = address("FATHOMDB_DWFL_CORRECT_BASE");
        if (fd == -1) {
            Dwfl_Module *module = existing_module(dwfl, target, corrected);
            if (module != NULL) {
                audit(file_name, base, corrected, true);
                errno = incoming_errno;
                return module;
            }
        }
        audit(file_name, base, corrected, false);
        base = corrected;
    }
    errno = incoming_errno;
    return next(dwfl, name, file_name, fd, base, add_p_vaddr);
}
