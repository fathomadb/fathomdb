#ifndef FATHOMDB_DWFL_BIAS_ABI_H
#define FATHOMDB_DWFL_BIAS_ABI_H

#include <elf.h>
#include <stdbool.h>

/* elfutils libdwfl/dwfl_report_elf.c: public native ELF64 ABI. */
typedef struct Dwfl Dwfl;
typedef struct Dwfl_Module Dwfl_Module;
typedef Elf64_Addr GElf_Addr;
typedef Elf64_Addr Dwarf_Addr;

Dwfl_Module *dwfl_report_elf(Dwfl *dwfl, const char *name,
                           const char *file_name, int fd,
                           GElf_Addr base, bool add_p_vaddr);

Dwfl_Module *dwfl_addrmodule(Dwfl *dwfl, Dwarf_Addr address);
const char *dwfl_module_info(Dwfl_Module *module, void ***userdata,
                             Dwarf_Addr *start, Dwarf_Addr *end,
                             Dwarf_Addr *dwbias, Dwarf_Addr *symbias,
                             const char **mainfile, const char **debugfile);

#endif
