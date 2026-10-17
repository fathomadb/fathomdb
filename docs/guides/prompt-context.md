# Prompt context from search hits

`pack_context` turns ranked search hits into prompt text within a
character budget. The first few items are shown in full. The rest keep the
parts that usually hold the answer, by default the user's own words, and show
only a short excerpt of the others. Every item comes back with provenance, so
you can see what was cut.

## Render hits

```python
from fathomdb import ContextItem, ContextPart, ContextPolicy, pack_context

hits = [
    ContextItem(
        id="round-1",
        parts=(
            ContextPart("user", "I moved to Oslo in May."),
            ContextPart("assistant", "Oslo is a lovely city with long summer days..."),
        ),
        header="[Session date: 2023/05/20]",
    ),
    # ... more ranked items
]
pack = pack_context(hits, ContextPolicy(budget_chars=30000, full_items=3, compact_chars=300))
prompt_evidence = pack.text
for entry in pack.items:
    print(entry.id, entry.tier, entry.chars, entry.truncated)
```

TypeScript uses `packContext(hits, { budgetChars: 30000 })`; Rust uses
`fathomdb_sdk::pack_context(&hits, &policy, &[])`. The three return the same
text.

The defaults are 30,000 characters in total, 4,000 per item, the top 3
items in full, and a 300-character excerpt shared by the non-user parts of
each later item. If your answers live in assistant or tool text, add those
roles to `keep_roles`. Headers are not charged to the budget; `text_chars`
gives the full length.

To show particular items in full wherever they rank, list their ids in
`full_ids` (TypeScript `fullIds`), for example every hit from the top hit's
session. They do not use up `full_items`, and they still count against the
budget, so a late one can be cut. Each entry's `forced` says whether
`full_ids` changed its tier.

## Add the rest of a conversation

Give each item a `group` (for example the session) and a `position`, and pass
the other rounds of those groups as `context`. After the hits, each hit's
group is filled in, in position order, until the budget runs out. Set
`order="chronological"` and an `order_key` (for example the date) to print
the selected items oldest first.

## Keep roles in stored rows (optional)

To compact by role straight from search hits, store each round with
`encode_round_body(parts)` and read it back with `context_item_from_hit(hit)`.
The tagged body ranks exactly like the plain text in text and hybrid search.
It contains control characters (U+001E and U+0010–U+0014), so do not split
it with `str.splitlines()`; use `decode_round_body`. Untagged rows still work:
they are treated as one part that is never compacted.
