//! Inline typed properties, canonical JSON serialization, and closed predicate
//! validation/evaluation for Schema 38+ canonical nodes and edges.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::errors::EngineError;

/// Typed scalar value stored in an inline property map or predicate.
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    /// UTF-8 string scalar without NUL (`\0`) or ASCII RS (`\x1e`).
    String(String),
    /// Signed 64-bit integer scalar with full exact precision across `i64::MIN..=i64::MAX`.
    Int(i64),
    /// Finite IEEE-754 double-precision floating-point scalar (`-0.0` canonicalized to `0.0`).
    Float(f64),
    /// Boolean scalar.
    Bool(bool),
}

/// Ordered canonical property map keyed by property name (`BTreeMap` order).
pub type PropertyMap = BTreeMap<String, PropertyValue>;

/// Entity class targeted by a declared property projection (`"node"` or `"edge"`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PropertyEntityClass {
    /// Canonical node entity (`canonical_nodes`).
    #[default]
    Node,
    /// Canonical edge entity (`canonical_edges`).
    Edge,
}

impl PropertyEntityClass {
    /// Stable lowercase wire and SQL representation (`"node"` or `"edge"`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Edge => "edge",
        }
    }

    /// Parse a wire or SQL entity-class token (`"node"` or `"edge"`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "node" => Some(Self::Node),
            "edge" => Some(Self::Edge),
            _ => None,
        }
    }
}

/// Closed scalar comparison operator for [`PropertyPredicate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyComparisonOp {
    /// Equality (`"eq"`).
    Eq,
    /// Inequality (`"ne"`).
    Ne,
    /// Strictly less than (`"lt"`).
    Lt,
    /// Less than or equal (`"lte"`).
    Lte,
    /// Strictly greater than (`"gt"`).
    Gt,
    /// Greater than or equal (`"gte"`).
    Gte,
}

impl PropertyComparisonOp {
    /// Stable lowercase wire representation (`"eq"`, `"ne"`, `"lt"`, `"lte"`, `"gt"`, `"gte"`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Gt => "gt",
            Self::Gte => "gte",
        }
    }

    /// Parse a wire comparison-operator token.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "eq" => Some(Self::Eq),
            "ne" => Some(Self::Ne),
            "lt" => Some(Self::Lt),
            "lte" => Some(Self::Lte),
            "gt" => Some(Self::Gt),
            "gte" => Some(Self::Gte),
            _ => None,
        }
    }
}

/// Closed typed property predicate evaluated over inline or indexed properties.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyPredicate {
    /// Property key name to evaluate.
    pub key: String,
    /// Comparison operator to apply.
    pub op: PropertyComparisonOp,
    /// Scalar reference value to compare against.
    pub value: PropertyValue,
}

/// Registered operational-collection schema and retention metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationalCollectionInfo {
    /// Registered collection name.
    pub name: String,
    /// Storage kind (`"append_only_log"` or `"latest_state"`).
    pub kind: String,
    /// JSON Schema definition text.
    pub schema_json: String,
    /// JSON retention policy text.
    pub retention_json: String,
    /// Schema format version recorded for the collection.
    pub format_version: i64,
    /// Unix creation timestamp recorded in `operational_collections`.
    pub created_at: i64,
}

fn is_valid_property_key(key: &str) -> bool {
    !key.is_empty() && !key.contains(['\0', '\x1e'])
}

fn is_valid_property_string(value: &str) -> bool {
    !value.contains(['\0', '\x1e'])
}

pub(crate) fn normalize_finite_f64(value: f64) -> Option<f64> {
    if !value.is_finite() {
        None
    } else if value == 0.0 {
        Some(0.0)
    } else {
        Some(value)
    }
}

pub(crate) fn property_value_to_json(value: &PropertyValue) -> Option<serde_json::Value> {
    match value {
        PropertyValue::String(s) => Some(serde_json::Value::String(s.clone())),
        PropertyValue::Int(i) => Some(serde_json::Value::Number(serde_json::Number::from(*i))),
        PropertyValue::Float(f) => {
            let norm = normalize_finite_f64(*f)?;
            serde_json::Number::from_f64(norm).map(serde_json::Value::Number)
        }
        PropertyValue::Bool(b) => Some(serde_json::Value::Bool(*b)),
    }
}

