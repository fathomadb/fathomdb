use super::*;
use crate::{ReadView, SearchFilter};
use serde::Serialize;

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IdWire<'a> {
    space: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuerySeedWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    text: &'a str,
    ranked_limit: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExplicitSeedWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    logical_ids: Vec<IdWire<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SeedWire<'a> {
    Query(QuerySeedWire<'a>),
    Explicit(ExplicitSeedWire<'a>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ViewWire {
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    valid_as_of: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FilterWire<'a> {
    source_type: Option<&'a str>,
    kind: Option<&'a str>,
    created_after: Option<i64>,
    status: Option<&'a str>,
    attributes: Vec<(&'a str, &'a str)>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadContextWire<'a> {
    schema_version: u32,
    view: ViewWire,
    eligibility: FilterWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrozenContextWire<'a> {
    schema_version: u32,
    effective_valid_at: i64,
    context: ReadContextWire<'a>,
    token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentContextWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    context: ReadContextWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrozenGraphContextWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    context: FrozenContextWire<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GraphContextWire<'a> {
    Current(CurrentContextWire<'a>),
    Frozen(FrozenGraphContextWire<'a>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestWire<'a> {
    schema_version: u32,
    seed: SeedWire<'a>,
    direction: &'static str,
    edge_kinds: &'a [String],
    target_kinds: &'a [String],
    context: GraphContextWire<'a>,
    max_depth: u32,
    result_limit: u32,
    max_work_units: String,
    include_explanation: bool,
    #[serde(skip_serializing_if = "is_false")]
    include_evidence: bool,
}

pub(super) fn direction_str(value: TraversalDirection) -> &'static str {
    match value {
        TraversalDirection::Incoming => "incoming",
        TraversalDirection::Outgoing => "outgoing",
        TraversalDirection::Both => "both",
    }
}

fn read_context_wire(value: &ReadContextV1) -> ReadContextWire<'_> {
    ReadContextWire {
        schema_version: value.schema_version,
        view: ViewWire {
            include_superseded: value.view.include_superseded,
            include_inactive: value.view.include_inactive,
            include_out_of_window: value.view.include_out_of_window,
            valid_as_of: value.view.valid_as_of,
        },
        eligibility: FilterWire {
            source_type: value.eligibility.source_type.as_deref(),
            kind: value.eligibility.kind.as_deref(),
            created_after: value.eligibility.created_after,
            status: value.eligibility.status.as_deref(),
            attributes: value
                .eligibility
                .attributes
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect(),
        },
    }
}

/// Encode a graph-expansion request into canonical declaration-order JSON.
pub fn encode_graph_expand_request_v1(
    value: &GraphExpandRequestV1,
) -> Result<Vec<u8>, GraphExpansionErrorV1> {
    let seed = match &value.seed {
        GraphSeedV1::Query { schema_version, text, ranked_limit } => {
            SeedWire::Query(QuerySeedWire {
                schema_version: *schema_version,
                r#type: "query",
                text,
                ranked_limit: *ranked_limit,
            })
        }
        GraphSeedV1::Explicit { schema_version, logical_ids } => {
            SeedWire::Explicit(ExplicitSeedWire {
                schema_version: *schema_version,
                r#type: "explicit",
                logical_ids: logical_ids
                    .iter()
                    .map(|id| IdWire { space: id.space.as_str(), value: &id.value })
                    .collect(),
            })
        }
    };
    let context = match &value.context {
        GraphReadContextV1::Current { schema_version, context } => {
            GraphContextWire::Current(CurrentContextWire {
                schema_version: *schema_version,
                r#type: "current",
                context: read_context_wire(context),
            })
        }
        GraphReadContextV1::Frozen { schema_version, context } => {
            GraphContextWire::Frozen(FrozenGraphContextWire {
                schema_version: *schema_version,
                r#type: "frozen",
                context: FrozenContextWire {
                    schema_version: context.schema_version,
                    effective_valid_at: context.effective_valid_at,
                    context: read_context_wire(&context.context),
                    token: &context.token,
                },
            })
        }
    };
    serde_json::to_vec(&RequestWire {
        schema_version: value.schema_version,
        seed,
        direction: direction_str(value.direction),
        edge_kinds: &value.edge_kinds,
        target_kinds: &value.target_kinds,
        context,
        max_depth: value.max_depth,
        result_limit: value.result_limit,
        max_work_units: value.max_work_units.to_string(),
        include_explanation: value.include_explanation,
        include_evidence: value.include_evidence,
    })
    .map_err(|_| GraphExpansionErrorV1::new(GraphExpansionErrorReasonV1::GraphCorrupt, ""))
}

fn request_error(
    reason: GraphExpansionErrorReasonV1,
    path: impl Into<String>,
) -> GraphExpansionErrorV1 {
    GraphExpansionErrorV1::new(reason, path)
}

fn object<'a>(
    value: &'a serde_json::Value,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, GraphExpansionErrorV1> {
    value.as_object().ok_or_else(|| request_error(reason, path))
}

fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<&'a serde_json::Value, GraphExpansionErrorV1> {
    object.get(field).ok_or_else(|| request_error(reason, path))
}

