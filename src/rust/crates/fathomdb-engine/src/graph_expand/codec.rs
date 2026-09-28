use crate::filter::SearchFilter;
use crate::frozen_read::{FrozenReadContextV1, ReadContextV1};
use crate::identity::IdSpace;
use crate::search_types::{StructuralDependencyStateV1, StructuralLifecycleStateV1};
use crate::temporal::ReadView;
use serde::Serialize;

use super::types::{
    GraphExpandRequestV1, GraphExpandResultV1, GraphExpansionDegradationCodeV1,
    GraphExpansionErrorReasonV1, GraphExpansionErrorV1, GraphExpansionExplanationV1, GraphOriginV1,
    GraphProjectionOriginV1, GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1,
    GraphSeedSourceV1, GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, ResolvedGraphSeedV1,
    TraversalDirection,
};

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

fn direction_str(value: TraversalDirection) -> &'static str {
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

fn parse_canonical_u64(
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

fn degradation_str(value: GraphExpansionDegradationCodeV1) -> &'static str {
    match value {
        GraphExpansionDegradationCodeV1::QuerySeedTextFallback => "query_seed_text_fallback",
        GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified => {
            "projection_legacy_unverified"
        }
        GraphExpansionDegradationCodeV1::ProjectionProcessing => "projection_processing",
        GraphExpansionDegradationCodeV1::ProjectionBlocked => "projection_blocked",
        GraphExpansionDegradationCodeV1::ProjectionDeferred => "projection_deferred",
        GraphExpansionDegradationCodeV1::ProjectionDegraded => "projection_degraded",
    }
}

fn projection_origin_str(value: GraphProjectionOriginV1) -> &'static str {
    match value {
        GraphProjectionOriginV1::NotApplicable => "not_applicable",
        GraphProjectionOriginV1::Fresh => "fresh",
        GraphProjectionOriginV1::LegacyUnverified => "legacy_unverified",
        GraphProjectionOriginV1::Configuration => "configuration",
        GraphProjectionOriginV1::Rebuild => "rebuild",
    }
}

fn projection_readiness_str(value: GraphProjectionReadinessV1) -> &'static str {
    match value {
        GraphProjectionReadinessV1::NotApplicable => "not_applicable",
        GraphProjectionReadinessV1::Ready => "ready",
        GraphProjectionReadinessV1::Processing => "processing",
        GraphProjectionReadinessV1::Blocked => "blocked",
        GraphProjectionReadinessV1::Deferred => "deferred",
        GraphProjectionReadinessV1::Degraded => "degraded",
    }
}

fn lifecycle_str(value: StructuralLifecycleStateV1) -> &'static str {
    match value {
        StructuralLifecycleStateV1::NodePending => "node_pending",
        StructuralLifecycleStateV1::NodeActive => "node_active",
        StructuralLifecycleStateV1::NodeDeleted => "node_deleted",
        StructuralLifecycleStateV1::EdgeValid => "edge_valid",
    }
}

fn dependency_str(value: StructuralDependencyStateV1) -> &'static str {
    match value {
        StructuralDependencyStateV1::NotApplicable => "not_applicable",
        StructuralDependencyStateV1::NotRegistered => "not_registered",
        StructuralDependencyStateV1::Registered => "registered",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedSeedWire<'a> {
    schema_version: u32,
    logical_id: &'a str,
    seed_ordinal: u32,
    query_score: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OriginWire<'a> {
    schema_version: u32,
    seed_logical_id: &'a str,
    seed_ordinal: u32,
    predecessor_logical_id: &'a str,
    target_logical_id: &'a str,
    hop_count: u32,
    terminal_edge_kind: &'a str,
    terminal_direction: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetWire<'a> {
    schema_version: u32,
    logical_id: &'a str,
    kind: &'a str,
    body: &'a str,
    write_cursor: String,
    origin: OriginWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetExplanationWire<'a> {
    schema_version: u32,
    target_index: u32,
    origin: OriginWire<'a>,
    lifecycle_state: &'static str,
    dependency_state: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExplanationWire<'a> {
    schema_version: u32,
    correlation_id: &'a str,
    seed_source: &'static str,
    read_mode: &'static str,
    projection_generation_id: Option<&'a str>,
    projection_origin: &'static str,
    projection_readiness: &'static str,
    degradation_codes: Vec<&'static str>,
    per_target: Vec<TargetExplanationWire<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceEntryWire<'a> {
    schema_version: u32,
    target_index: u32,
    target_artifact_revision_id: &'a str,
    target_evidence_ref: &'a str,
    terminal_edge_artifact_revision_id: &'a str,
    terminal_edge_evidence_ref: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceSidecarWire<'a> {
    schema_version: u32,
    entries: Vec<EvidenceEntryWire<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultWire<'a> {
    schema_version: u32,
    seeds: Vec<ResolvedSeedWire<'a>>,
    targets: Vec<TargetWire<'a>>,
    complete: bool,
    work_units: String,
    degradation_codes: Vec<&'static str>,
    explanation: Option<ExplanationWire<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<EvidenceSidecarWire<'a>>,
}