pub(crate) fn json_to_property_value(value: &serde_json::Value) -> Option<PropertyValue> {
    match value {
        serde_json::Value::String(s) => {
            if is_valid_property_string(s) {
                Some(PropertyValue::String(s.clone()))
            } else {
                None
            }
        }
        serde_json::Value::Bool(b) => Some(PropertyValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(PropertyValue::Int(i))
            } else {
                n.as_f64().and_then(normalize_finite_f64).map(PropertyValue::Float)
            }
        }
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            None
        }
    }
}

pub(crate) fn properties_to_json_value(
    map: &PropertyMap,
) -> Result<serde_json::Value, EngineError> {
    validate_property_map(map)?;
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        let json_val = property_value_to_json(v).ok_or(EngineError::WriteValidation)?;
        obj.insert(k.clone(), json_val);
    }
    Ok(serde_json::Value::Object(obj))
}

pub(crate) fn decode_properties_value(
    value: &serde_json::Value,
) -> Result<Option<PropertyMap>, EngineError> {
    let obj = value.as_object().ok_or(EngineError::Storage)?;
    let mut map = PropertyMap::new();
    for (k, v) in obj {
        if !is_valid_property_key(k) {
            return Err(EngineError::Storage);
        }
        let prop_val = json_to_property_value(v).ok_or(EngineError::Storage)?;
        map.insert(k.clone(), prop_val);
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(map))
    }
}

/// Validate every key and value in a [`PropertyMap`] before storage.
pub(crate) fn validate_property_map(map: &PropertyMap) -> Result<(), EngineError> {
    for (key, value) in map {
        if !is_valid_property_key(key) {
            return Err(EngineError::WriteValidation);
        }
        match value {
            PropertyValue::String(s) if !is_valid_property_string(s) => {
                return Err(EngineError::WriteValidation);
            }
            PropertyValue::Float(f) if !f.is_finite() => {
                return Err(EngineError::WriteValidation);
            }
            PropertyValue::String(_)
            | PropertyValue::Int(_)
            | PropertyValue::Float(_)
            | PropertyValue::Bool(_) => {}
        }
    }
    Ok(())
}

/// Validate and serialize an optional [`PropertyMap`] to compact canonical JSON.
pub(crate) fn encode_properties_json(
    props: Option<&PropertyMap>,
) -> Result<Option<String>, EngineError> {
    let Some(map) = props else {
        return Ok(None);
    };
    if map.is_empty() {
        return Ok(None);
    }
    let json_val = properties_to_json_value(map)?;
    serde_json::to_string(&json_val).map(Some).map_err(|_| EngineError::WriteValidation)
}

/// Decode a stored canonical JSON property object into `Option<PropertyMap>`.
///
/// Normalizes `{}` to `None` so persisted/hydrated `Option<PropertyMap>` never
/// carries an empty map.
pub(crate) fn decode_properties_json(
    raw: Option<&str>,
) -> Result<Option<PropertyMap>, EngineError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let parsed: serde_json::Value = serde_json::from_str(raw).map_err(|_| EngineError::Storage)?;
    decode_properties_value(&parsed)
}

/// Decode a nullable TEXT SQL column containing canonical JSON properties.
pub(crate) fn decode_properties_row_col(
    row: &rusqlite::Row<'_>,
    col_idx: usize,
) -> rusqlite::Result<Option<PropertyMap>> {
    let raw: Option<String> = row.get(col_idx)?;
    decode_properties_json(raw.as_deref()).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            col_idx,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid stored properties JSON",
            )),
        )
    })
}

/// Validate a [`PropertyPredicate`] for query-time evaluation.
pub(crate) fn validate_property_predicate(pred: &PropertyPredicate) -> Result<(), EngineError> {
    if !is_valid_property_key(&pred.key) {
        return Err(EngineError::InvalidFilter {
            reason: "property predicate key must be non-empty and must not contain NUL or \\x1e"
                .to_string(),
        });
    }
    match &pred.value {
        PropertyValue::String(s) if !is_valid_property_string(s) => {
            return Err(EngineError::InvalidFilter {
                reason: "property predicate string value must not contain NUL or \\x1e".to_string(),
            });
        }
        PropertyValue::Float(f) if !f.is_finite() => {
            return Err(EngineError::InvalidFilter {
                reason: "property predicate float value must be finite".to_string(),
            });
        }
        PropertyValue::Bool(_)
            if matches!(
                pred.op,
                PropertyComparisonOp::Lt
                    | PropertyComparisonOp::Lte
                    | PropertyComparisonOp::Gt
                    | PropertyComparisonOp::Gte
            ) =>
        {
            return Err(EngineError::InvalidFilter {
                reason: "ordering operators (lt, lte, gt, gte) are invalid for boolean properties"
                    .to_string(),
            });
        }
        PropertyValue::String(_)
        | PropertyValue::Int(_)
        | PropertyValue::Float(_)
        | PropertyValue::Bool(_) => {}
    }
    Ok(())
}

