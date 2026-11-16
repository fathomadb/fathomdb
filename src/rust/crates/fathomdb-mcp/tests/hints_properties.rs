//! Hints sidecar properties over synthetic pages, through the shaping seam
//! (`fathomdb_mcp::seams`, review fix 1): ref subsets, content freedom, and
//! kept-count consistency (hints-sidecar § 8 tests 2, 3, 10).

use fathomdb_mcp::seams::{prepare_search_page, shape_search_page, SearchPageInput, SyntheticHit};
use proptest::prelude::*;
use serde_json::{json, Value};

fn ranked(lengths: &[usize]) -> Vec<SyntheticHit> {
    lengths
        .iter()
        .enumerate()
        .map(|(index, length)| SyntheticHit {
            reference: format!("l:note:{index:03}"),
            kind: "note".to_string(),
            body: "river ".repeat(length / 6 + 1),
            source_id: Some("fixture".to_string()),
            score: 1.0 / (index as f64 + 1.0),
        })
        .collect()
}

fn hints(structured: &Value) -> Vec<Value> {
    structured.get("hints").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn strings(value: &Value) -> Vec<String> {
    value.as_array().map_or_else(Vec::new, |values| {
        values.iter().map(|value| value.as_str().unwrap().to_string()).collect()
    })
}

// Independent wire oracle for the authored `river ` ASCII fixture. It uses
// only JSON serialization, SHA-256 and the closed format/hint specification;
// no adapter renderer, shaper, snippet selector or hint planner is called.
fn reference_handle(prefix: char, parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let hex = hash.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect::<String>();
    format!("{prefix}1-{hex}")
}

fn authored_snippet(body: &str, cap: usize) -> (String, bool, &'static str) {
    assert!(
        body.bytes().enumerate().all(|(i, b)| b == b"river "[i % 6]),
        "oracle fixture alphabet"
    );
    if body.len() <= cap {
        return (body.into(), false, "whole");
    }
    let budget = cap - 4; // authored trailing ` …` is four UTF8 bytes.
    let end = if budget % 6 == 5 { budget } else { budget / 6 * 6 - 1 };
    (format!("{} …", &body[..end]), true, "start")
}

fn reference_page(
    input: &SearchPageInput,
    kept: usize,
    with_hints: bool,
) -> (Value, String, Option<usize>) {
    assert!(!input.hybrid && input.degradation.is_empty());
    let end = (input.offset + input.limit).min(100).min(input.ranked.len());
    let page = &input.ranked[input.offset.min(end)..end];
    let page_limit = input.limit.min(100 - input.offset);
    let cut = kept < page.len();
    let next = ((cut || page.len() >= page_limit) && kept > 0 && input.offset + kept < 100)
        .then_some(input.offset + kept);
    let canonical = format!(
        r#"{{"query":"synthetic","mode":"text","kind":null,"source_type":null,"status":null,"created_after":null,"attributes":[],"as_of":0,"limit":{},"explain":false}}"#,
        input.limit
    );
    let continuation =
        next.map(|next| reference_handle('c', &["s", &canonical, &next.to_string()]));
    let joined =
        page[..kept].iter().map(|hit| hit.reference.as_str()).collect::<Vec<_>>().join("\n");
    let response = reference_handle('r', &["s", &canonical, &input.offset.to_string(), &joined]);
    let hits=page[..kept].iter().map(|hit| {
        let (snippet,snippet_cut,anchor)=authored_snippet(&hit.body,input.snippet_bytes);
        json!({"ref":hit.reference,"kind":hit.kind,"snippet":snippet,"snippet_cut":snippet_cut,"snippet_anchor":anchor,"source_id":hit.source_id,"score":hit.score,"arm":"text"})
    }).collect::<Vec<_>>();
    let mut hints = Vec::new();
    if with_hints {
        if page.is_empty() {
            hints.push(json!({"action":"search","reason":"no_results"}));
        }
        let cut_refs = hits
            .iter()
            .filter(|hit| hit["snippet_cut"] == true)
            .map(|hit| hit["ref"].clone())
            .collect::<Vec<_>>();
        if !cut_refs.is_empty() {
            let mut hint = json!({"action":"fetch","reason":"snippet_cut","refs":cut_refs[..cut_refs.len().min(50)]});
            if cut_refs.len() > 50 {
                hint["refs_omitted"] = json!(cut_refs.len() - 50);
            }
            hints.push(hint);
        }
        if let Some(handle) = &continuation {
            if cut {
                hints.push(json!({"action":"search","reason":"cut_by_budget","continuation":handle,"max_tokens":input.max_tokens}));
            } else if page.len() >= page_limit {
                hints
                    .push(json!({"action":"search","reason":"more_results","continuation":handle}));
            }
        }
    }
    let completeness = if cut {
        "cut_by_budget"
    } else if kept == 0 {
        "empty"
    } else if kept >= page_limit {
        "limit_reached"
    } else {
        "complete"
    };
    let mut out = json!({"store":"s","epoch":1,"validity_instant":0,"degradation":[],"budget":{"max_tokens":input.max_tokens,"used_tokens":0,"cut":cut},"mode":"text","response_id":response,"results":hits,"completeness":completeness,"continuation":continuation});
    if !hints.is_empty() {
        out["hints"] = json!(hints);
    }
    for _ in 0..4 {
        let text = serde_json::to_string(&out).unwrap();
        let mut without = out.clone();
        without.as_object_mut().unwrap().remove("budget");
        let used = (serde_json::to_string(&without).unwrap().len() + text.len()).div_ceil(4) as u64;
        if out["budget"]["used_tokens"] == used {
            return (out, text, next);
        }
        out["budget"]["used_tokens"] = json!(used);
    }
    panic!("independent authored count did not converge");
}

fn reference_shape(input: &SearchPageInput) -> Option<(Value, String, Option<usize>, usize)> {
    let available =
        input.ranked.len().min((input.offset + input.limit).min(100)).saturating_sub(input.offset);
    for with_hints in [input.hints == Some(true), false] {
        let fitting = (0..=available).rev().find_map(|kept| {
            let (payload, text, next) = reference_page(input, kept, with_hints);
            ((2 * text.len()).div_ceil(4) <= input.max_tokens as usize)
                .then_some((payload, text, next, kept))
        });
        if with_hints && fitting.as_ref().is_none_or(|(_, _, _, kept)| *kept == 0 && available > 0)
        {
            continue;
        }
        return fitting;
    }
    None
}

#[test]
fn structured_tiny_budget_has_an_independently_sized_zero_progress_page() {
    let input = SearchPageInput {
        ranked: ranked(&[300]),
        limit: 10,
        snippet_bytes: 300,
        max_tokens: 256,
        hints: Some(true),
        ..Default::default()
    };
    let (expected, text, next, kept) = reference_shape(&input).unwrap();
    assert_eq!(kept, 0);
    assert_eq!(next, None);
    assert_eq!(expected["budget"]["cut"], true);
    assert_eq!(expected["completeness"], "cut_by_budget");
    assert!(expected.get("hints").is_none());
    assert!((2 * text.len()).div_ceil(4) <= 256);
    let shaped = shape_search_page(&prepare_search_page(&input));
    assert_eq!(shaped.kept, kept, "the authored whole hit exceeds the current-format budget");
    assert_eq!(shaped.structured, expected);
    assert_eq!(shaped.text, text);
    assert_eq!(shaped.next_offset, next);
}

#[test]
fn structured_hint_fallback_preserves_the_sole_otherwise_fitting_hit() {
    let mut input = SearchPageInput {
        ranked: ranked(&[300]),
        limit: 10,
        snippet_bytes: 120,
        hints: Some(true),
        ..Default::default()
    };
    let budget = (256..=1000)
        .find(|budget| {
            input.max_tokens = *budget;
            let (_, on, _) = reference_page(&input, 1, true);
            let (_, off, _) = reference_page(&input, 1, false);
            (2 * off.len()).div_ceil(4) <= *budget as usize
                && (2 * on.len()).div_ceil(4) > *budget as usize
        })
        .expect("authored case exercises hint fallback");
    input.max_tokens = budget;
    let (expected, text, _, kept) = reference_shape(&input).unwrap();
    assert_eq!(kept, 1);
    assert!(expected.get("hints").is_none());
    let shaped = shape_search_page(&prepare_search_page(&input));
    assert_eq!(shaped.structured, expected);
    assert_eq!(shaped.text, text);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// § 8 test 2: every hint ref is a returned ref, no hint names a dropped
    /// hit, and the `snippet_cut` hint names exactly the kept cut hits, in
    /// result order, up to 50.
    #[test]
    fn fetch_hint_refs_are_a_subset_of_returned_refs(
        lengths in proptest::collection::vec(0_usize..400, 0..100),
        limit in 1_usize..=100,
        max_tokens in 256_u32..20_000,
        snippet_bytes in 40_usize..120,
    ) {
        let input = SearchPageInput {
            ranked: ranked(&lengths),
            limit,
            snippet_bytes,
            max_tokens,
            hints: Some(true),
            ..Default::default()
        };
        let shaped = shape_search_page(&prepare_search_page(&input));
        let results = shaped.structured["results"].as_array().unwrap().clone();
        let returned: Vec<String> =
            results.iter().map(|hit| hit["ref"].as_str().unwrap().to_string()).collect();
        for hint in hints(&shaped.structured) {
            for reference in strings(&hint["refs"]) {
                prop_assert!(returned.contains(&reference), "{reference} not returned");
            }
        }
        let cut: Vec<String> = results
            .iter()
            .filter(|hit| hit["snippet_cut"] == true)
            .map(|hit| hit["ref"].as_str().unwrap().to_string())
            .collect();
        let hinted: Vec<Value> = hints(&shaped.structured)
            .into_iter()
            .filter(|hint| hint["reason"] == "snippet_cut")
            .collect();
        // A page that keeps hits only by dropping its hints carries none
        // (the hints-on-may-shrink-kept property); otherwise the hint is
        // complete.
        let carries_hints = shaped.structured.get("hints").is_some();
        if !carries_hints {
            // A hints-on page without hints is exactly the hints-off page:
            // either nothing called for a hint, or the fallback dropped them.
            let off = shape_search_page(&prepare_search_page(&SearchPageInput {
                hints: Some(false),
                ..input.clone()
            }));
            prop_assert_eq!(&shaped.structured, &off.structured);
            prop_assert_eq!(&shaped.text, &off.text);
        }
        if cut.is_empty() || !carries_hints {
            prop_assert!(hinted.is_empty());
        } else {
            prop_assert_eq!(hinted.len(), 1);
            let listed = strings(&hinted[0]["refs"]);
            prop_assert_eq!(&listed[..], &cut[..cut.len().min(50)]);
            let omitted = hinted[0].get("refs_omitted").and_then(Value::as_u64).unwrap_or(0);
            prop_assert_eq!(omitted as usize, cut.len().saturating_sub(50));
        }
    }
}

const MARKER: &str = "ZQXJ";

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// § 8 test 3 and H1: with bodies, kinds and source ids drawn from a
    /// marker alphabet, no hint string, structured or text, holds a marker.
    #[test]
    fn hints_never_contain_record_content(
        lengths in proptest::collection::vec(0_usize..400, 0..60),
        limit in 1_usize..=60,
        max_tokens in 256_u32..20_000,
        degraded in any::<bool>(),
    ) {
        let ranked: Vec<SyntheticHit> = ranked(&lengths)
            .into_iter()
            .map(|hit| SyntheticHit {
                kind: format!("{MARKER}kind"),
                body: format!("{MARKER} {}", hit.body.replace("river", MARKER)),
                source_id: Some(format!("{MARKER}source")),
                ..hit
            })
            .collect();
        let degradation = if degraded {
            vec![("vector".to_string(), "embedder_unavailable".to_string())]
        } else {
            Vec::new()
        };
        let input = SearchPageInput {
            ranked,
            limit,
            snippet_bytes: 40,
            max_tokens,
            hybrid: degraded,
            degradation,
            hints: Some(true),
            ..Default::default()
        };
        let shaped = shape_search_page(&prepare_search_page(&input));
        for hint in hints(&shaped.structured) {
            prop_assert!(!hint.to_string().contains(MARKER), "{hint}");
        }
        for line in shaped.text.lines().filter(|line| line.starts_with("hint: ")) {
            prop_assert!(!line.contains(MARKER), "{line}");
        }
    }

    /// Every generated budget retains its independently sized leading prefix.
    /// Tiny budgets stop truthfully; complete traversal follows only when every
    /// nonterminal page independently admits at least one whole hit.
    #[test]
    fn hints_on_may_shrink_kept_but_offsets_stay_consistent(
        lengths in proptest::collection::vec(0_usize..400, 0..100),
        limit in 1_usize..=100,
        budgets in proptest::collection::vec(256_u32..6_000, 1..20),
        snippet_bytes in 40_usize..120,
    ) {
        let ranked=ranked(&lengths);
        let mut first_kept=Vec::new();
        for hints_on in [false,true] {
            let mut seen=Vec::new();let mut start=0;let mut pages=0;
            loop {
                let input=SearchPageInput {ranked:ranked.clone(),offset:start,limit,snippet_bytes,max_tokens:budgets[pages%budgets.len()],hints:Some(hints_on),..Default::default()};
                let (expected,text,next,kept)=reference_shape(&input).expect("short authored metadata fits accepted min budget");
                let shaped=shape_search_page(&prepare_search_page(&input));
                if pages==0 {first_kept.push(shaped.kept);}
                prop_assert_eq!(shaped.kept,kept);prop_assert_eq!(&shaped.structured,&expected);
                prop_assert_eq!(&shaped.text,&text);prop_assert_eq!(shaped.next_offset,next);
                prop_assert!((2*text.len()).div_ceil(4)<=input.max_tokens as usize);
                let results=expected["results"].as_array().unwrap();
                seen.extend(results.iter().map(|hit|hit["ref"].as_str().unwrap().to_owned()));
                let authored=ranked.iter().take(seen.len()).map(|hit|hit.reference.clone()).collect::<Vec<_>>();
                prop_assert_eq!(&seen,&authored,"every page preserves the authored ranked prefix");
                pages+=1;prop_assert!(pages<=200);
                if let Some(next)=next {prop_assert_eq!(next,start+kept);prop_assert!(kept>0);start=next;}
                else {
                    if seen.len()<ranked.len() {prop_assert_eq!(kept,0);prop_assert_eq!(&expected["completeness"],"cut_by_budget");prop_assert!(expected.get("hints").is_none());}
                    else {prop_assert_eq!(seen.len(),ranked.len());}
                    break;
                }
            }
        }
        prop_assert!(first_kept[1]<=first_kept[0]);
    }

    #[test]
    fn progress_capable_budget_traverses_all_authored_hits(
        lengths in proptest::collection::vec(0_usize..400,0..100),
        limit in 1_usize..=100,
        snippet_bytes in 40_usize..120,
    ) {
        let ranked=ranked(&lengths);let mut start=0;let mut seen=Vec::new();
        for _ in 0..101 {
            let input=SearchPageInput {ranked:ranked.clone(),offset:start,limit,snippet_bytes,max_tokens:20_000,hints:Some(true),..Default::default()};
            let (expected,text,next,kept)=reference_shape(&input).unwrap();
            if start<ranked.len() {prop_assert!(kept>0,"independent oracle establishes progress");}
            let shaped=shape_search_page(&prepare_search_page(&input));
            prop_assert_eq!(shaped.structured,expected);prop_assert_eq!(shaped.text,text);
            seen.extend(ranked.iter().skip(start).take(kept).map(|hit|hit.reference.clone()));
            if let Some(next)=next {start=next;} else {break;}
        }
        prop_assert_eq!(seen,ranked.iter().map(|hit|hit.reference.clone()).collect::<Vec<_>>());
    }

}