fn origin_wire(value: &GraphOriginV1) -> OriginWire<'_> {
    OriginWire {
        schema_version: value.schema_version,
        seed_logical_id: &value.seed_logical_id,
        seed_ordinal: value.seed_ordinal,
        predecessor_logical_id: &value.predecessor_logical_id,
        target_logical_id: &value.target_logical_id,
        hop_count: value.hop_count,
        terminal_edge_kind: &value.terminal_edge_kind,
        terminal_direction: direction_str(value.terminal_direction),
    }
}

/// Encode a graph-expansion response into canonical declaration-order JSON.
pub fn encode_graph_expand_result_v1(
    value: &GraphExpandResultV1,
) -> Result<Vec<u8>, GraphExpansionErrorV1> {
    validate_response_coherence(value)?;
    let explanation = value.explanation.as_ref().map(|explanation| ExplanationWire {
        schema_version: explanation.schema_version,
        correlation_id: &explanation.correlation_id,
        seed_source: match explanation.seed_source {
            GraphSeedSourceV1::Query => "query",
            GraphSeedSourceV1::Explicit => "explicit",
        },
        read_mode: match explanation.read_mode {
            GraphReadModeV1::Current => "current",
            GraphReadModeV1::Frozen => "frozen",
        },
        projection_generation_id: explanation.projection_generation_id.as_deref(),
        projection_origin: projection_origin_str(explanation.projection_origin),
        projection_readiness: projection_readiness_str(explanation.projection_readiness),
        degradation_codes: explanation
            .degradation_codes
            .iter()
            .copied()
            .map(degradation_str)
            .collect(),
        per_target: explanation
            .per_target
            .iter()
            .map(|target| TargetExplanationWire {
                schema_version: target.schema_version,
                target_index: target.target_index,
                origin: origin_wire(&target.origin),
                lifecycle_state: lifecycle_str(target.lifecycle_state),
                dependency_state: dependency_str(target.dependency_state),
            })
            .collect(),
    });
    serde_json::to_vec(&ResultWire {
        schema_version: value.schema_version,
        seeds: value
            .seeds
            .iter()
            .map(|seed| ResolvedSeedWire {
                schema_version: seed.schema_version,
                logical_id: &seed.logical_id,
                seed_ordinal: seed.seed_ordinal,
                query_score: seed.query_score,
            })
            .collect(),
        targets: value
            .targets
            .iter()
            .map(|target| TargetWire {
                schema_version: target.schema_version,
                logical_id: &target.logical_id,
                kind: &target.kind,
                body: &target.body,
                write_cursor: target.write_cursor.to_string(),
                origin: origin_wire(&target.origin),
            })
            .collect(),
        complete: value.complete,
        work_units: value.work_units.to_string(),
        degradation_codes: value.degradation_codes.iter().copied().map(degradation_str).collect(),
        explanation,
        evidence: value.evidence.as_ref().map(|sidecar| EvidenceSidecarWire {
            schema_version: sidecar.schema_version,
            entries: sidecar
                .entries
                .iter()
                .map(|entry| EvidenceEntryWire {
                    schema_version: entry.schema_version,
                    target_index: entry.target_index,
                    target_artifact_revision_id: entry.target_artifact_revision_id.as_str(),
                    target_evidence_ref: entry.target_evidence_ref.as_str(),
                    terminal_edge_artifact_revision_id: entry
                        .terminal_edge_artifact_revision_id
                        .as_str(),
                    terminal_edge_evidence_ref: entry.terminal_edge_evidence_ref.as_str(),
                })
                .collect(),
        }),
    })
    .map_err(|_| GraphExpansionErrorV1::new(GraphExpansionErrorReasonV1::GraphCorrupt, ""))
}