fn check_closed(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    base: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if let Some(field) = object.keys().filter(|field| !allowed.contains(&field.as_str())).min() {
        let escaped = field.replace('~', "~0").replace('/', "~1");
        return Err(request_error(
            GraphExpansionErrorReasonV1::UnknownField,
            format!("{base}/{escaped}"),
        ));
    }
    Ok(())
}

fn check_schema(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if object.get("schemaVersion").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(request_error(GraphExpansionErrorReasonV1::UnsupportedSchemaVersion, path));
    }
    Ok(())
}

fn parse_u32(
    value: &serde_json::Value,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<u32, GraphExpansionErrorV1> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| request_error(reason, path))
}

pub(super) fn parse_canonical_u64(
    value: &serde_json::Value,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<u64, GraphExpansionErrorV1> {
    let text = value.as_str().ok_or_else(|| request_error(reason, path))?;
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(request_error(reason, path));
    }
    text.parse().map_err(|_| request_error(reason, path))
}

fn parse_read_context(
    value: &serde_json::Value,
    base: &str,
) -> Result<ReadContextV1, GraphExpansionErrorV1> {
    let context_object = object(value, GraphExpansionErrorReasonV1::GraphContextInvalid, base)?;
    check_schema(context_object, &format!("{base}/schemaVersion"))?;
    check_closed(context_object, &["schemaVersion", "view", "eligibility"], base)?;
    let view_base = format!("{base}/view");
    let view_object = object(
        required(
            context_object,
            "view",
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &view_base,
        )?,
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &view_base,
    )?;
    check_closed(
        view_object,
        &["includeSuperseded", "includeInactive", "includeOutOfWindow", "validAsOf"],
        &view_base,
    )?;
    let boolean = |field: &str| {
        required(
            view_object,
            field,
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &format!("{view_base}/{field}"),
        )?
        .as_bool()
        .ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{view_base}/{field}"),
            )
        })
    };
    let valid_as_of_value = required(
        view_object,
        "validAsOf",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &format!("{view_base}/validAsOf"),
    )?;
    let valid_as_of = if valid_as_of_value.is_null() {
        None
    } else {
        Some(valid_as_of_value.as_i64().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{view_base}/validAsOf"),
            )
        })?)
    };
    let eligibility_base = format!("{base}/eligibility");
    let eligibility_object = object(
        required(
            context_object,
            "eligibility",
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &eligibility_base,
        )?,
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &eligibility_base,
    )?;
    check_closed(
        eligibility_object,
        &["sourceType", "kind", "createdAfter", "status", "attributes"],
        &eligibility_base,
    )?;
    let optional_string = |field: &str| -> Result<Option<String>, GraphExpansionErrorV1> {
        let value = required(
            eligibility_object,
            field,
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &format!("{eligibility_base}/{field}"),
        )?;
        if value.is_null() {
            Ok(None)
        } else {
            value.as_str().map(|value| Some(value.to_string())).ok_or_else(|| {
                request_error(
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    format!("{eligibility_base}/{field}"),
                )
            })
        }
    };
    let created = required(
        eligibility_object,
        "createdAfter",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &format!("{eligibility_base}/createdAfter"),
    )?;
    let created_after = if created.is_null() {
        None
    } else {
        Some(created.as_i64().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/createdAfter"),
            )
        })?)
    };
    let attributes_value = required(
        eligibility_object,
        "attributes",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &format!("{eligibility_base}/attributes"),
    )?;
    let mut attributes = Vec::new();
    for (index, item) in attributes_value
        .as_array()
        .ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes"),
            )
        })?
        .iter()
        .enumerate()
    {
        let pair = item.as_array().filter(|pair| pair.len() == 2).ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes/{index}"),
            )
        })?;
        let name = pair[0].as_str().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes/{index}/0"),
            )
        })?;
        let value = pair[1].as_str().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes/{index}/1"),
            )
        })?;
        attributes.push((name.to_string(), value.to_string()));
    }
    Ok(ReadContextV1 {
        schema_version: 1,
        view: ReadView {
            include_superseded: boolean("includeSuperseded")?,
            include_inactive: boolean("includeInactive")?,
            include_out_of_window: boolean("includeOutOfWindow")?,
            valid_as_of,
        },
        eligibility: SearchFilter {
            source_type: optional_string("sourceType")?,
            kind: optional_string("kind")?,
            created_after,
            status: optional_string("status")?,
            attributes,
        },
    })
}