fn apply_ordering(ord: Ordering, op: PropertyComparisonOp) -> bool {
    match op {
        PropertyComparisonOp::Eq => ord == Ordering::Equal,
        PropertyComparisonOp::Ne => ord != Ordering::Equal,
        PropertyComparisonOp::Lt => ord == Ordering::Less,
        PropertyComparisonOp::Lte => ord != Ordering::Greater,
        PropertyComparisonOp::Gt => ord == Ordering::Greater,
        PropertyComparisonOp::Gte => ord != Ordering::Less,
    }
}

fn cmp_i64_f64(i: i64, f: f64) -> Option<Ordering> {
    if !f.is_finite() {
        return None;
    }
    // 2^63 is exactly representable in IEEE-754 binary64; `i64::MAX = 2^63 - 1`
    // is strictly less than `2^63`, and `i64::MIN = -2^63` equals `-2^63`.
    const TWO_POW_63_F64: f64 = 9_223_372_036_854_775_808.0;
    if f >= TWO_POW_63_F64 {
        return Some(Ordering::Less);
    }
    if f < -TWO_POW_63_F64 {
        return Some(Ordering::Greater);
    }
    if f == -TWO_POW_63_F64 {
        return Some(i.cmp(&i64::MIN));
    }
    // Inside `(-2^63, 2^63)`, `f.trunc()` is an exact integer in `i64`.
    let trunc = f.trunc();
    let t = trunc as i64;
    if f == trunc {
        Some(i.cmp(&t))
    } else if f > trunc {
        Some(if i <= t { Ordering::Less } else { Ordering::Greater })
    } else {
        Some(if i < t { Ordering::Less } else { Ordering::Greater })
    }
}

/// Evaluate a [`PropertyPredicate`] against an optional resolved [`PropertyValue`].
#[must_use]
pub(crate) fn eval_property_value(
    lhs: Option<&PropertyValue>,
    op: PropertyComparisonOp,
    rhs: &PropertyValue,
) -> bool {
    let Some(lhs) = lhs else {
        return false;
    };
    match (lhs, rhs) {
        (PropertyValue::String(a), PropertyValue::String(b)) => apply_ordering(a.cmp(b), op),
        (PropertyValue::Bool(a), PropertyValue::Bool(b)) => match op {
            PropertyComparisonOp::Eq => a == b,
            PropertyComparisonOp::Ne => a != b,
            PropertyComparisonOp::Lt
            | PropertyComparisonOp::Lte
            | PropertyComparisonOp::Gt
            | PropertyComparisonOp::Gte => false,
        },
        (PropertyValue::Int(a), PropertyValue::Int(b)) => apply_ordering(a.cmp(b), op),
        (PropertyValue::Float(a), PropertyValue::Float(b)) => {
            if !a.is_finite() || !b.is_finite() {
                false
            } else {
                a.partial_cmp(b).is_some_and(|ord| apply_ordering(ord, op))
            }
        }
        (PropertyValue::Int(i), PropertyValue::Float(f)) => {
            cmp_i64_f64(*i, *f).is_some_and(|ord| apply_ordering(ord, op))
        }
        (PropertyValue::Float(f), PropertyValue::Int(i)) => {
            cmp_i64_f64(*i, *f).map(Ordering::reverse).is_some_and(|ord| apply_ordering(ord, op))
        }
        _ => false,
    }
}