fn response_error(
    reason: GraphExpansionErrorReasonV1,
    path: impl Into<String>,
) -> GraphExpansionErrorV1 {
    GraphExpansionErrorV1::new(reason, path)
}

fn response_object<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, GraphExpansionErrorV1> {
    value.as_object().ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn response_required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
) -> Result<&'a serde_json::Value, GraphExpansionErrorV1> {
    object.get(field).ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn response_closed(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    base: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if let Some(field) = object.keys().filter(|field| !allowed.contains(&field.as_str())).min() {
        let escaped = field.replace('~', "~0").replace('/', "~1");
        return Err(response_error(
            GraphExpansionErrorReasonV1::GraphCorrupt,
            format!("{base}/{escaped}"),
        ));
    }
    Ok(())
}

fn response_schema(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if object.get("schemaVersion").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(response_error(GraphExpansionErrorReasonV1::UnsupportedSchemaVersion, path));
    }
    Ok(())
}

fn response_u32(value: &serde_json::Value, path: &str) -> Result<u32, GraphExpansionErrorV1> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn response_u64(value: &serde_json::Value, path: &str) -> Result<u64, GraphExpansionErrorV1> {
    parse_canonical_u64(value, GraphExpansionErrorReasonV1::GraphCorrupt, path)
}

fn response_string(value: &serde_json::Value, path: &str) -> Result<String, GraphExpansionErrorV1> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn parse_direction(
    value: &serde_json::Value,
    path: &str,
) -> Result<TraversalDirection, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("incoming") => Ok(TraversalDirection::Incoming),
        Some("outgoing") => Ok(TraversalDirection::Outgoing),
        Some("both") => Ok(TraversalDirection::Both),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_origin(
    value: &serde_json::Value,
    base: &str,
) -> Result<GraphOriginV1, GraphExpansionErrorV1> {
    let object = response_object(value, base)?;
    response_schema(object, &format!("{base}/schemaVersion"))?;
    Ok(GraphOriginV1 {
        schema_version: 1,
        seed_logical_id: response_string(
            response_required(object, "seedLogicalId", &format!("{base}/seedLogicalId"))?,
            &format!("{base}/seedLogicalId"),
        )?,
        seed_ordinal: response_u32(
            response_required(object, "seedOrdinal", &format!("{base}/seedOrdinal"))?,
            &format!("{base}/seedOrdinal"),
        )?,
        predecessor_logical_id: response_string(
            response_required(
                object,
                "predecessorLogicalId",
                &format!("{base}/predecessorLogicalId"),
            )?,
            &format!("{base}/predecessorLogicalId"),
        )?,
        target_logical_id: response_string(
            response_required(object, "targetLogicalId", &format!("{base}/targetLogicalId"))?,
            &format!("{base}/targetLogicalId"),
        )?,
        hop_count: response_u32(
            response_required(object, "hopCount", &format!("{base}/hopCount"))?,
            &format!("{base}/hopCount"),
        )?,
        terminal_edge_kind: response_string(
            response_required(object, "terminalEdgeKind", &format!("{base}/terminalEdgeKind"))?,
            &format!("{base}/terminalEdgeKind"),
        )?,
        terminal_direction: parse_direction(
            response_required(object, "terminalDirection", &format!("{base}/terminalDirection"))?,
            &format!("{base}/terminalDirection"),
        )?,
    })
}

fn parse_degradation(
    value: &serde_json::Value,
    path: &str,
) -> Result<GraphExpansionDegradationCodeV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("query_seed_text_fallback") => {
            Ok(GraphExpansionDegradationCodeV1::QuerySeedTextFallback)
        }
        Some("projection_legacy_unverified") => {
            Ok(GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified)
        }
        Some("projection_processing") => Ok(GraphExpansionDegradationCodeV1::ProjectionProcessing),
        Some("projection_blocked") => Ok(GraphExpansionDegradationCodeV1::ProjectionBlocked),
        Some("projection_deferred") => Ok(GraphExpansionDegradationCodeV1::ProjectionDeferred),
        Some("projection_degraded") => Ok(GraphExpansionDegradationCodeV1::ProjectionDegraded),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_degradations(
    value: &serde_json::Value,
    base: &str,
) -> Result<Vec<GraphExpansionDegradationCodeV1>, GraphExpansionErrorV1> {
    value
        .as_array()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, base))?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_degradation(value, &format!("{base}/{index}")))
        .collect()
}

