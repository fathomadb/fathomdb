#define _GNU_SOURCE
#include "dwfl_bias_abi.h"

#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

typedef Dwfl_Module *(*report_elf_fn)(Dwfl *, const char *, const char *, int,
                                    GElf_Addr, bool);

Dwfl_Module *dwfl_report_elf(Dwfl *dwfl, const char *name,
                           const char *file_name, int fd,
                           GElf_Addr base, bool add_p_vaddr) {
    report_elf_fn next = (report_elf_fn)dlsym(RTLD_NEXT, "dwfl_report_elf");
    if (next == NULL) {
        fputs("dwfl bias: RTLD_NEXT resolution failed\n", stderr);
        exit(127);
    }
    return next(dwfl, name, file_name, fd, base, add_p_vaddr);
}