#[test]
fn authored_reference_omits_only_unavailable_envelope_signals() {
    let input = SearchPageInput {
        ranked: ranked(&[300]),
        limit: 10,
        snippet_bytes: 120,
        max_tokens: 4000,
        hints: Some(true),
        ..Default::default()
    };
    let (payload, text, _, kept) = reference_shape(&input).unwrap();
    assert_eq!(kept, 1);
    assert!(payload.get("state").is_none(), "unavailable envelope state must be omitted");
    assert!(payload.get("verdict").is_none(), "unavailable verdict must be omitted");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), payload);
    assert_eq!(payload["results"][0]["ref"], "l:note:000");
    assert_eq!(payload["results"][0]["score"], 1.0);
    assert_eq!(payload["results"][0]["source_id"], "fixture");
}

#[test]
fn reviewed_envelope_capacity_keeps_the_original_small_hit() {
    let input = SearchPageInput {
        ranked: ranked(&[120]),
        limit: 10,
        snippet_bytes: 120,
        max_tokens: 256,
        hints: Some(true),
        ..Default::default()
    };
    let (expected, text, next, kept) = reference_shape(&input).unwrap();
    assert_eq!(kept, 1, "omitting unavailable envelope signals admits this authored hit");
    assert_eq!(next, None);
    assert_eq!(expected["budget"]["cut"], false);
    assert_eq!(expected["completeness"], "complete");
    assert!(expected.get("hints").is_none());
    assert!((2 * text.len()).div_ceil(4) <= 256);
    let shaped = shape_search_page(&prepare_search_page(&input));
    assert_eq!(shaped.kept, kept);
    assert_eq!(shaped.structured, expected);
    assert_eq!(shaped.text, text);
    assert_eq!(shaped.next_offset, next);
}
