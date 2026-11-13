# Windows capacity-only operation

This proposal implements owner2026-11-13 seq-370: creation, necessary narrow ACL
and live attachment of one64GiB scratch disk in existing gh-runner-wonl-win11.
Independent Astra-medium technical review of this derivative precedes execution.
Actual ruling is settled; no new generic approval or campaign is requested.
Nothing in this packet has executed VM/storage/compiler/helper/test mutations.

## Exact inputs and allowed steps

Frozen predecessor directory is
/mnt/data/tmp2/fathomdb-0829-continuation/slice20/windows-operation-proposal-v2-frozen;
manifest SHA2565122aa082974461cb8a6f04e2f96accd645c19554844419cd43131355171aafa.
Its provision-host.py SHA256 is
30f3c681ce3a4fa6da1a1fdea9ac3e07545655b127c79b6aed56ffa6ff756075.
Preserve that packet unchanged. It invokes guest initialization/format and MUST
NOT be executed under capacity-only authority. capacity-only.py is the narrow
derivative; capacity-only.py.diff shows every change. manifest.json binds the
new source/docs; input-pins.json binds actual Git inputs. No old static review
is promoted into review or runtime approval of this derivative.

1. Validate frozen and derivative hashes and actual parent seq-370 record. Parent
   supplies operation=capacity-only-create-attach, approved=true, exact derivative
   manifest hash and released native window. This records existing authority;
   it does not solicit another user decision. Refuse false/missing identity.
2. Refresh VM live/inactive XML. Require name gh-runner-wonl-win11,
   UUID3a928939-57ed-4c62-999b-b610f3e44d42, original OS sda image
   /var/lib/libvirt/images/gh-runner-wonl-win11.qcow2, qemu-xhci USB0 with port10
   available, target sdg and serialFDB29S20A6 unused. Require sole matching QEMU
   process UID64055. Refresh authenticated existing guest hostname
   MEMEX-AFE8V4I00 and read-only disks/drives/process/RAM inventory; refuse
   occupied R: or native build window. No guest executable/compiler is launched.
3. Require wholly absent nonsymlink images-only directory
   /mnt/data/tmp2/fathomdb-0829-continuation/slice20-windows-images-approved-20261113
   and image windows-a6-64g.qcow2. Create only that directory0700 and image qcow2,
   exactly68,719,476,736B virtual size, no backing image. Grant only UID64055
   execute on this directory and read/write on this image; verify ACL output.
   Parent directories already traverse; no shared-parent/private-task ACL change.
4. Attach exactly source image, sdg/USB0:10, serialFDB29S20A6 via virsh
   attach-device --live, without --config/--persistent. Verify exact attachment,
   unchanged previous disk inventory and identical inactive XML.
5. Read guest Get-Disk inventory only. Require exactly one new UniqueId absent
   from preinventory; Number is not0; exact serial/64GiB; nonboot/nonsystem; RAW;
   original guest disks unchanged. PartitionStyle is converted to a string
   explicitly before JSON. Write ATTACHED_RAW_CAPACITY_ONLY result and STOP.

No GPT initialization, partition, format, mount/drive assignment, source transfer,
compiler/helper/test, VM restart/shutdown/persistent topology, detach/delete,
cleanup, installer, alternate compiler or confinement weakening is performed.
Any timeout/failure/partial attachment leaves UNKNOWN and stops later actions.
No blind retry or automatic rollback. A later read-only inventory can resolve
what happened; uncertain state never authorizes a second disk or guest mutation.

## Capacity and current readiness

Use reviewed host start floors32GiB root,250GiB taskfs and16GiB MemAvailable.
Total owned cap200GiB includes max(task plus source actual bytes,128GiB
combined Linux source/build/control reservation) plus max(actual image,64GiB
image reservation). This reserves192GiB, leaving8GiB headroom.
Source accounting is bound to parent-created fresh sole-writer checkout
/mnt/data/worktrees/fathomdb-0.8.29-s20-approved-20261113.
Do not sum free-space readings from co-located filesystems or count sparse image
allocation as its full future budget. Refresh before each mutating host step.

Read-only November13 observations: root80,429,350,912B; /mnt/data609,778,946,048B;
MemAvailable30,873,688KiB. VM live XML still has original sda plus three read-only
media and qemu-xhci15ports; DAC UID64055/GID993. Guest has only Disk0/QM00001 GPT,
C:7,055,458,304B (6.57GiB), RAM30,404,480KiB and no cargo/rustc/cl/ninja processes.
These samples are observations, not reservations or future readiness receipts.
C: is below existing8GiB workload start floor. Capacity-only writes nothing on C:
and removes that inapplicable inherited formatting/build floor for this operation
only. Later guest setup/build/test still needs C:8/5GiB start/stop, workspace32/20GiB
and guest RAM16/8GiB. No waiver, cleanup or inferred formatting authority follows.

## Invocation after exact technical PASS

Parent prepares its actual authority JSON under the task directory. Execute the
reviewed file, with all Python temporary/cache output under the assigned task:

```bash
python3 /mnt/data/tmp2/fathomdb-0829-continuation/slice20/approved-2026-11-13/capacity-only.py --approval /mnt/data/tmp2/fathomdb-0829-continuation/slice20/actual-capacity-only-authority.json
```

Original stdout/stderr/exits/command initiation and timeout partial output go to
new task/capacity-only-<random> files. Success proves attached RAW capacity only.
A6 compiler/helper/test sequence remains HELD for technical readiness and actual
broader authority. Frozen5122's PowerShell parser/ABI/compiler/binary/private-job
controls remain unexecuted. Existing four A6 selectors and product oracles remain
unchanged. No Slice20/30/Block1/platform/release closure follows.

## Contained boundary verification before live operation

Before actual execution, run a local isolated harness against the actual derivative
main path. Stub only external subprocess/host/guest-inventory/resource boundaries;
real temporary fixture directories and actual policy branches execute. No database
is mocked; no virsh/SSH/compiler/storage command reaches the real host or guest.
The harness records every requested action, raw exception and returned status.

- Healthy fixture reaches exactly one create, two narrow ACL updates, one --live
  attach, before/after read-only guest inventory and ATTACHED_RAW_CAPACITY_ONLY.
  It never submits provision-guest.ps1, Initialize-Disk, New-Partition,
  Format-Volume, drive assignment, compiler/helper/test or persistent topology.
- Run the unchanged frozen predecessor under its own synthetic full authority as
  a capability witness: it reaches the stubbed guarded guest sequence. This proves
  why it cannot be selected for this capacity-only operation; it is not an old
  product defect or fabricated product RED. The derivative's same no-guest-mutation
  oracle is required to pass with its own capacity-only authority.
- Wrong derivative manifest or modified script, false/wrong ruling/operation,
  occupied image/directory, mismatched UUID/QEMU UID or occupied port/target/serial
  refuses before image/ACL/attach. Verify reached counts, not only exception text.
- Ambiguous/mismatched new guest disk or changed old-disk/inactive XML after attach
  leaves UNKNOWN; no initialization/format/retry/detach/delete follows. An injected
  attach timeout stops later requests and retains initiation/partial output.

These are operation-boundary qualification controls, not the A3/A5 policy exception.
Any actual derivative defect found by them keeps the observed failing assertion
before correction and unchanged assertion afterward. Do not invent a product RED
or treat a passing source parser as evidence these branches ran. Retain actual
results for the parent/reviewer before the already-authorized live capacity work.