/// Decode the recursively closed canonical graph-expansion request.
pub fn decode_graph_expand_request_v1(
    bytes: &[u8],
) -> Result<GraphExpandRequestV1, GraphExpansionErrorV1> {
    if bytes.len() > 64 * 1024 {
        return Err(request_error(GraphExpansionErrorReasonV1::GraphContextInvalid, ""));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| request_error(GraphExpansionErrorReasonV1::GraphContextInvalid, ""))?;
    let root = object(&value, GraphExpansionErrorReasonV1::GraphContextInvalid, "")?;
    check_schema(root, "/schemaVersion")?;
    check_closed(
        root,
        &[
            "schemaVersion",
            "seed",
            "direction",
            "edgeKinds",
            "targetKinds",
            "context",
            "maxDepth",
            "resultLimit",
            "maxWorkUnits",
            "includeExplanation",
            "includeEvidence",
        ],
        "",
    )?;
    let seed_value =
        required(root, "seed", GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed")?;
    let seed_object = object(seed_value, GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed")?;
    check_schema(seed_object, "/seed/schemaVersion")?;
    check_closed(
        seed_object,
        &["schemaVersion", "type", "text", "rankedLimit", "logicalIds"],
        "/seed",
    )?;
    let seed_type =
        required(seed_object, "type", GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/type")?
            .as_str()
            .ok_or_else(|| {
                request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/type")
            })?;
    let seed = match seed_type {
        "query" => {
            if seed_object.contains_key("logicalIds") {
                return Err(request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed"));
            }
            GraphSeedV1::Query {
                schema_version: 1,
                text: required(
                    seed_object,
                    "text",
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/text",
                )?
                .as_str()
                .ok_or_else(|| {
                    request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/text")
                })?
                .to_string(),
                ranked_limit: parse_u32(
                    required(
                        seed_object,
                        "rankedLimit",
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        "/seed/rankedLimit",
                    )?,
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/rankedLimit",
                )?,
            }
        }
        "explicit" => {
            if seed_object.contains_key("text") || seed_object.contains_key("rankedLimit") {
                return Err(request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed"));
            }
            let values = required(
                seed_object,
                "logicalIds",
                GraphExpansionErrorReasonV1::GraphSeedInvalid,
                "/seed/logicalIds",
            )?
            .as_array()
            .ok_or_else(|| {
                request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/logicalIds")
            })?;
            let mut logical_ids = Vec::new();
            for (index, value) in values.iter().enumerate() {
                let base = format!("/seed/logicalIds/{index}");
                let object = object(value, GraphExpansionErrorReasonV1::GraphSeedInvalid, &base)?;
                check_closed(object, &["space", "value"], &base)?;
                let space = required(
                    object,
                    "space",
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    &format!("{base}/space"),
                )?
                .as_str();
                let value = required(
                    object,
                    "value",
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    &format!("{base}/value"),
                )?
                .as_str()
                .ok_or_else(|| {
                    request_error(
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        format!("{base}/value"),
                    )
                })?;
                logical_ids.push(match space {
                    Some("logical") => IdSpace::logical(value),
                    Some("content") => IdSpace::content(value),
                    Some("passage") => IdSpace::passage(value),
                    _ => {
                        return Err(request_error(
                            GraphExpansionErrorReasonV1::GraphSeedInvalid,
                            format!("{base}/space"),
                        ))
                    }
                });
            }
            GraphSeedV1::Explicit { schema_version: 1, logical_ids }
        }
        _ => {
            return Err(request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/type"))
        }
    };
    let direction = match required(
        root,
        "direction",
        GraphExpansionErrorReasonV1::GraphDirectionInvalid,
        "/direction",
    )?
    .as_str()
    {
        Some("incoming") => TraversalDirection::Incoming,
        Some("outgoing") => TraversalDirection::Outgoing,
        Some("both") => TraversalDirection::Both,
        _ => {
            return Err(request_error(
                GraphExpansionErrorReasonV1::GraphDirectionInvalid,
                "/direction",
            ))
        }
    };
    let strings = |field: &str, reason: GraphExpansionErrorReasonV1| {
        required(root, field, reason, &format!("/{field}"))?
            .as_array()
            .ok_or_else(|| request_error(reason, format!("/{field}")))?
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| request_error(reason, format!("/{field}/{index}")))
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let context_value =
        required(root, "context", GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")?;
    let context_object =
        object(context_value, GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")?;
    check_schema(context_object, "/context/schemaVersion")?;
    check_closed(context_object, &["schemaVersion", "type", "context"], "/context")?;
    let context_type = required(
        context_object,
        "type",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        "/context/type",
    )?
    .as_str();
    let context = match context_type {
        Some("current") => GraphReadContextV1::Current {
            schema_version: 1,
            context: parse_read_context(
                required(
                    context_object,
                    "context",
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    "/context/context",
                )?,
                "/context/context",
            )?,
        },
        Some("frozen") => {
            let base = "/context/context";
            let frozen_object = object(
                required(
                    context_object,
                    "context",
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    base,
                )?,
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                base,
            )?;
            check_schema(frozen_object, "/context/context/schemaVersion")?;
            check_closed(
                frozen_object,
                &["schemaVersion", "effectiveValidAt", "context", "token"],
                base,
            )?;
            GraphReadContextV1::Frozen {
                schema_version: 1,
                context: FrozenReadContextV1 {
                    schema_version: 1,
                    effective_valid_at: required(
                        frozen_object,
                        "effectiveValidAt",
                        GraphExpansionErrorReasonV1::GraphContextInvalid,
                        "/context/context/effectiveValidAt",
                    )?
                    .as_i64()
                    .ok_or_else(|| {
                        request_error(
                            GraphExpansionErrorReasonV1::GraphContextInvalid,
                            "/context/context/effectiveValidAt",
                        )
                    })?,
                    context: parse_read_context(
                        required(
                            frozen_object,
                            "context",
                            GraphExpansionErrorReasonV1::GraphContextInvalid,
                            "/context/context/context",
                        )?,
                        "/context/context/context",
                    )?,
                    token: required(
                        frozen_object,
                        "token",
                        GraphExpansionErrorReasonV1::GraphContextInvalid,
                        "/context/context/token",
                    )?
                    .as_str()
                    .ok_or_else(|| {
                        request_error(
                            GraphExpansionErrorReasonV1::GraphContextInvalid,
                            "/context/context/token",
                        )
                    })?
                    .to_string(),
                },
            }
        }
        _ => {
            return Err(request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                "/context/type",
            ))
        }
    };
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed,
        direction,
        edge_kinds: strings("edgeKinds", GraphExpansionErrorReasonV1::GraphEdgeKindsInvalid)?,
        target_kinds: strings("targetKinds", GraphExpansionErrorReasonV1::GraphTargetKindsInvalid)?,
        context,
        max_depth: parse_u32(
            required(
                root,
                "maxDepth",
                GraphExpansionErrorReasonV1::GraphDepthInvalid,
                "/maxDepth",
            )?,
            GraphExpansionErrorReasonV1::GraphDepthInvalid,
            "/maxDepth",
        )?,
        result_limit: parse_u32(
            required(
                root,
                "resultLimit",
                GraphExpansionErrorReasonV1::GraphResultLimitInvalid,
                "/resultLimit",
            )?,
            GraphExpansionErrorReasonV1::GraphResultLimitInvalid,
            "/resultLimit",
        )?,
        max_work_units: parse_canonical_u64(
            required(
                root,
                "maxWorkUnits",
                GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
                "/maxWorkUnits",
            )?,
            GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
            "/maxWorkUnits",
        )?,
        include_explanation: required(
            root,
            "includeExplanation",
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            "/includeExplanation",
        )?
        .as_bool()
        .ok_or_else(|| {
            request_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/includeExplanation")
        })?,
        include_evidence: root
            .get("includeEvidence")
            .map(|value| {
                value.as_bool().ok_or_else(|| {
                    request_error(
                        GraphExpansionErrorReasonV1::GraphContextInvalid,
                        "/includeEvidence",
                    )
                })
            })
            .transpose()?
            .unwrap_or(false),
    };
    Ok(request)
}
