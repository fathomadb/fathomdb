---
title: FathomDB 0.8.26 Slice 3 — draft product and architecture CRUD
status: DRAFT
---

# Slice 3 plan — draft product and architecture CRUD

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It is draft-only: reconcile intervening work and allocated items, complete and
independently review the draft contracts/design, verify traceability, write
status, and perform no behavioral implementation.

## Purpose

Translate the selected Memex P0–P2 needs into reviewable draft changes to
FathomDB needs, requirements, acceptance criteria, interfaces, ADRs, and
architecture. Allocate every draft to one delivery slice. This slice changes
no accepted contract and performs no implementation.

## Read-only method

1. Reconcile the Memex prioritization scaffold with current needs,
   requirements, acceptance criteria, ADRs, and interface documents.
2. Classify each proposed document operation as create, update, rename,
   deprecate-in-place, or delete.
3. Give each draft requirement an owning Slice 10, 20, 30, 40, or 50.
4. Identify public compatibility or successor-type decisions that Slice 8
   must resolve.
5. Produce a complete draft trace from need through acceptance evidence.

## Deliverables

- the draft contract register in the companion design note;
- exact candidate files and CRUD operations;
- allocation of every draft to one feature slice; and
- unresolved decisions for Slices 4–8.

## Exit criteria

The draft set is complete, non-overlapping, and explicitly non-authoritative.
No product, contract, ADR, generated view, or implementation file is edited.
