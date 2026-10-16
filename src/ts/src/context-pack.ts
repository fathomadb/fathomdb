// Private context-pack owner for the TypeScript SDK package root (0.8.28 Slice 41).
//
// All logic is the engine's single Rust implementation; this module converts
// camelCase values to and from its JSON wire, so TypeScript, Python and Rust
// return identical output. Counts are Unicode scalar values, not UTF-16 code
// units: `chars`, `ofChars` and `textChars` differ from `string.length` for
// astral-plane text.

import { native } from "./binding.js";
import { interceptSync } from "./native-call.js";
import type { SearchHit } from "./search.js";

/** One piece of an item: free-text `role` (or `null`) and its text. */
export interface ContextPart {
  role: string | null;
  text: string;
}

/**
 * One caller item. `group` and `position` drive group expansion, `orderKey`
 * drives chronological order, and `header` is rendered verbatim on its own
 * line above the item text without being charged to the budget.
 */
export interface ContextItem {
  id: string;
  parts: readonly ContextPart[];
  group?: string | null;
  position?: number | null;
  orderKey?: string | null;
  header?: string | null;
}

/** Budget and tier settings; omitted fields take {@link DEFAULT_CONTEXT_POLICY}. */
export interface ContextPolicy {
  budgetChars?: number;
  itemChars?: number;
  fullItems?: number;
  compactChars?: number;
  keepRoles?: readonly string[];
  expandGroups?: boolean;
  order?: "rank" | "chronological";
}

/** The product defaults: top 3 full, 300-character excerpt, keep user text. */
export const DEFAULT_CONTEXT_POLICY: Readonly<Required<ContextPolicy>> = Object.freeze({
  budgetChars: 30000,
  itemChars: 4000,
  fullItems: 3,
  compactChars: 300,
  keepRoles: Object.freeze(["user"]),
  expandGroups: true,
  order: "rank",
});

/** Rendered characters of one part out of its full length. */
export interface ContextPartEntry {
  role: string | null;
  chars: number;
  ofChars: number;
}

/**
 * Provenance of one input item. `chars` counts the item text including the
 * `"\n"` joins between parts; `truncated` is true when a rendered item shows
 * fewer characters of some part than it has, and false for omitted items.
 */
export interface ContextPackEntry {
  id: string;
  source: "hit" | "context";
  tier: "full" | "compact" | "omitted";
  chars: number;
  truncated: boolean;
  renderedIndex: number | null;
  parts: ContextPartEntry[];
}

/**
 * Rendered text plus one provenance entry per input item, hits then context,
 * in input order. `charsUsed` is the sum of item `chars`; `textChars` is the
 * scalar length of `text`, headers and separators included.
 */
export interface ContextPack {
  text: string;
  charsUsed: number;
  textChars: number;
  items: ContextPackEntry[];
}

const ID_PREFIX: Record<string, string> = { logical: "l:", content: "h:", passage: "p:" };

function partWire(part: ContextPart): Record<string, unknown> {
  return { role: part.role, text: part.text };
}

function itemWire(item: ContextItem): Record<string, unknown> {
  const wire: Record<string, unknown> = { id: item.id, parts: item.parts.map(partWire) };
  if (item.group !== undefined && item.group !== null) wire.group = item.group;
  if (item.position !== undefined && item.position !== null) wire.position = item.position;
  if (item.orderKey !== undefined && item.orderKey !== null) wire.order_key = item.orderKey;
  if (item.header !== undefined && item.header !== null) wire.header = item.header;
  return wire;
}

function policyWire(policy: ContextPolicy): Record<string, unknown> {
  const wire: Record<string, unknown> = {};
  if (policy.budgetChars !== undefined) wire.budget_chars = policy.budgetChars;
  if (policy.itemChars !== undefined) wire.item_chars = policy.itemChars;
  if (policy.fullItems !== undefined) wire.full_items = policy.fullItems;
  if (policy.compactChars !== undefined) wire.compact_chars = policy.compactChars;
  if (policy.keepRoles !== undefined) wire.keep_roles = Array.from(policy.keepRoles);
  if (policy.expandGroups !== undefined) wire.expand_groups = policy.expandGroups;
  if (policy.order !== undefined) wire.order = policy.order;
  return wire;
}

