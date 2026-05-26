//! Axum route handler functions for all Tesela HTTP endpoints.

use crate::types::{ApiError, AppState};
use axum::{
    Json,
    extract::{Path, Query as AxumQuery, State},
    http::HeaderMap,
    response::sse::{Event as SseEvent, Sse},
};
use futures_util::Stream;
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{AggregateResult, Branch, MutationResult, Page, PipelineResult, Record, Spec};
use tesela_runtime::{
    ports::{FederatedQuery, LineageRecord, VectorResult, VectorSearchQuery},
    query::{
        Actor, AggregateQuery, ArtifactLocator, CapabilityToken, Mutation, ObjectMetadata, Query,
        RequestMeta, RunRecord, SignedUpload, Sort, TraversalQuery,
    },
};
use std::collections::BTreeMap;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;

pub(crate) fn request_meta(headers: &HeaderMap) -> RequestMeta {
    RequestMeta {
        authorization: headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        headers: headers
            .iter()
            .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
            .collect(),
        remote_addr: None,
        workspace: headers
            .get("x-tesela-workspace")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
        correlation_id: headers
            .get("x-correlation-id")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string),
    }
}

pub(crate) fn extract_actor(state: &AppState, headers: &HeaderMap) -> Result<Actor, Error> {
    let meta = request_meta(headers);

    match &state.actor_resolver {
        Some(resolver) => resolver.resolve(&meta),
        None => Err(Error::unauthorized("actor resolver is required")),
    }
}

fn capability_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-tesela-capability")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string)
}

pub(crate) fn parse_api_name(s: &str) -> Result<ApiName, Error> {
    s.parse::<ApiName>()
        .map_err(|e| Error::bad_request(format!("invalid name '{}': {}", s, e)))
}

// -- Request body types ------------------------------------------------------

#[derive(serde::Deserialize)]
pub(crate) struct SearchBody {
    #[serde(default)]
    pub filter: Option<tesela_ir::Filter>,
    #[serde(default)]
    pub sort: Vec<Sort>,
    #[serde(default)]
    pub limit: Option<i32>,
    #[serde(default)]
    pub offset: Option<i32>,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct RollbackBody {
    pub load_id: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct UploadBody {
    #[serde(default = "default_upload_format")]
    pub format: String,
    #[serde(default = "default_upload_ttl_secs")]
    pub ttl_secs: u64,
}

#[derive(serde::Deserialize)]
pub(crate) struct OperationalParamsBody {
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
    #[serde(default = "default_upload_ttl_secs")]
    pub ttl_secs: u64,
}

#[derive(serde::Deserialize)]
pub(crate) struct CapabilityIssueBody {
    #[serde(default)]
    pub constraints: BTreeMap<String, Value>,
}

#[derive(serde::Deserialize)]
pub(crate) struct JobStartBody {
    #[serde(default)]
    pub input: BTreeMap<String, Value>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct UploadCompleteBody {
    pub path: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct UploadLoadBody {
    #[serde(default)]
    pub records: Vec<Record>,
    #[serde(default)]
    pub load_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct ActionBody {
    #[serde(default)]
    pub input: serde_json::Value,
}

#[derive(serde::Deserialize)]
pub(crate) struct AgentStartBody {
    #[serde(default)]
    pub input: serde_json::Value,
}

#[derive(serde::Deserialize)]
pub(crate) struct TraverseBody {
    pub source_pk: serde_json::Value,
    #[serde(default)]
    pub filter: Option<tesela_ir::Filter>,
    #[serde(default)]
    pub sort: Vec<Sort>,
    #[serde(default)]
    pub limit: Option<i32>,
    #[serde(default)]
    pub offset: Option<i32>,
}

#[derive(serde::Deserialize)]
pub(crate) struct VectorSearchBody {
    pub query_vector: Vec<f32>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_ef")]
    pub ef: usize,
    #[serde(default)]
    pub filter: Option<tesela_ir::Filter>,
}

fn default_top_k() -> usize {
    10
}
fn default_ef() -> usize {
    50
}
fn default_upload_format() -> String {
    "csv".to_string()
}
fn default_upload_ttl_secs() -> u64 {
    900
}

#[derive(serde::Deserialize)]
pub(crate) struct LineageParams {
    #[serde(default)]
    pub depth: Option<u32>,
}

#[derive(serde::Deserialize)]
pub(crate) struct ComposeBody {
    pub names: Vec<String>,
    #[serde(default)]
    pub op: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct PipelineExecuteBody {
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct FederatedSearchBody {
    pub queries: Vec<FederatedQueryBody>,
}

#[derive(serde::Deserialize)]
pub(crate) struct FederatedQueryBody {
    pub datasource: String,
    pub object_type: String,
    #[serde(default)]
    pub filter: Option<tesela_ir::Filter>,
    #[serde(default)]
    pub sort: Vec<Sort>,
    #[serde(default)]
    pub limit: Option<i32>,
}

#[derive(serde::Deserialize)]
pub(crate) struct BranchCreateBody {
    pub display: String,
}

// -- Object handlers ---------------------------------------------------------

pub(crate) async fn search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(body): Json<SearchBody>,
) -> Result<Json<Page>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    let query = Query {
        filter: body.filter,
        sort: body.sort,
        limit: body.limit,
        offset: body.offset,
        cursor: body.cursor,
    };
    Ok(Json(state.runtime.search(&actor, &obj, query)?))
}

pub(crate) async fn get_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((type_name, pk)): Path<(String, String)>,
) -> Result<Json<Record>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    let pk_val = Value::string(pk);
    Ok(Json(state.runtime.get(&actor, &obj, &pk_val)?))
}

pub(crate) async fn mutate_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(mutation): Json<Mutation>,
) -> Result<Json<MutationResult>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    Ok(Json(state.runtime.mutate(&actor, &obj, mutation)?))
}

pub(crate) async fn aggregate_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(query): Json<AggregateQuery>,
) -> Result<Json<AggregateResult>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    Ok(Json(state.runtime.aggregate(&actor, &obj, query)?))
}

