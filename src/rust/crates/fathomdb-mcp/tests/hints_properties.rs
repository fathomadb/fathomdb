//! Hints sidecar properties over synthetic pages, through the shaping seam
//! (`fathomdb_mcp::seams`, review fix 1): ref subsets, content freedom, and
//! kept-count consistency (hints-sidecar § 8 tests 2, 3, 10).

use fathomdb_mcp::seams::{prepare_search_page, shape_search_page, SearchPageInput, SyntheticHit};
use proptest::prelude::*;
use serde_json::Value;

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
        snippet_chars in 40_usize..120,
    ) {
        let input = SearchPageInput {
            ranked: ranked(&lengths),
            limit,
            snippet_chars,
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
        if cut.is_empty() {
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
            snippet_chars: 40,
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

    /// § 8 test 10 and § 5.1: over the same ranked list and budgets, hints on
    /// keep no more hits than hints off; in each mode the continuation chain
    /// returns every hit once, in order; and each continuation resumes at
    /// `offset + kept` of its own response.
    #[test]
    fn hints_on_may_shrink_kept_but_offsets_stay_consistent(
        lengths in proptest::collection::vec(0_usize..400, 0..100),
        limit in 1_usize..=100,
        budgets in proptest::collection::vec(256_u32..6_000, 1..20),
        snippet_chars in 40_usize..120,
    ) {
        let ranked = ranked(&lengths);
        let expected: Vec<String> = ranked.iter().map(|hit| hit.reference.clone()).collect();
        let mut first_kept = Vec::new();
        for hints_on in [false, true] {
            let mut seen = Vec::new();
            let mut offset = Some(0_usize);
            let mut pages = 0;
            while let Some(start) = offset {
                let input = SearchPageInput {
                    ranked: ranked.clone(),
                    offset: start,
                    limit,
                    snippet_chars,
                    max_tokens: budgets[pages % budgets.len()],
                    hints: Some(hints_on),
                    ..Default::default()
                };
                let shaped = shape_search_page(&prepare_search_page(&input));
                if pages == 0 {
                    first_kept.push(shaped.kept);
                }
                let results = shaped.structured["results"].as_array().unwrap();
                prop_assert_eq!(results.len(), shaped.kept);
                seen.extend(results.iter().map(|hit| hit["ref"].as_str().unwrap().to_string()));
                if let Some(next) = shaped.next_offset {
                    prop_assert_eq!(next, start + shaped.kept);
                }
                offset = shaped.next_offset;
                pages += 1;
                prop_assert!(pages <= 200, "paging must terminate");
            }
            prop_assert_eq!(&seen, &expected, "hints {}", hints_on);
        }
        prop_assert!(first_kept[1] <= first_kept[0], "kept_on {} > kept_off {}", first_kept[1], first_kept[0]);
    }
}