/// Evaluate a [`PropertyPredicate`] against an optional [`PropertyMap`].
#[cfg(test)]
#[must_use]
pub(crate) fn eval_property_predicate(
    props: Option<&PropertyMap>,
    pred: &PropertyPredicate,
) -> bool {
    eval_property_value(props.and_then(|map| map.get(&pred.key)), pred.op, &pred.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn valid_key_strategy() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            any::<char>().prop_filter("no NUL or RS", |c| *c != '\0' && *c != '\x1e'),
            1..=24,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn valid_string_strategy() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            any::<char>().prop_filter("no NUL or RS", |c| *c != '\0' && *c != '\x1e'),
            0..=48,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn valid_finite_f64_strategy() -> impl Strategy<Value = f64> {
        prop_oneof![
            Just(0.0_f64),
            Just(-0.0_f64),
            Just(1.0_f64),
            Just(-1.0_f64),
            Just(5.5_f64),
            Just(-5.5_f64),
            Just((1_i64 << 53) as f64),
            Just(-((1_i64 << 53) as f64)),
            Just(f64::MIN_POSITIVE),
            Just(-f64::MIN_POSITIVE),
            any::<f64>().prop_filter("finite float", |f| f.is_finite()),
        ]
    }

    fn valid_i64_strategy() -> impl Strategy<Value = i64> {
        prop_oneof![
            Just(0_i64),
            Just(1_i64),
            Just(-1_i64),
            Just(i64::MIN),
            Just(i64::MAX),
            Just((1_i64 << 53) + 1),
            Just(-(1_i64 << 53) - 1),
            Just(1_i64 << 53),
            Just(-(1_i64 << 53)),
            any::<i64>(),
        ]
    }

    fn valid_value_strategy() -> impl Strategy<Value = PropertyValue> {
        prop_oneof![
            valid_string_strategy().prop_map(PropertyValue::String),
            valid_i64_strategy().prop_map(PropertyValue::Int),
            valid_finite_f64_strategy().prop_map(PropertyValue::Float),
            any::<bool>().prop_map(PropertyValue::Bool),
        ]
    }

    fn valid_property_map_strategy() -> impl Strategy<Value = PropertyMap> {
        proptest::collection::btree_map(valid_key_strategy(), valid_value_strategy(), 0..=12)
    }

    #[test]
    fn entity_class_and_comparison_op_round_trip() {
        assert_eq!(PropertyEntityClass::default(), PropertyEntityClass::Node);
        for (variant, token) in
            [(PropertyEntityClass::Node, "node"), (PropertyEntityClass::Edge, "edge")]
        {
            assert_eq!(variant.as_str(), token);
            assert_eq!(PropertyEntityClass::parse(token), Some(variant));
        }
        assert_eq!(PropertyEntityClass::parse("NODE"), None);
        assert_eq!(PropertyEntityClass::parse(""), None);

        for (variant, token) in [
            (PropertyComparisonOp::Eq, "eq"),
            (PropertyComparisonOp::Ne, "ne"),
            (PropertyComparisonOp::Lt, "lt"),
            (PropertyComparisonOp::Lte, "lte"),
            (PropertyComparisonOp::Gt, "gt"),
            (PropertyComparisonOp::Gte, "gte"),
        ] {
            assert_eq!(variant.as_str(), token);
            assert_eq!(PropertyComparisonOp::parse(token), Some(variant));
        }
        assert_eq!(PropertyComparisonOp::parse("EQ"), None);
        assert_eq!(PropertyComparisonOp::parse("!="), None);
    }

    #[test]
    fn validate_property_map_rejects_invalid_keys_and_values() {
        let mut valid = PropertyMap::new();
        valid.insert("k_str".into(), PropertyValue::String("a\"b\\c".into()));
        valid.insert("k_int_min".into(), PropertyValue::Int(i64::MIN));
        valid.insert("k_int_max".into(), PropertyValue::Int(i64::MAX));
        valid.insert("k_neg_zero".into(), PropertyValue::Float(-0.0));
        valid.insert("k_bool".into(), PropertyValue::Bool(true));
        assert_eq!(validate_property_map(&valid), Ok(()));

        for (bad_key, bad_val) in [
            ("", PropertyValue::Int(1)),
            ("bad\0key", PropertyValue::Int(1)),
            ("bad\x1ekey", PropertyValue::Int(1)),
            ("ok", PropertyValue::String("bad\0val".into())),
            ("ok", PropertyValue::String("bad\x1eval".into())),
            ("ok", PropertyValue::Float(f64::NAN)),
            ("ok", PropertyValue::Float(f64::INFINITY)),
            ("ok", PropertyValue::Float(f64::NEG_INFINITY)),
        ] {
            let mut m = PropertyMap::new();
            m.insert(bad_key.into(), bad_val);
            assert_eq!(
                validate_property_map(&m),
                Err(EngineError::WriteValidation),
                "expected WriteValidation for key={bad_key:?}"
            );
        }
    }

    #[test]
    fn encode_and_decode_properties_json_edge_cases() {
        assert_eq!(encode_properties_json(None), Ok(None));
        assert_eq!(encode_properties_json(Some(&PropertyMap::new())), Ok(None));
        assert_eq!(decode_properties_json(None), Ok(None));
        assert_eq!(decode_properties_json(Some("{}")), Ok(None));

        let mut props = PropertyMap::new();
        props.insert("z_str".into(), PropertyValue::String("hello".into()));
        props.insert("a_neg_zero".into(), PropertyValue::Float(-0.0));
        props.insert("b_big_pos".into(), PropertyValue::Int((1_i64 << 53) + 1));
        props.insert("c_big_neg".into(), PropertyValue::Int(-(1_i64 << 53) - 1));
        props.insert("d_min".into(), PropertyValue::Int(i64::MIN));
        props.insert("e_max".into(), PropertyValue::Int(i64::MAX));
        props.insert("f_bool".into(), PropertyValue::Bool(false));

        let encoded = encode_properties_json(Some(&props)).unwrap().unwrap();
        assert!(!encoded.contains("-0.0"), "must canonicalize -0.0: {encoded}");
        assert_eq!(
            encoded,
            r#"{"a_neg_zero":0.0,"b_big_pos":9007199254740993,"c_big_neg":-9007199254740993,"d_min":-9223372036854775808,"e_max":9223372036854775807,"f_bool":false,"z_str":"hello"}"#
        );

        let decoded = decode_properties_json(Some(&encoded)).unwrap().unwrap();
        assert_eq!(decoded, props);
        match decoded.get("a_neg_zero") {
            Some(PropertyValue::Float(f)) => {
                assert_eq!(f.to_bits(), 0.0_f64.to_bits());
            }
            other => panic!("expected Float(0.0), got {other:?}"),
        }

        for raw in [
            "",
            "{bad",
            "null",
            "[]",
            "123",
            "true",
            "\"str\"",
            r#"{"a":null}"#,
            r#"{"a":[1]}"#,
            r#"{"a":{"b":1}}"#,
            r#"{"":1}"#,
            "{\"a\\u0000b\":1}",
            "{\"a\\u001eb\":1}",
            "{\"a\":\"x\\u0000y\"}",
            "{\"a\":\"x\\u001ey\"}",
            r#"{"a":1e500}"#,
        ] {
            assert_eq!(
                decode_properties_json(Some(raw)),
                Err(EngineError::Storage),
                "expected Storage error for {raw:?}"
            );
        }

        let decoded_neg_zero = decode_properties_json(Some(r#"{"z":-0.0}"#)).unwrap().unwrap();
        match decoded_neg_zero.get("z") {
            Some(PropertyValue::Float(f)) => assert_eq!(f.to_bits(), 0.0_f64.to_bits()),
            other => panic!("expected canonicalized +0.0, got {other:?}"),
        }
    }

    #[test]
    fn validate_property_predicate_rules() {
        for bad in [
            PropertyPredicate {
                key: "".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Int(1),
            },
            PropertyPredicate {
                key: "k\0".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Int(1),
            },
            PropertyPredicate {
                key: "k\x1e".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Int(1),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::String("v\0".into()),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::String("v\x1e".into()),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Float(f64::NAN),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Float(f64::INFINITY),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Lt,
                value: PropertyValue::Bool(true),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Lte,
                value: PropertyValue::Bool(false),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Gt,
                value: PropertyValue::Bool(true),
            },
            PropertyPredicate {
                key: "k".into(),
                op: PropertyComparisonOp::Gte,
                value: PropertyValue::Bool(false),
            },
        ] {
            assert!(
                matches!(validate_property_predicate(&bad), Err(EngineError::InvalidFilter { .. })),
                "expected InvalidFilter for {bad:?}"
            );
        }

        for op in [PropertyComparisonOp::Eq, PropertyComparisonOp::Ne] {
            assert_eq!(
                validate_property_predicate(&PropertyPredicate {
                    key: "flag".into(),
                    op,
                    value: PropertyValue::Bool(true),
                }),
                Ok(())
            );
        }
    }

    #[test]
    fn eval_property_predicate_exact_semantics() {
        let all_ops = [
            PropertyComparisonOp::Eq,
            PropertyComparisonOp::Ne,
            PropertyComparisonOp::Lt,
            PropertyComparisonOp::Lte,
            PropertyComparisonOp::Gt,
            PropertyComparisonOp::Gte,
        ];

        // Absent map or absent key -> false for all 6 ops.
        let mut props = PropertyMap::new();
        props.insert("present".into(), PropertyValue::Int(10));
        for op in all_ops {
            let pred =
                PropertyPredicate { key: "missing".into(), op, value: PropertyValue::Int(10) };
            assert!(!eval_property_predicate(None, &pred));
            assert!(!eval_property_predicate(Some(&props), &pred));
        }

        // Cross-family mismatch -> false for all 6 ops.
        props.insert("s".into(), PropertyValue::String("10".into()));
        props.insert("b".into(), PropertyValue::Bool(true));
        for (key, val) in [
            ("s", PropertyValue::Int(10)),
            ("s", PropertyValue::Float(10.0)),
            ("s", PropertyValue::Bool(true)),
            ("present", PropertyValue::String("10".into())),
            ("present", PropertyValue::Bool(true)),
            ("b", PropertyValue::Int(1)),
            ("b", PropertyValue::Float(1.0)),
            ("b", PropertyValue::String("true".into())),
        ] {
            for op in all_ops {
                let pred = PropertyPredicate { key: key.into(), op, value: val.clone() };
                assert!(
                    !eval_property_predicate(Some(&props), &pred),
                    "cross-family must be false: {pred:?}"
                );
            }
        }

        // Bool Eq / Ne only.
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "b".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Bool(true),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "b".into(),
                op: PropertyComparisonOp::Ne,
                value: PropertyValue::Bool(false),
            }
        ));
        for op in [
            PropertyComparisonOp::Lt,
            PropertyComparisonOp::Lte,
            PropertyComparisonOp::Gt,
            PropertyComparisonOp::Gte,
        ] {
            assert!(!eval_property_predicate(
                Some(&props),
                &PropertyPredicate { key: "b".into(), op, value: PropertyValue::Bool(true) }
            ));
        }

        // Int vs Float exact numeric semantics without 2^53 truncation.
        let two_53 = 1_i64 << 53;
        props.insert("big".into(), PropertyValue::Int(two_53 + 1));
        props.insert("five".into(), PropertyValue::Int(5));
        props.insert("five_f".into(), PropertyValue::Float(5.0));
        props.insert("zero_f".into(), PropertyValue::Float(-0.0));
        props.insert("max_i".into(), PropertyValue::Int(i64::MAX));
        props.insert("min_i".into(), PropertyValue::Int(i64::MIN));

        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "big".into(),
                op: PropertyComparisonOp::Gt,
                value: PropertyValue::Float(two_53 as f64),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "big".into(),
                op: PropertyComparisonOp::Ne,
                value: PropertyValue::Float(two_53 as f64),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "five".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Float(5.0),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "five_f".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Int(5),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "five".into(),
                op: PropertyComparisonOp::Lt,
                value: PropertyValue::Float(5.1),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "zero_f".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Float(0.0),
            }
        ));
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "zero_f".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Int(0),
            }
        ));
        // `i64::MAX as f64` rounds up to 2^63 > i64::MAX.
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "max_i".into(),
                op: PropertyComparisonOp::Lt,
                value: PropertyValue::Float(i64::MAX as f64),
            }
        ));
        // `i64::MIN` is exactly `-2^63`.
        assert!(eval_property_predicate(
            Some(&props),
            &PropertyPredicate {
                key: "min_i".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Float(i64::MIN as f64),
            }
        ));
    }

    proptest! {
        #[test]
        fn proptest_property_map_json_roundtrip(map in valid_property_map_strategy()) {
            prop_assert_eq!(validate_property_map(&map), Ok(()));
            let encoded = encode_properties_json(Some(&map)).unwrap();
            if map.is_empty() {
                prop_assert_eq!(encoded, None);
            } else {
                let json = encoded.unwrap();
                let decoded = decode_properties_json(Some(&json)).unwrap().unwrap();
                for value in decoded.values() {
                    if let PropertyValue::Float(f) = value {
                        prop_assert_ne!(f.to_bits(), (-0.0_f64).to_bits());
                    }
                }
                prop_assert_eq!(&decoded, &map);
                let reencoded = encode_properties_json(Some(&decoded)).unwrap().unwrap();
                prop_assert_eq!(reencoded, json);
            }
        }

        #[test]
        fn proptest_numeric_comparison_invariants(
            i in valid_i64_strategy(),
            f in valid_finite_f64_strategy(),
        ) {
            let mut props_i = PropertyMap::new();
            props_i.insert("k".into(), PropertyValue::Int(i));
            let mut props_f = PropertyMap::new();
            props_f.insert("k".into(), PropertyValue::Float(f));

            let eval_if = |op| eval_property_predicate(
                Some(&props_i),
                &PropertyPredicate { key: "k".into(), op, value: PropertyValue::Float(f) },
            );
            let eval_fi = |op| eval_property_predicate(
                Some(&props_f),
                &PropertyPredicate { key: "k".into(), op, value: PropertyValue::Int(i) },
            );

            let eq = eval_if(PropertyComparisonOp::Eq);
            let lt = eval_if(PropertyComparisonOp::Lt);
            let gt = eval_if(PropertyComparisonOp::Gt);

            // Trichotomy: exactly one of Eq, Lt, Gt is true.
            prop_assert_eq!(u8::from(eq) + u8::from(lt) + u8::from(gt), 1);
            prop_assert_eq!(eval_if(PropertyComparisonOp::Ne), !eq);
            prop_assert_eq!(eval_if(PropertyComparisonOp::Lte), eq || lt);
            prop_assert_eq!(eval_if(PropertyComparisonOp::Gte), eq || gt);

            // Antisymmetry across Int <-> Float.
            prop_assert_eq!(eq, eval_fi(PropertyComparisonOp::Eq));
            prop_assert_eq!(lt, eval_fi(PropertyComparisonOp::Gt));
            prop_assert_eq!(gt, eval_fi(PropertyComparisonOp::Lt));
            prop_assert_eq!(eval_if(PropertyComparisonOp::Lte), eval_fi(PropertyComparisonOp::Gte));
            prop_assert_eq!(eval_if(PropertyComparisonOp::Gte), eval_fi(PropertyComparisonOp::Lte));

            // Exact cross-check when f is in (-2^63, 2^63).
            if f > -9_223_372_036_854_775_808.0 && f < 9_223_372_036_854_775_808.0 {
                let t = f.trunc() as i64;
                if f.fract() == 0.0 {
                    prop_assert_eq!(eq, i == t);
                    prop_assert_eq!(lt, i < t);
                    prop_assert_eq!(gt, i > t);
                } else {
                    prop_assert!(!eq);
                    if f > f.trunc() {
                        prop_assert_eq!(lt, i <= t);
                        prop_assert_eq!(gt, i > t);
                    } else {
                        prop_assert_eq!(lt, i < t);
                        prop_assert_eq!(gt, i >= t);
                    }
                }
            }
        }

        #[test]
        fn proptest_control_delimiters_rejected(
            prefix in "[a-z]{0,8}",
            suffix in "[a-z]{0,8}",
            sep in prop_oneof![Just('\0'), Just('\x1e')],
            in_key in any::<bool>(),
        ) {
            let bad = format!("{prefix}{sep}{suffix}");
            let mut map = PropertyMap::new();
            let pred = if in_key {
                map.insert(bad.clone(), PropertyValue::Int(1));
                PropertyPredicate { key: bad, op: PropertyComparisonOp::Eq, value: PropertyValue::Int(1) }
            } else {
                map.insert("valid".into(), PropertyValue::String(bad.clone()));
                PropertyPredicate { key: "valid".into(), op: PropertyComparisonOp::Eq, value: PropertyValue::String(bad) }
            };
            prop_assert_eq!(validate_property_map(&map), Err(EngineError::WriteValidation));
            let pred_res = validate_property_predicate(&pred);
            prop_assert!(
                matches!(pred_res, Err(EngineError::InvalidFilter { .. })),
                "expected InvalidFilter, got {:?}",
                pred_res
            );
        }
    }
}