pub(crate) async fn upload_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(body): Json<UploadBody>,
) -> Result<Json<SignedUpload>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    Ok(Json(state.runtime.initiate_upload(
        &actor,
        &obj,
        &body.format,
        body.ttl_secs,
    )?))
}

pub(crate) async fn upload_flow_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<OperationalParamsBody>,
) -> Result<Json<SignedUpload>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let flow = parse_api_name(&name)?;
    Ok(Json(state.runtime.initiate_upload_flow_with_context(
        &actor,
        &flow,
        body.params,
        body.ttl_secs,
        Some(request_meta(&headers)),
        capability_header(&headers),
    )?))
}

pub(crate) async fn upload_flow_complete_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<UploadCompleteBody>,
) -> Result<Json<ObjectMetadata>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let flow = parse_api_name(&name)?;
    Ok(Json(
        state
            .runtime
            .complete_upload_flow(&actor, &flow, &body.path)?,
    ))
}

pub(crate) async fn upload_flow_load_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<UploadLoadBody>,
) -> Result<Json<tesela_ir::UploadResult>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let flow = parse_api_name(&name)?;
    Ok(Json(state.runtime.load_upload_flow_records(
        &actor,
        &flow,
        body.records,
        body.load_id,
    )?))
}

pub(crate) async fn upload_flow_rollback_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<RollbackBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let flow = parse_api_name(&name)?;
    state
        .runtime
        .rollback_upload_flow(&actor, &flow, &body.load_id)?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub(crate) async fn artifact_read_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<OperationalParamsBody>,
) -> Result<Json<ArtifactLocator>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let artifact = parse_api_name(&name)?;
    Ok(Json(state.runtime.authorize_artifact_read_with_context(
        &actor,
        &artifact,
        body.params,
        body.ttl_secs,
        Some(request_meta(&headers)),
        capability_header(&headers),
    )?))
}

pub(crate) async fn capability_issue_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<CapabilityIssueBody>,
) -> Result<Json<CapabilityToken>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let grant = parse_api_name(&name)?;
    Ok(Json(state.runtime.issue_capability(
        &actor,
        &grant,
        body.constraints,
    )?))
}

pub(crate) async fn job_start_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<JobStartBody>,
) -> Result<Json<RunRecord>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let job = parse_api_name(&name)?;
    Ok(Json(state.runtime.start_job(
        &actor,
        &job,
        body.input,
        body.idempotency_key,
    )?))
}