fn parse_projection_origin(
    value: &serde_json::Value,
    path: &str,
) -> Result<GraphProjectionOriginV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("not_applicable") => Ok(GraphProjectionOriginV1::NotApplicable),
        Some("fresh") => Ok(GraphProjectionOriginV1::Fresh),
        Some("legacy_unverified") => Ok(GraphProjectionOriginV1::LegacyUnverified),
        Some("configuration") => Ok(GraphProjectionOriginV1::Configuration),
        Some("rebuild") => Ok(GraphProjectionOriginV1::Rebuild),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_projection_readiness(
    value: &serde_json::Value,
    path: &str,
) -> Result<GraphProjectionReadinessV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("not_applicable") => Ok(GraphProjectionReadinessV1::NotApplicable),
        Some("ready") => Ok(GraphProjectionReadinessV1::Ready),
        Some("processing") => Ok(GraphProjectionReadinessV1::Processing),
        Some("blocked") => Ok(GraphProjectionReadinessV1::Blocked),
        Some("deferred") => Ok(GraphProjectionReadinessV1::Deferred),
        Some("degraded") => Ok(GraphProjectionReadinessV1::Degraded),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_lifecycle(
    value: &serde_json::Value,
    path: &str,
) -> Result<StructuralLifecycleStateV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("node_pending") => Ok(StructuralLifecycleStateV1::NodePending),
        Some("node_active") => Ok(StructuralLifecycleStateV1::NodeActive),
        Some("node_deleted") => Ok(StructuralLifecycleStateV1::NodeDeleted),
        Some("edge_valid") => Ok(StructuralLifecycleStateV1::EdgeValid),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_dependency(
    value: &serde_json::Value,
    path: &str,
) -> Result<StructuralDependencyStateV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("not_applicable") => Ok(StructuralDependencyStateV1::NotApplicable),
        Some("not_registered") => Ok(StructuralDependencyStateV1::NotRegistered),
        Some("registered") => Ok(StructuralDependencyStateV1::Registered),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn validate_response_coherence(value: &GraphExpandResultV1) -> Result<(), GraphExpansionErrorV1> {
    if value.schema_version != 1 {
        return Err(response_error(
            GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ));
    }
    if !value.complete {
        return Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/complete"));
    }
    for (index, seed) in value.seeds.iter().enumerate() {
        if seed.schema_version != 1 {
            return Err(response_error(
                GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                format!("/seeds/{index}/schemaVersion"),
            ));
        }
        if seed.seed_ordinal as usize != index {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/seeds/{index}/seedOrdinal"),
            ));
        }
        if seed.query_score.is_some_and(|score| !score.is_finite()) {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/seeds/{index}/queryScore"),
            ));
        }
    }
    for (index, target) in value.targets.iter().enumerate() {
        let ordinal = target.origin.seed_ordinal as usize;
        if ordinal >= value.seeds.len() {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/targets/{index}/origin/seedOrdinal"),
            ));
        }
        if target.origin.seed_logical_id != value.seeds[ordinal].logical_id {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/targets/{index}/origin/seedLogicalId"),
            ));
        }
        if target.origin.target_logical_id != target.logical_id {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/targets/{index}/origin/targetLogicalId"),
            ));
        }
    }
    if let Some(evidence) = &value.evidence {
        if evidence.schema_version != 1 || evidence.entries.len() != value.targets.len() {
            return Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/evidence"));
        }
        for (index, entry) in evidence.entries.iter().enumerate() {
            if entry.schema_version != 1 || entry.target_index as usize != index {
                return Err(response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("/evidence/entries/{index}"),
                ));
            }
        }
    }
    if let Some(explanation) = &value.explanation {
        if explanation.per_target.len() != value.targets.len() {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                "/explanation/perTarget",
            ));
        }
        for (index, item) in explanation.per_target.iter().enumerate() {
            if item.target_index as usize != index {
                return Err(response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("/explanation/perTarget/{index}/targetIndex"),
                ));
            }
            if item.origin != value.targets[index].origin {
                return Err(response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("/explanation/perTarget/{index}/origin"),
                ));
            }
        }
        if explanation.degradation_codes != value.degradation_codes {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                "/explanation/degradationCodes",
            ));
        }
    }
    Ok(())
}

