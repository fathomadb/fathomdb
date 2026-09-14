# Slice 15 prototype chronology

- `f1e608c2`: transient RED contract tests; compiler reports absent graph
  evidence types and methods.
- `79aca61b`: fixture validity correction removes edge bodies that required an
  unrelated embedder; assertions unchanged.
- `ad640796`: fixture validity correction makes the explicit seed satisfy the
  existing target-kind filter; assertions unchanged.
- `b6478d1f`: first test-hooks-only GREEN shortcut.
- `6235daab`: release-mode exploratory harness.

The two RED corrections repair setup preconditions and do not weaken any
oracle. The subsequent exploratory result exposed that the shortcut is not the
approved post-selection hydration design. It is retained only to make that
finding auditable before planned teardown.