pub(crate) async fn run_get_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
) -> Result<Json<RunRecord>, ApiError> {
    let _actor = extract_actor(&state, &headers)?;
    Ok(Json(state.runtime.get_run(&run_id)?))
}

pub(crate) async fn aggregate_view_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<AggregateResult>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let view = parse_api_name(&name)?;
    Ok(Json(state.runtime.aggregate_view(&actor, &view)?))
}

pub(crate) async fn rollback_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(body): Json<RollbackBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    state.runtime.rollback_upload(&actor, &obj, &body.load_id)?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub(crate) async fn explain_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(body): Json<SearchBody>,
) -> Result<Json<tesela_ir::ExplainPlan>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    let query = Query {
        filter: body.filter,
        sort: body.sort,
        limit: body.limit,
        offset: body.offset,
        cursor: body.cursor,
    };
    Ok(Json(state.runtime.explain(&actor, &obj, query)?))
}

// -- Action / agent handlers -------------------------------------------------

pub(crate) async fn action_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<ActionBody>,
) -> Result<Json<tesela_ir::ActionResult>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let action = parse_api_name(&name)?;
    let input = Value::new(body.input);
    Ok(Json(state.runtime.execute_action(&actor, &action, input)?))
}

pub(crate) async fn agent_start_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<AgentStartBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let agent_name = parse_api_name(&name)?;
    let run_id = state
        .runtime
        .start_agent_run(&actor, &agent_name, Value::new(body.input))?;
    Ok(Json(serde_json::json!({ "run_id": run_id })))
}

pub(crate) async fn agent_get_run_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_name, run_id)): Path<(String, String)>,
) -> Result<Json<tesela_ir::AgentRun>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    Ok(Json(state.runtime.get_agent_run(&actor, run_id.as_str())?))
}

// -- Link / traverse handlers ------------------------------------------------

pub(crate) async fn traverse_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<TraverseBody>,
) -> Result<Json<Page>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let link_name = parse_api_name(&name)?;
    let query = TraversalQuery {
        source_pk: Value::new(body.source_pk),
        filter: body.filter,
        sort: body.sort,
        limit: body.limit,
        offset: body.offset,
    };
    Ok(Json(state.runtime.traverse(&actor, &link_name, query)?))
}

pub(crate) async fn explain_traverse_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<TraverseBody>,
) -> Result<Json<tesela_ir::ExplainPlan>, ApiError> {
    let _actor = extract_actor(&state, &headers)?;
    let _ = name;
    let _ = body;
    let steps = vec![{
        let mut m = BTreeMap::new();
        m.insert("op".to_string(), Value::string("traverse"));
        m
    }];
    Ok(Json(tesela_ir::ExplainPlan { steps }))
}

// -- Subscribe (SSE) ---------------------------------------------------------

pub(crate) async fn subscribe_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>, ApiError> {
    let _actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;

    let sync_rx = state
        .runtime
        .subscribe(Some(&obj))
        .map_err(ApiError::from)?;

    let (tx, rx_async) = tokio::sync::mpsc::channel::<Result<SseEvent, Infallible>>(256);

    tokio::task::spawn_blocking(move || {
        while let Ok(event) = sync_rx.recv() {
            let data = match serde_json::to_string(&event) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to serialize SSE event, skipping");
                    continue;
                }
            };
            let sse = SseEvent::default().data(data);
            if tx.blocking_send(Ok(sse)).is_err() {
                break;
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx_async)))
}

// -- Vector search -----------------------------------------------------------

pub(crate) async fn vector_search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(type_name): Path<String>,
    Json(body): Json<VectorSearchBody>,
) -> Result<Json<Vec<VectorResult>>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    let query = VectorSearchQuery {
        object_type: obj,
        query_vector: body.query_vector,
        top_k: body.top_k,
        ef: body.ef,
        filter: body.filter,
    };
    Ok(Json(state.runtime.vector_search(&actor, query)?))
}

// -- Lineage -----------------------------------------------------------------

pub(crate) async fn lineage_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((type_name, pk)): Path<(String, String)>,
    AxumQuery(params): AxumQuery<LineageParams>,
) -> Result<Json<Vec<LineageRecord>>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let obj = parse_api_name(&type_name)?;
    let pk_val = Value::string(pk);
    Ok(Json(state.runtime.get_lineage(
        &actor,
        &obj,
        &pk_val,
        params.depth,
    )?))
}

