#ifndef FATHOMDB_DWFL_BIAS_ABI_H
#define FATHOMDB_DWFL_BIAS_ABI_H

#include <elf.h>
#include <stdbool.h>

/* elfutils libdwfl/dwfl_report_elf.c: public native ELF64 ABI. */
typedef struct Dwfl Dwfl;
typedef struct Dwfl_Module Dwfl_Module;
typedef Elf64_Addr GElf_Addr;

Dwfl_Module *dwfl_report_elf(Dwfl *dwfl, const char *name,
                           const char *file_name, int fd,
                           GElf_Addr base, bool add_p_vaddr);

#endif
