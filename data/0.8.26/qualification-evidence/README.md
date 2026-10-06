# Qualification evidence index

| Directory | Contents |
| --- | --- |
| [slice-90/](slice-90/README.md) | FathomDB 0.8.27 Slice 90 runtime, performance, GPU, binding, and final-candidate evidence. |

The release checkpoint validates repository-held receipts on every host. On
windchill3 it also revalidates these retained bundles. A host without the
bundle directory reports `skipped: qualification evidence not on this host`;
a present but incomplete or changed bundle fails validation.