// -- Object sets -------------------------------------------------------------

pub(crate) async fn object_set_resolve_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<Page>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let set_name = parse_api_name(&name)?;
    Ok(Json(state.runtime.resolve_object_set(&actor, &set_name)?))
}

pub(crate) async fn object_set_compose_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_name): Path<String>,
    Json(body): Json<ComposeBody>,
) -> Result<Json<Page>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let names: Result<Vec<ApiName>, Error> = body.names.iter().map(|n| parse_api_name(n)).collect();
    let op = match body.op.as_deref() {
        Some("intersect") => tesela_ir::SetOp::Intersect,
        Some("subtract") => tesela_ir::SetOp::Subtract,
        _ => tesela_ir::SetOp::Union,
    };
    Ok(Json(
        state.runtime.compose_object_sets(&actor, &names?, op)?,
    ))
}

// -- Pipelines ---------------------------------------------------------------

pub(crate) async fn pipeline_execute_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<PipelineExecuteBody>,
) -> Result<Json<PipelineResult>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let pipeline_name = parse_api_name(&name)?;
    let mode = match body.mode.as_deref() {
        Some("snapshot") => tesela_ir::ExecutionMode::Snapshot,
        _ => tesela_ir::ExecutionMode::Incremental,
    };
    Ok(Json(state.runtime.execute_pipeline(
        &actor,
        &pipeline_name,
        mode,
    )?))
}

// -- Federated search --------------------------------------------------------

pub(crate) async fn federated_search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<FederatedSearchBody>,
) -> Result<Json<Page>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let queries: Result<Vec<FederatedQuery>, Error> = body
        .queries
        .into_iter()
        .map(|q| {
            Ok(FederatedQuery {
                datasource: parse_api_name(&q.datasource)?,
                object_type: parse_api_name(&q.object_type)?,
                query: Query {
                    filter: q.filter,
                    sort: q.sort,
                    limit: q.limit,
                    offset: None,
                    cursor: None,
                },
            })
        })
        .collect();
    Ok(Json(state.runtime.cross_search(&actor, queries?)?))
}

// -- Branches ----------------------------------------------------------------

pub(crate) async fn branch_create_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BranchCreateBody>,
) -> Result<Json<Branch>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    Ok(Json(state.runtime.create_branch(&actor, &body.display)?))
}

pub(crate) async fn branch_list_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Branch>>, ApiError> {
    let _actor = extract_actor(&state, &headers)?;
    Ok(Json(state.runtime.list_branches()?))
}

pub(crate) async fn branch_update_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(spec): Json<Spec>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    state.runtime.update_branch_spec(&actor, &id, spec)?;
    Ok(Json(serde_json::json!({ "status": "updated" })))
}

pub(crate) async fn branch_merge_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    let diff = state.runtime.merge_branch(&actor, &id)?;
    Ok(Json(serde_json::json!({
        "status": "merged",
        "added": diff.added.len(),
        "removed": diff.removed.len(),
        "changed": diff.changed.len(),
    })))
}

pub(crate) async fn branch_discard_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = extract_actor(&state, &headers)?;
    state.runtime.discard_branch(&actor, &id)?;
    Ok(Json(serde_json::json!({ "status": "discarded" })))
}

// -- Ontology & system -------------------------------------------------------

pub(crate) async fn spec_handler(State(state): State<AppState>) -> Result<Json<Spec>, ApiError> {
    Ok(Json(state.runtime.spec()?))
}

pub(crate) async fn apply_spec_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(spec): Json<Spec>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _actor = extract_actor(&state, &headers)?;
    state.runtime.apply_spec(spec)?;
    Ok(Json(serde_json::json!({ "status": "applied" })))
}

pub(crate) async fn capabilities_handler(
    State(state): State<AppState>,
) -> Json<tesela_ir::Capabilities> {
    Json(state.runtime.capabilities())
}

pub(crate) async fn health_handler(
    State(state): State<AppState>,
) -> Result<Json<tesela_ir::HealthStatus>, ApiError> {
    Ok(Json(state.runtime.health()?))
}
