# Slice 7 delivery delay register

Source-only review, November 10 at `6edc1155f`; all changes proposed for 9.

| Finding | Evidence / classification | Smallest proposed action / owner |
| --- | --- | --- |
| D7-1 owned setup missing | [Verifier](../../../../../scripts/agent-verify.sh) requires local venv; fresh checkout has none and no npm modules | 0/1→10: pinned command-local tools, owned venv/wheel/dependencies; no full build before setup proof. |
| D7-2 editable-install conflict | [Bootstrap](../../../../../scripts/bootstrap.sh) and [build](../../../../../scripts/agent-build.sh) install editable; AGENTS forbids worktree rebinding | 10: reviewed linked-worktree setup path and corresponding instructions/tests. |
| D7-3 root capacity | Current root/tmp about 16 GiB free, versus large native build/artifact phases; large /mnt/data capacity is only a candidate | 0→9/10: owned scratch/build location, measured phase budget, conservative concurrency; no implicit cleanup or use of existing standing install. |
| D7-4 first-tag route | [Tag guard](../../../../../scripts/release/check-tag-cuda-route.sh) explicitly permits bootstrap only for `v0.8.28` | 7/9→10: choose exact .29 route/amendment; preserve current access protections; stop affected rehearsal if unavailable. |
| D7-5 SDK trusted publisher | [Release design](../../../../design/release.md) and [.28 status](../../../0.8.28/features/slice-95/status.md) already record saved publisher and OIDC-only run | Keep fixed; no token recovery or repeated settings operation. Source-recorded auth proof is not current package qualification. |
| D7-6 historical verification | .28 status explicitly preserves failed full CPU run and scoped fixes | 160: reuse only relevant unchanged evidence; don't relabel assembled qualification uninterrupted green. |
| D7-7 history scan | [Scanner](../../../../../scripts/security/gitleaks-history.sh) runs current scan, isolated mirror, redacted report and manifest classifier | Retain controls; scope new classification to actual changed findings; no scan run or new leak claim today. |
| D7-8 Markdown setup | [Markdown gate](../../../../../scripts/agent-lint-md.sh) hard-fails without local binary; fresh checkout has no root modules | 10: owned pinned tooling. Interim direct available-linter checks disclose source-version mismatch and are not the owned authoritative gate. |

No workflow dispatched, external access changed, registry upload attempted or
performance saving measured. Historical runner identities are in Slice 0;
current candidate access must be checked before the corresponding campaign.
