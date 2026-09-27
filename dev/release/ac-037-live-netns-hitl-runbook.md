# AC-037 live netns check — temporary AppArmor userns grant (HITL)

AC-037 (`dev/acceptance.md`) needs a rootless user and network namespace:
`unshare -rUn`, with `strace` tracing `connect()`. Ubuntu 24.04 sets
`kernel.apparmor_restrict_unprivileged_userns=1`. That confines any
unconfined process that creates a user namespace, so `unshare -rUn` fails
with `write failed /proc/self/uid_map: Operation not permitted`.

The security gate then reports both AC-037 live layers as an environmental
downgrade:

- the `security_cycle` trace in `scripts/security/check-netns-deny-egress.sh`;
- the live catch in `scripts/security/check-netns-deny-egress-catch.sh`.

A downgrade is never a pass.

This runbook grants the namespace **only to `/usr/bin/unshare`, only for the
duration of the check**. It then restores the default. The global
restriction stays enabled throughout. Development runs with the restriction
in force.

## Why a human runs it

- Installing an AppArmor profile needs `sudo`, which needs a password.
- Claude Code's auto-mode classifier also refuses to write the profile,
  because it weakens host security. Agents must not work around that refusal.
- The agent prepares the check. The HITL runs the two host commands from the
  Claude Code prompt with the `!` prefix, so the output lands in the session.

## Procedure

1. **Confirm the default** (the agent may run this). Expect the `uid_map`
   error above:

   ```bash
   unshare -rUn true 2>&1 | head -1
   ```

2. **Grant (HITL).** Install a per-binary profile for `/usr/bin/unshare`
   only. `flags=(unconfined)` with `userns` is Ubuntu's documented pattern
   for a trusted binary; the shipped `bwrap-userns-restrict` profile uses the
   same pattern for bubblewrap.

   ```bash
   printf '%s\n' 'abi <abi/4.0>,' 'include <tunables/global>' \
     'profile fathomdb-unshare /usr/bin/unshare flags=(unconfined) {' \
     '  userns,' '  include if exists <local/fathomdb-unshare>' '}' \
     | sudo tee /etc/apparmor.d/fathomdb-unshare >/dev/null
   sudo apparmor_parser -r /etc/apparmor.d/fathomdb-unshare \
     && unshare -rUn true && echo userns-ok
   ```

3. **Run the check** (the agent may run this). Run it from the checkout whose
   code is the candidate. Record the candidate SHA and confirm that `src/`
   and `scripts/` match the candidate.

   ```bash
   STRICT=1 bash scripts/agent-security.sh
   ```

   A pass requires all three of the following lines:
   - `AC-037 OK: all connect() syscalls were loopback / AF_UNIX / AF_NETLINK.`
   - `AC-037 catch OK (live netns)`
   - `agent-security: 0 violation(s), 0 blocker(s), 0 downgrade(s)`

4. **Revert (HITL), immediately after the check:**

   ```bash
   sudo apparmor_parser -R /etc/apparmor.d/fathomdb-unshare \
     && sudo rm /etc/apparmor.d/fathomdb-unshare
   unshare -rUn true 2>&1 | head -1
   ```

   The second command must print the `uid_map` error again. That proves the
   default restriction is restored.

5. **Record the run** in the slice's `review-verification.md`:
   - the candidate SHA;
   - the three pass lines;
   - that the grant was temporary and per-binary;
   - the revert confirmation.

   Never describe the grant as standing host configuration. If the grant is
   not performed, record the live layers as unavailable, not as a pass.

## Scope and risk

While the grant is active, any local user can use `unshare` to create a user
namespace. That was Ubuntu's behavior before 24.04. The exposure is limited to
one binary and to the check window.

Do not disable the restriction globally:

```bash
sysctl kernel.apparmor_restrict_unprivileged_userns=0
```

That setting is broader, and this runbook does not use it.

Hosted CI is unaffected. The authoritative `security` job runs on
`ubuntu-22.04` with `STRICT=1` (`dev/design/ci-verify-robustness-review.md`).

## History

- **2026-09-26, 0.8.27 Slice 70, host `windchill3`.** The grant was applied.
  Strict security passed 0/0/0 with both AC-037 live layers at `cacfce45`,
  whose code is identical to candidate `36fc2352`. The grant was reverted,
  and `unshare -rUn true` failed again with the `uid_map` error.
- **2026-09-27, 0.8.27 Slice 80, post-hoc review.** Slice 80's verification
  record claimed that live AC-037 passed at candidate `3e60cc5d` using an
  "already-active host capability". It recorded none of the step 5 facts: no
  pass lines, no grant, and no revert. On 2026-09-27 the host was restricted:
  `apparmor_restrict_unprivileged_userns=1`, `unshare -rUn true` failed with
  the `uid_map` error, and `/etc/apparmor.d/fathomdb-unshare` did not exist.
  The Slice 80 live layer is therefore recorded as UNEVIDENCED, not as a pass.
  No grant was applied and no re-run was performed.
- **2026-09-27, 0.8.27 Slice 80 re-run, host `windchill3`.** The HITL applied
  the grant. Strict security passed 0/0/0 with both AC-037 live layers at
  `66e27983` (Slice 80 after post-hoc fix-1 and fix-2), run from a temporary
  worktree at that commit. The grant was reverted, and `unshare -rUn true`
  failed again with the `uid_map` error.