interface WireEntry {
  id: string;
  source: "hit" | "context";
  tier: "full" | "compact" | "omitted";
  chars: number;
  truncated: boolean;
  rendered_index: number | null;
  parts: { role: string | null; chars: number; of_chars: number }[];
}

/**
 * Render ranked `hits` (and, with group expansion, `context`) into prompt
 * context within the policy's character budget.
 *
 * Selection is the hits in order, then for each hit group (in order of its
 * first hit) the `context` items of that group by `position`. The first
 * `fullItems` selected items show every part; the rest keep `keepRoles` parts
 * (and role-less parts) whole and share `compactChars` among the others.
 *
 * @throws InvalidArgumentError naming the field, for a zero `budgetChars` or
 *   `itemChars`, a negative or non-integer count, an unknown `order`, an empty
 *   or repeated id, or a grouped `context` item without `position` while
 *   `expandGroups` is on.
 */
export function packContext(
  hits: readonly ContextItem[],
  policy: ContextPolicy = {},
  context: readonly ContextItem[] = [],
): ContextPack {
  const request = JSON.stringify({
    hits: hits.map(itemWire),
    policy: policyWire(policy),
    context: context.map(itemWire),
  });
  const wire = JSON.parse(interceptSync(() => native.packContextJson(request))) as {
    text: string;
    chars_used: number;
    text_chars: number;
    items: WireEntry[];
  };
  return {
    text: wire.text,
    charsUsed: wire.chars_used,
    textChars: wire.text_chars,
    items: wire.items.map((entry) => ({
      id: entry.id,
      source: entry.source,
      tier: entry.tier,
      chars: entry.chars,
      truncated: entry.truncated,
      renderedIndex: entry.rendered_index,
      parts: entry.parts.map((p) => ({ role: p.role, chars: p.chars, ofChars: p.of_chars })),
    })),
  };
}

/**
 * Encode parts as an opt-in role-tagged round body: each part is U+001E, a
 * role code (role `null` U+0010, `user` U+0011, `assistant` U+0012, `system`
 * U+0013, `tool` U+0014) and the text, joined by `"\n"`. Removing the markers
 * gives the plain `"\n"`-joined texts, and FTS ranking is unchanged.
 *
 * @throws InvalidArgumentError for an empty part list, another role, or a
 *   text containing U+001E or a role code.
 */
export function encodeRoundBody(parts: readonly ContextPart[]): string {
  const request = JSON.stringify(parts.map(partWire));
  return interceptSync(() => native.encodeRoundBodyJson(request));
}

/**
 * Decode a body into parts; an untagged body is one part with role `null`.
 *
 * @throws InvalidArgumentError when the body starts with U+001E but does not
 *   decode.
 */
export function decodeRoundBody(body: string): ContextPart[] {
  if (typeof body !== "string") {
    throw new TypeError(`body must be a string, got ${typeof body}`);
  }
  const request = JSON.stringify(body);
  const parts = JSON.parse(interceptSync(() => native.decodeRoundBodyJson(request))) as ContextPart[];
  return parts.map((p) => ({ role: p.role, text: p.text }));
}

/** Optional item fields for {@link contextItemFromHit}. */
export interface ContextHitOptions {
  group?: string | null;
  position?: number | null;
  orderKey?: string | null;
  header?: string | null;
}

/**
 * Build a {@link ContextItem} from a search hit: the id is the hit's prefixed
 * id (`l:`, `h:` or `p:` plus the bare value) and the parts are
 * `decodeRoundBody(hit.body)`.
 *
 * @throws InvalidArgumentError when the hit body starts with U+001E but does
 *   not decode.
 * @throws RangeError for an unknown id space.
 */
export function contextItemFromHit(hit: SearchHit, options: ContextHitOptions = {}): ContextItem {
  const prefix = ID_PREFIX[hit.id.space];
  if (prefix === undefined) {
    throw new RangeError(`unknown hit id space ${hit.id.space}`);
  }
  return {
    id: `${prefix}${hit.id.value}`,
    parts: decodeRoundBody(hit.body),
    group: options.group ?? null,
    position: options.position ?? null,
    orderKey: options.orderKey ?? null,
    header: options.header ?? null,
  };
}
