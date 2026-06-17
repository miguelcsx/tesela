#![deny(warnings)]
#![deny(missing_docs)]

//! GraphQL integration for the Tesela runtime.
//!
//! Generates a `async-graphql` schema from a compiled [`Spec`] and delegates
//! all resolver calls to a [`Runtime`] instance.
//!
//! # Architecture
//!
//! - [`GraphQLSchemaBuilder`] inspects the spec and builds a dynamic schema with
//!   one query field per object type (`search_<name>`, `get_<name>`, `aggregate_<name>`).
//! - [`GraphQLOptions`] bundles the runtime and optional actor resolver.
//! - `graphql_router()` returns an Axum-compatible router that mounts the
//!   GraphQL playground at `GET /graphql` and the endpoint at `POST /graphql`.
//!
//! Because `async-graphql`'s dynamic schema API requires runtime field
//! registration, the schema is rebuilt whenever `apply_spec` is called.

use async_graphql::{
    Value as GqlValue,
    dynamic::{
        Field, FieldFuture, FieldValue, InputValue, Object, ResolverContext, Scalar, Schema,
        SchemaBuilder, TypeRef,
    },
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tesela_core::{ApiName, DataType, Error, Value};
use tesela_ir::{AggregateView, ArtifactType, JobType, ObjectType, Spec, UploadFlow};
use tesela_runtime::{
    ports::ActorResolver,
    query::{Actor, AggregateQuery, Query},
    runtime::Runtime,
};

mod contract;

pub use contract::{
    FieldProjection, graphql_has_any_field, graphql_has_field, graphql_page, project_record,
};

// ---------------------------------------------------------------------------
// GraphQLOptions
// ---------------------------------------------------------------------------

/// Options for the GraphQL integration.
pub struct GraphQLOptions {
    /// The runtime to delegate queries to.
    pub runtime: Arc<Runtime>,
    /// Optional actor resolver.
    pub actor_resolver: Option<Arc<dyn ActorResolver>>,
}

impl GraphQLOptions {
    /// Create options from a runtime.
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            actor_resolver: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Schema builder
// ---------------------------------------------------------------------------

/// Builds a `async-graphql` dynamic `Schema` from a Tesela spec.
pub struct GraphQLSchemaBuilder;

impl GraphQLSchemaBuilder {
    /// Build the schema.
    ///
    /// Generates query fields for every object type in the spec:
    /// - `search_<name>(limit: Int, offset: Int): [<Name>]`
    /// - `get_<name>(pk: String): <Name>`
    pub fn build(spec: &Spec, runtime: Arc<Runtime>) -> Result<Schema, Error> {
        // async-graphql requires at least one field on Query.
        let mut query = Object::new("Query").field(Field::new(
            "_ping",
            TypeRef::named_nn(TypeRef::STRING),
            |_ctx| {
                FieldFuture::new(async {
                    Ok(Some(FieldValue::value(GqlValue::String(
                        "pong".to_string(),
                    ))))
                })
            },
        ));

        for ot in &spec.object_types {
            let type_name = pascal_case(ot.api_name.as_ref());
            query = add_search_field(query, ot, runtime.clone());
            query = add_get_field(query, ot, &type_name, runtime.clone());
            query = add_aggregate_field(query, ot, runtime.clone());
        }
        for view in &spec.aggregate_views {
            query = add_aggregate_view_field(query, view, runtime.clone());
        }
        for artifact in &spec.artifact_types {
            query = add_artifact_locator_field(query, artifact, runtime.clone());
        }
        for upload in &spec.upload_flows {
            query = add_upload_flow_field(query, upload, runtime.clone());
        }
        for job in &spec.job_types {
            query = add_job_start_field(query, job, runtime.clone());
        }

        // Build object types.
        let mut builder: SchemaBuilder = Schema::build("Query", None, None).register(query);

        for ot in &spec.object_types {
            let obj = build_gql_object(ot);
            builder = builder.register(obj);
        }

        // Register JSON scalar for arbitrary values.
        let json_scalar = Scalar::new("JSON").description("Arbitrary JSON value");
        builder = builder.register(json_scalar);

        builder
            .finish()
            .map_err(|e| Error::internal(format!("GraphQL schema build failed: {}", e)))
    }
}

// ---------------------------------------------------------------------------
// Field builders
// ---------------------------------------------------------------------------

fn add_search_field(query: Object, ot: &ObjectType, runtime: Arc<Runtime>) -> Object {
    let api_name = ot.api_name.clone();
    let type_name = pascal_case(api_name.as_ref());
    let field_name = format!("search_{}", api_name);

    query.field(
        Field::new(
            field_name,
            TypeRef::named_nn_list_nn(type_name),
            move |ctx| {
                let rt = runtime.clone();
                let obj = api_name.clone();
                FieldFuture::new(async move {
                    let actor = actor_from_ctx(&ctx)?;
                    let limit = ctx
                        .args
                        .get("limit")
                        .and_then(|v| v.i64().ok())
                        .map(|v| v as i32);
                    let offset = ctx
                        .args
                        .get("offset")
                        .and_then(|v| v.i64().ok())
                        .map(|v| v as i32);
                    let query = Query {
                        limit,
                        offset,
                        filter: None,
                        sort: Vec::new(),
                        cursor: None,
                    };
                    let page = rt
                        .search(&actor, &obj, query)
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    let values: Vec<FieldValue<'_>> = page
                        .records
                        .into_iter()
                        .map(|r| {
                            let obj_val = record_to_gql_object(r.values);
                            FieldValue::owned_any(obj_val)
                        })
                        .collect();
                    Ok(Some(FieldValue::list(values)))
                })
            },
        )
        .argument(InputValue::new("limit", TypeRef::named(TypeRef::INT)))
        .argument(InputValue::new("offset", TypeRef::named(TypeRef::INT))),
    )
}

fn add_get_field(query: Object, ot: &ObjectType, type_name: &str, runtime: Arc<Runtime>) -> Object {
    let api_name = ot.api_name.clone();
    let type_name = type_name.to_string();

    query.field(
        Field::new(
            format!("get_{}", api_name),
            TypeRef::named(type_name),
            move |ctx| {
                let rt = runtime.clone();
                let obj = api_name.clone();
                FieldFuture::new(async move {
                    let actor = actor_from_ctx(&ctx)?;
                    let pk_str = ctx
                        .args
                        .get("pk")
                        .and_then(|v| v.string().ok())
                        .unwrap_or_default()
                        .to_string();
                    let pk = Value::string(pk_str);
                    let record = rt
                        .get(&actor, &obj, &pk)
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    let obj_val = record_to_gql_object(record.values);
                    Ok(Some(FieldValue::owned_any(obj_val)))
                })
            },
        )
        .argument(InputValue::new("pk", TypeRef::named_nn(TypeRef::STRING))),
    )
}

fn add_aggregate_field(query: Object, ot: &ObjectType, runtime: Arc<Runtime>) -> Object {
    let api_name = ot.api_name.clone();

    query.field(Field::new(
        format!("aggregate_{}", api_name),
        TypeRef::named_nn("JSON"),
        move |ctx| {
            let rt = runtime.clone();
            let obj = api_name.clone();
            FieldFuture::new(async move {
                let actor = actor_from_ctx(&ctx)?;
                let q = AggregateQuery::default();
                let result = rt
                    .aggregate(&actor, &obj, q)
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let json = serde_json::to_value(&result).unwrap_or_default();
                Ok(Some(FieldValue::value(json_to_gql(json))))
            })
        },
    ))
}

fn add_aggregate_view_field(query: Object, view: &AggregateView, runtime: Arc<Runtime>) -> Object {
    let api_name = view.api_name.clone();
    query.field(Field::new(
        format!("aggregate_view_{}", api_name),
        TypeRef::named_nn("JSON"),
        move |ctx| {
            let rt = runtime.clone();
            let view_name = api_name.clone();
            FieldFuture::new(async move {
                let actor = actor_from_ctx(&ctx)?;
                let result = rt
                    .aggregate_view(&actor, &view_name)
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let json = serde_json::to_value(&result).unwrap_or_default();
                Ok(Some(FieldValue::value(json_to_gql(json))))
            })
        },
    ))
}

fn add_artifact_locator_field(
    query: Object,
    artifact: &ArtifactType,
    runtime: Arc<Runtime>,
) -> Object {
    let api_name = artifact.api_name.clone();
    query.field(
        Field::new(
            format!("artifact_{}", api_name),
            TypeRef::named_nn("JSON"),
            move |ctx| {
                let rt = runtime.clone();
                let artifact_name = api_name.clone();
                FieldFuture::new(async move {
                    let actor = actor_from_ctx(&ctx)?;
                    let params = gql_json_arg(&ctx, "params")?;
                    let ttl = ctx
                        .args
                        .get("ttl")
                        .and_then(|v| v.i64().ok())
                        .unwrap_or(300) as u64;
                    let locator = rt
                        .authorize_artifact_read(&actor, &artifact_name, params, ttl)
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(Some(FieldValue::value(json_to_gql(
                        serde_json::to_value(locator).unwrap_or_default(),
                    ))))
                })
            },
        )
        .argument(InputValue::new("params", TypeRef::named(TypeRef::STRING)))
        .argument(InputValue::new("ttl", TypeRef::named(TypeRef::INT))),
    )
}

fn add_upload_flow_field(query: Object, flow: &UploadFlow, runtime: Arc<Runtime>) -> Object {
    let api_name = flow.api_name.clone();
    query.field(
        Field::new(
            format!("upload_flow_{}", api_name),
            TypeRef::named_nn("JSON"),
            move |ctx| {
                let rt = runtime.clone();
                let flow_name = api_name.clone();
                FieldFuture::new(async move {
                    let actor = actor_from_ctx(&ctx)?;
                    let params = gql_json_arg(&ctx, "params")?;
                    let ttl = ctx
                        .args
                        .get("ttl")
                        .and_then(|v| v.i64().ok())
                        .unwrap_or(900) as u64;
                    let upload = rt
                        .initiate_upload_flow(&actor, &flow_name, params, ttl)
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(Some(FieldValue::value(json_to_gql(
                        serde_json::to_value(upload).unwrap_or_default(),
                    ))))
                })
            },
        )
        .argument(InputValue::new("params", TypeRef::named(TypeRef::STRING)))
        .argument(InputValue::new("ttl", TypeRef::named(TypeRef::INT))),
    )
}

fn add_job_start_field(query: Object, job: &JobType, runtime: Arc<Runtime>) -> Object {
    let api_name = job.api_name.clone();
    query.field(
        Field::new(
            format!("start_job_{}", api_name),
            TypeRef::named_nn("JSON"),
            move |ctx| {
                let rt = runtime.clone();
                let job_name = api_name.clone();
                FieldFuture::new(async move {
                    let actor = actor_from_ctx(&ctx)?;
                    let input = gql_json_arg(&ctx, "input")?;
                    let idempotency_key = ctx
                        .args
                        .get("idempotencyKey")
                        .and_then(|v| v.string().ok())
                        .map(ToString::to_string);
                    let run = rt
                        .start_job(&actor, &job_name, input, idempotency_key)
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    Ok(Some(FieldValue::value(json_to_gql(
                        serde_json::to_value(run).unwrap_or_default(),
                    ))))
                })
            },
        )
        .argument(InputValue::new("input", TypeRef::named(TypeRef::STRING)))
        .argument(InputValue::new(
            "idempotencyKey",
            TypeRef::named(TypeRef::STRING),
        )),
    )
}

// ---------------------------------------------------------------------------
// Object type registration
// ---------------------------------------------------------------------------

fn build_gql_object(ot: &ObjectType) -> Object {
    let type_name = pascal_case(ot.api_name.as_ref());
    let mut obj = Object::new(&type_name);

    for prop in &ot.properties {
        let field_name = prop.api_name.to_string();
        let type_ref = data_type_to_gql_type(prop.data_type);

        obj = obj.field(Field::new(field_name.clone(), type_ref, move |ctx| {
            let fname = field_name.clone();
            FieldFuture::new(async move {
                let record: &BTreeMap<String, GqlValue> = ctx
                    .parent_value
                    .downcast_ref()
                    .ok_or_else(|| async_graphql::Error::new("type error"))?;
                let val = record.get(&fname).cloned().unwrap_or(GqlValue::Null);
                Ok(Some(FieldValue::value(val)))
            })
        }));
    }

    obj
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn actor_from_ctx(ctx: &ResolverContext<'_>) -> async_graphql::Result<Actor> {
    ctx.data_opt::<Actor>()
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("GraphQL actor context is required"))
}

fn gql_json_arg(
    ctx: &ResolverContext<'_>,
    name: &str,
) -> async_graphql::Result<BTreeMap<String, Value>> {
    let Some(value) = ctx.args.get(name) else {
        return Ok(BTreeMap::new());
    };
    let raw = value.string().unwrap_or("{}");
    serde_json::from_str(raw)
        .map_err(|e| async_graphql::Error::new(format!("invalid JSON argument '{}': {}", name, e)))
}

fn data_type_to_gql_type(dt: DataType) -> TypeRef {
    match dt {
        DataType::String
        | DataType::Date
        | DataType::Timestamp
        | DataType::TimestampTz
        | DataType::Uuid
        | DataType::Geometry
        | DataType::Enum => TypeRef::named(TypeRef::STRING),
        DataType::Integer | DataType::BigInt => TypeRef::named(TypeRef::INT),
        DataType::Float | DataType::Decimal => TypeRef::named(TypeRef::FLOAT),
        DataType::Boolean => TypeRef::named(TypeRef::BOOLEAN),
        DataType::Json | DataType::Array => TypeRef::named("JSON"),
        DataType::Vector(_) => TypeRef::named("JSON"),
    }
}

fn record_to_gql_object(values: BTreeMap<ApiName, Value>) -> BTreeMap<String, GqlValue> {
    values
        .into_iter()
        .map(|(k, v)| (k.to_string(), json_to_gql(v.0)))
        .collect()
}

fn json_to_gql(v: serde_json::Value) -> GqlValue {
    match v {
        serde_json::Value::Null => GqlValue::Null,
        serde_json::Value::Bool(b) => GqlValue::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                GqlValue::Number(async_graphql::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                GqlValue::Number(
                    async_graphql::Number::from_f64(f)
                        .unwrap_or_else(|| async_graphql::Number::from(0i64)),
                )
            } else {
                GqlValue::Null
            }
        }
        serde_json::Value::String(s) => GqlValue::String(s),
        serde_json::Value::Array(arr) => GqlValue::List(arr.into_iter().map(json_to_gql).collect()),
        serde_json::Value::Object(map) => GqlValue::Object(
            map.into_iter()
                .map(|(k, v)| (async_graphql::Name::new(k), json_to_gql(v)))
                .collect(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case() {
        assert_eq!(pascal_case("user_profile"), "UserProfile");
        assert_eq!(pascal_case("order"), "Order");
        assert_eq!(pascal_case("api_name_test"), "ApiNameTest");
    }

    #[test]
    fn test_build_empty_schema() {
        let spec = Spec::default();
        let rt_spec = spec.clone();
        let opts = tesela_runtime::runtime::RuntimeOptions::dev();
        let rt = Runtime::new(rt_spec, opts).unwrap();
        let schema = GraphQLSchemaBuilder::build(&spec, rt);
        assert!(schema.is_ok(), "schema build failed: {:?}", schema.err());
    }
}
