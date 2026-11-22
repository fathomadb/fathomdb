use crate::{
    fixtures::{database, node},
    policy::Policy,
    session::Session,
    storage::{Class, Filter, Reader},
};
use fathomdb_engine::{EmbedderChoice, Engine};
#[test]
fn lazy_tree_preserves_original_scalar_lexemes_and_nested_ranges() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let body = r#"{"integer":9007199254740993123456789,"nested":{"null":null,"empty":"","html":"<script>throw 1</script>"},"array":[true,false,1.00e+20]}"#;
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened.engine.write(&[node("structured", body)]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r
        .page(
            Class::Node,
            0,
            200,
            Filter { identity: Some("structured".into()), ..Filter::default() },
        )
        .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let tree = r.tree(handle, 0, None).unwrap();
    assert_eq!(tree["kind"], "object");
    let items = tree["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["key"], "integer");
    assert_eq!(items[0]["lexeme"], "9007199254740993123456789");
    assert_eq!(items[1]["kind"], "object");
    let start = items[1]["start"].as_str().unwrap().parse().unwrap();
    let nested = r.tree(handle, start, None).unwrap();
    assert_eq!(nested["items"][0]["kind"], "null");
    assert_eq!(nested["items"][1]["lexeme"], r#""""#);
    assert_eq!(nested["items"][2]["lexeme"], r#""<script>throw 1</script>""#);
    let array = r.tree(handle, items[2]["start"].as_str().unwrap().parse().unwrap(), None).unwrap();
    assert_eq!(array["items"][2]["lexeme"], "1.00e+20");
    r.session.close().unwrap();
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(40))]
    #[test]
    fn structured_ranges_round_trip_unicode_and_reject_trailing_non_json(value in ".{0,300}", spaces in 1_usize..8) {
        let encoded=serde_json::to_string(&value).unwrap();
        let body=format!("{}[{},9007199254740993123456789,1.00e+20]", " ".repeat(spaces),encoded);
        let read=|offset:usize,limit:usize| Ok(body.as_bytes()[offset..body.len().min(offset+limit)].to_vec());
        let tree=crate::tree::children(read,body.len(),0,None).unwrap();
        for item in tree["items"].as_array().unwrap() {
            let start=item["start"].as_str().unwrap().parse::<usize>().unwrap();let end=item["end"].as_str().unwrap().parse::<usize>().unwrap();
            proptest::prop_assert_eq!(item["lexeme"].as_str().unwrap(),&body[start..end]);
        }
        let first=&tree["items"][0];let decoded:String=serde_json::from_str(first["lexeme"].as_str().unwrap()).unwrap();proptest::prop_assert_eq!(decoded,value);
        let bad=format!("{body}x");
        let rejected=crate::tree::children(|offset,limit|Ok(bad.as_bytes()[offset..bad.len().min(offset+limit)].to_vec()),bad.len(),0,None);
        proptest::prop_assert!(rejected.is_err(),"non-JSON suffix after a whitespace-prefixed root must refuse");
    }
}

#[test]
fn tree_pages_bound_serialized_bytes_and_resume_every_exact_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let expected =
        (0..200).map(|n| format!("field-{n:03}-{}", "k".repeat(3990))).collect::<Vec<_>>();
    let body: serde_json::Map<String, serde_json::Value> = expected
        .iter()
        .map(|name| (name.clone(), serde_json::Value::String("\\".repeat(4000))))
        .collect();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[node("wide-tree", &serde_json::Value::Object(body).to_string())])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r
        .page(
            Class::Node,
            0,
            10,
            Filter { identity: Some("wide-tree".into()), ..Filter::default() },
        )
        .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let mut after = None;
    let mut names = vec![];
    let mut pages = 0;
    loop {
        let tree = r.tree(handle, 0, after).unwrap();
        pages += 1;
        assert!(
            serde_json::to_vec(&tree).unwrap().len() <= 2 * 1024 * 1024,
            "serialized keys plus escaped scalar previews share one response cap"
        );
        assert!(tree["items"].as_array().unwrap().len() <= 200);
        for item in tree["items"].as_array().unwrap() {
            names.push(item["key"].as_str().unwrap().to_string());
        }
        match tree["continuation"].as_str() {
            Some(n) => {
                assert_eq!(tree["status"], "partial");
                after = Some(n.parse().unwrap());
            }
            None => break,
        }
    }
    assert!(pages >= 2);
    assert_eq!(names, expected);
    r.session.close().unwrap();
}
