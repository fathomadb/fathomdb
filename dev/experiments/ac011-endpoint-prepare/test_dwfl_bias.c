#include "dwfl_bias_abi.h"

#include <assert.h>
#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#ifdef BUILD_NEXT
int dwfl_marker;
int module_marker;

Dwfl_Module *dwfl_report_elf(Dwfl *dwfl, const char *name,
                           const char *file_name, int fd,
                           GElf_Addr base, bool add_p_vaddr) {
    assert(dwfl == (Dwfl *)&dwfl_marker);
    assert(strcmp(name, "stable-module") == 0);
    assert(fd == -1);
    printf("%" PRIx64 " %d %d %s %s\n", base, add_p_vaddr, fd, name, file_name);
    return (Dwfl_Module *)&module_marker;
}
#else
extern int dwfl_marker;
extern int module_marker;

static void report(const char *filename, GElf_Addr base, bool add_p_vaddr) {
    Dwfl_Module *module = dwfl_report_elf((Dwfl *)&dwfl_marker, "stable-module",
                                        filename, -1, base, add_p_vaddr);
    assert(module == (Dwfl_Module *)&module_marker);
}

int main(int argc, char **argv) {
    assert(argc == 4);
    report(argv[1], 0x5000, false);
    report(argv[2], 0x5000, false);
    report(argv[3], 0x5000, false);
    report(argv[1], 0x6000, false);
    report(argv[1], 0x5000, true);
    report(argv[1], 0x4000, false);
    report(argv[1], 0x5000, false);
    return 0;
}
#endif
