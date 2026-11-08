#include "dwfl_bias_abi.h"

#include <assert.h>
#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#ifdef BUILD_NEXT
int receiver_dwfl;
int reported_module;
int existing_module;
static int scenario;
static const char *target;

void receiver_state(int value, const char *filename) {
    scenario = value;
    target = filename;
}

Dwfl_Module *dwfl_report_elf(Dwfl *dwfl, const char *name,
                           const char *file_name, int fd,
                           GElf_Addr base, bool add_p_vaddr) {
    assert(dwfl == (Dwfl *)&receiver_dwfl);
    assert(strcmp(name, "stable-module") == 0);
    assert(strcmp(file_name, target) == 0);
    assert(!add_p_vaddr);
    printf("next base=%" PRIx64 " fd=%d\n", base, fd);
    return (Dwfl_Module *)&reported_module;
}

Dwfl_Module *dwfl_addrmodule(Dwfl *dwfl, Dwarf_Addr address) {
    assert(dwfl == (Dwfl *)&receiver_dwfl);
    assert(address == 0x4000);
    assert(scenario != 5);
    return scenario == 0 ? NULL : (Dwfl_Module *)&existing_module;
}

const char *dwfl_module_info(Dwfl_Module *module, void ***userdata,
                             Dwarf_Addr *start, Dwarf_Addr *end,
                             Dwarf_Addr *dwbias, Dwarf_Addr *symbias,
                             const char **mainfile, const char **debugfile) {
    assert(module == (Dwfl_Module *)&existing_module);
    assert(userdata == NULL && dwbias == NULL && symbias == NULL && debugfile == NULL);
    *start = scenario == 3 ? 0x3800 : 0x4000;
    *end = scenario == 4 ? 0x4000 : 0x5000;
    *mainfile = scenario == 2 ? "/fixture/other-native.so" : target;
    return "existing-native";
}
#else
extern int receiver_dwfl;
extern int reported_module;
extern int existing_module;
void receiver_state(int value, const char *filename);

int main(int argc, char **argv) {
    assert(argc == 2);
    for (int scenario = 0; scenario < 6; ++scenario) {
        receiver_state(scenario, argv[1]);
        Dwfl_Module *module = dwfl_report_elf((Dwfl *)&receiver_dwfl, "stable-module",
                                             argv[1], scenario == 5 ? 77 : -1,
                                             0x5000, false);
        assert(module == (Dwfl_Module *)&reported_module
               || module == (Dwfl_Module *)&existing_module);
        printf("existing=%d\n", module == (Dwfl_Module *)&existing_module);
    }
    return 0;
}
#endif
