---
title: FathomDB 0.8.26 Slice 7 — CI/CD and release-delivery evidence review
status: DRAFT
---

# Slice 7 plan — CI/CD and release-delivery evidence review

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It is evidence-only: reconcile newly available transcripts and delivery work,
independently review the failure model, verify claims with non-publishing
checks where safe, write status, and perform no CI/CD or registry mutation.

## Purpose

Review available evidence, including relevant transcripts, for failures after
product builds were working: CI, packaging, gitleaks, artifact signing or
provenance, publishing, registry, and related release-delivery problems.
Propose pragmatic corrections without changing CI/CD or publishing anything.

## Evidence scope

- GitHub Actions runs, annotations, logs, artifacts, and retry history;
- release workflows, action pins, packaging scripts, manifests, and target
  matrices;
- gitleaks output and secret-remediation records;
- PyPI, npm, crates.io, GitHub Release, and native artifact publication
  evidence where available;
- post-publish smoke results, release boards, issues/PRs, handoffs, and
  relevant agent transcripts; and
- authentication, network, runner, cache, signing, provenance, and registry
  status evidence that affected delivery.

No registry write, workflow dispatch, tag, credential test, or publication is
authorized. Sanitize secrets and tokens from durable findings.

## Review method

1. Establish the last known successful product-build boundary for each release
   incident, then trace CI through packaging, scanning, artifact assembly,
   publication, and post-publish smoke.
2. Separate product regression from workflow/configuration defect, package
   metadata or contents defect, target-runner failure, gitleaks true/false
   positive, credential/permission failure, registry behavior, and transient
   infrastructure failure.
3. Identify retries or manual workarounds and determine whether they masked a
   deterministic cause or were appropriate for a documented transient.
4. Consolidate repeated symptoms into root causes and quantify release delay,
   recurrence, blast radius, and current exposure.
5. Propose the smallest durable correction with a non-publishing test route.
6. Recommend Slice 9 only for feature-independent preparation. Allocate
   feature-dependent hardening after Slice 10 in a reserved gap, and final
   artifact or release-route proof to Slice 50. Slice 8 makes the decision.

## Pragmatism rules

- Prioritize deterministic failures that block all publication targets.
- Prefer local/non-publishing workflow validation before consuming registry or
  signing attempts.
- Keep action pins and supply-chain checks fail-closed.
- Do not weaken gitleaks, suppress a real secret, broadly expand credentials,
  or make registry success the first test of packaging correctness.
- Treat target-specific packaging truth separately from source build success.

## Deliverables and exit criteria

Produce a sourced CI/CD failure register with chronology, stage, exact symptom,
classification, root-cause confidence, recurrence, delay, workaround,
candidate correction, non-publishing proof, risk, effort, and proposed slice.
Record unavailable evidence and external ownership. No workflow, package,
secret rule, tag, registry, or release state is changed.