/// Decode an additive graph-expansion response and verify all cross-field coherence.
pub fn decode_graph_expand_result_v1(
    bytes: &[u8],
) -> Result<GraphExpandResultV1, GraphExpansionErrorV1> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, ""))?;
    let root = response_object(&value, "")?;
    response_schema(root, "/schemaVersion")?;
    let seed_values = response_required(root, "seeds", "/seeds")?
        .as_array()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/seeds"))?;
    let mut seeds = Vec::new();
    for (index, value) in seed_values.iter().enumerate() {
        let base = format!("/seeds/{index}");
        let object = response_object(value, &base)?;
        response_schema(object, &format!("{base}/schemaVersion"))?;
        let score_value = response_required(object, "queryScore", &format!("{base}/queryScore"))?;
        let query_score = if score_value.is_null() {
            None
        } else {
            Some(score_value.as_f64().filter(|score| score.is_finite()).ok_or_else(|| {
                response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("{base}/queryScore"),
                )
            })?)
        };
        seeds.push(ResolvedGraphSeedV1 {
            schema_version: 1,
            logical_id: response_string(
                response_required(object, "logicalId", &format!("{base}/logicalId"))?,
                &format!("{base}/logicalId"),
            )?,
            seed_ordinal: response_u32(
                response_required(object, "seedOrdinal", &format!("{base}/seedOrdinal"))?,
                &format!("{base}/seedOrdinal"),
            )?,
            query_score,
        });
    }
    let target_values = response_required(root, "targets", "/targets")?
        .as_array()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/targets"))?;
    let mut targets = Vec::new();
    for (index, value) in target_values.iter().enumerate() {
        let base = format!("/targets/{index}");
        let object = response_object(value, &base)?;
        response_schema(object, &format!("{base}/schemaVersion"))?;
        targets.push(GraphTargetV1 {
            schema_version: 1,
            logical_id: response_string(
                response_required(object, "logicalId", &format!("{base}/logicalId"))?,
                &format!("{base}/logicalId"),
            )?,
            kind: response_string(
                response_required(object, "kind", &format!("{base}/kind"))?,
                &format!("{base}/kind"),
            )?,
            body: response_string(
                response_required(object, "body", &format!("{base}/body"))?,
                &format!("{base}/body"),
            )?,
            write_cursor: response_u64(
                response_required(object, "writeCursor", &format!("{base}/writeCursor"))?,
                &format!("{base}/writeCursor"),
            )?,
            origin: parse_origin(
                response_required(object, "origin", &format!("{base}/origin"))?,
                &format!("{base}/origin"),
            )?,
        });
    }
    let complete = response_required(root, "complete", "/complete")?
        .as_bool()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/complete"))?;
    let work_units =
        response_u64(response_required(root, "workUnits", "/workUnits")?, "/workUnits")?;
    let degradation_codes = parse_degradations(
        response_required(root, "degradationCodes", "/degradationCodes")?,
        "/degradationCodes",
    )?;
    let explanation_value = response_required(root, "explanation", "/explanation")?;
    let explanation = if explanation_value.is_null() {
        None
    } else {
        let object = response_object(explanation_value, "/explanation")?;
        response_schema(object, "/explanation/schemaVersion")?;
        let per_target_values = response_required(object, "perTarget", "/explanation/perTarget")?
            .as_array()
            .ok_or_else(|| {
                response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/explanation/perTarget")
            })?;
        let mut per_target = Vec::new();
        for (index, value) in per_target_values.iter().enumerate() {
            let base = format!("/explanation/perTarget/{index}");
            let item = response_object(value, &base)?;
            response_schema(item, &format!("{base}/schemaVersion"))?;
            per_target.push(GraphTargetExplanationV1 {
                schema_version: 1,
                target_index: response_u32(
                    response_required(item, "targetIndex", &format!("{base}/targetIndex"))?,
                    &format!("{base}/targetIndex"),
                )?,
                origin: parse_origin(
                    response_required(item, "origin", &format!("{base}/origin"))?,
                    &format!("{base}/origin"),
                )?,
                lifecycle_state: parse_lifecycle(
                    response_required(item, "lifecycleState", &format!("{base}/lifecycleState"))?,
                    &format!("{base}/lifecycleState"),
                )?,
                dependency_state: parse_dependency(
                    response_required(item, "dependencyState", &format!("{base}/dependencyState"))?,
                    &format!("{base}/dependencyState"),
                )?,
            });
        }
        let generation = response_required(
            object,
            "projectionGenerationId",
            "/explanation/projectionGenerationId",
        )?;
        Some(GraphExpansionExplanationV1 {
            schema_version: 1,
            correlation_id: response_string(
                response_required(object, "correlationId", "/explanation/correlationId")?,
                "/explanation/correlationId",
            )?,
            seed_source: match response_required(object, "seedSource", "/explanation/seedSource")?
                .as_str()
            {
                Some("query") => GraphSeedSourceV1::Query,
                Some("explicit") => GraphSeedSourceV1::Explicit,
                _ => {
                    return Err(response_error(
                        GraphExpansionErrorReasonV1::GraphCorrupt,
                        "/explanation/seedSource",
                    ))
                }
            },
            read_mode: match response_required(object, "readMode", "/explanation/readMode")?
                .as_str()
            {
                Some("current") => GraphReadModeV1::Current,
                Some("frozen") => GraphReadModeV1::Frozen,
                _ => {
                    return Err(response_error(
                        GraphExpansionErrorReasonV1::GraphCorrupt,
                        "/explanation/readMode",
                    ))
                }
            },
            projection_generation_id: if generation.is_null() {
                None
            } else {
                Some(response_string(generation, "/explanation/projectionGenerationId")?)
            },
            projection_origin: parse_projection_origin(
                response_required(object, "projectionOrigin", "/explanation/projectionOrigin")?,
                "/explanation/projectionOrigin",
            )?,
            projection_readiness: parse_projection_readiness(
                response_required(
                    object,
                    "projectionReadiness",
                    "/explanation/projectionReadiness",
                )?,
                "/explanation/projectionReadiness",
            )?,
            degradation_codes: parse_degradations(
                response_required(object, "degradationCodes", "/explanation/degradationCodes")?,
                "/explanation/degradationCodes",
            )?,
            per_target,
        })
    };
    let evidence = match root.get("evidence") {
        None => None,
        Some(value) => {
            let object = response_object(value, "/evidence")?;
            response_schema(object, "/evidence/schemaVersion")?;
            response_closed(object, &["schemaVersion", "entries"], "/evidence")?;
            let values = response_required(object, "entries", "/evidence/entries")?
                .as_array()
                .ok_or_else(|| {
                    response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/evidence/entries")
                })?;
            let mut entries = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                let base = format!("/evidence/entries/{index}");
                let object = response_object(value, &base)?;
                response_schema(object, &format!("{base}/schemaVersion"))?;
                response_closed(
                    object,
                    &[
                        "schemaVersion",
                        "targetIndex",
                        "targetArtifactRevisionId",
                        "targetEvidenceRef",
                        "terminalEdgeArtifactRevisionId",
                        "terminalEdgeEvidenceRef",
                    ],
                    &base,
                )?;
                let string = |name: &str| {
                    response_string(
                        response_required(object, name, &format!("{base}/{name}"))?,
                        &format!("{base}/{name}"),
                    )
                };
                let revision = |name: &str| {
                    let value = string(name)?;
                    crate::ArtifactRevisionId::new(value).map_err(|_| {
                        response_error(
                            GraphExpansionErrorReasonV1::GraphCorrupt,
                            format!("{base}/{name}"),
                        )
                    })
                };
                let reference = |name: &str| {
                    crate::GraphEvidenceRefV1::new(string(name)?).map_err(|_| {
                        response_error(
                            GraphExpansionErrorReasonV1::GraphCorrupt,
                            format!("{base}/{name}"),
                        )
                    })
                };
                entries.push(crate::GraphEvidenceSidecarEntryV1 {
                    schema_version: 1,
                    target_index: response_u32(
                        response_required(object, "targetIndex", &format!("{base}/targetIndex"))?,
                        &format!("{base}/targetIndex"),
                    )?,
                    target_artifact_revision_id: revision("targetArtifactRevisionId")?,
                    target_evidence_ref: reference("targetEvidenceRef")?,
                    terminal_edge_artifact_revision_id: revision("terminalEdgeArtifactRevisionId")?,
                    terminal_edge_evidence_ref: reference("terminalEdgeEvidenceRef")?,
                });
            }
            Some(crate::GraphEvidenceSidecarV1 { schema_version: 1, entries })
        }
    };
    let result = GraphExpandResultV1 {
        schema_version: 1,
        seeds,
        targets,
        complete,
        work_units,
        degradation_codes,
        explanation,
        evidence,
    };
    validate_response_coherence(&result)?;
    Ok(result)
}
