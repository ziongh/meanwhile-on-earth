use std::{
    convert::Infallible,
    env,
    error::Error,
    net::SocketAddr,
    pin::Pin,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use historyworld_content::CompiledRelease;
use historyworld_domain::{
    DetailLevel, LayerKind, LayerMask, ResourceBudget, WorldCell, WorldInstant,
};
use historyworld_sim::{Materializer, SimulationError, SnapshotRequest};
use http_body::{Body as HttpBody, Frame, SizeHint};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::timeout;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const DEMO_RELEASE: &str = include_str!("../../../data/fixtures/vesuvius-architecture-demo.json");
const DEMO_CELL: &str = "0:14:ee085cd";

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .compact()
        .init();

    let config = Config::from_environment()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads)
        .enable_io()
        .enable_time()
        .build()?;
    runtime.block_on(serve(config))
}

async fn serve(config: Config) -> Result<(), Box<dyn Error>> {
    let release = CompiledRelease::from_json(DEMO_RELEASE)?;
    let state = AppState {
        materializer: Arc::new(Materializer::new(release)),
        budget_ceiling: config.budget_ceiling,
        compute_permits: Arc::new(Semaphore::new(config.max_compute_tasks)),
        snapshot_timeout: config.snapshot_timeout,
        metrics: Arc::new(Metrics::default()),
    };
    let app = application(state);

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    info!(
        bind = %config.bind,
        worker_threads = config.worker_threads,
        max_compute_tasks = config.max_compute_tasks,
        snapshot_timeout_ms = config.snapshot_timeout.as_millis(),
        max_response_bytes = config.budget_ceiling.max_output_bytes,
        "HistoryWorld read service started"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn application(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/metrics", get(metrics))
        .route("/v1/releases/current", get(current_manifest))
        .route("/v1/releases/{release_id}/manifest", get(pinned_manifest))
        .route("/v1/releases/{release_id}/snapshot", get(snapshot))
        .with_state(state)
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(%error, "failed to install Ctrl+C handler");
    }
}

#[derive(Clone, Debug)]
struct Config {
    bind: SocketAddr,
    worker_threads: usize,
    max_compute_tasks: usize,
    snapshot_timeout: Duration,
    budget_ceiling: ResourceBudget,
}

impl Config {
    fn from_environment() -> Result<Self, Box<dyn Error>> {
        let available = std::thread::available_parallelism().map_or(1, usize::from);
        let worker_threads = environment_value("HW_WORKER_THREADS")?
            .unwrap_or(available.min(4))
            .max(1);
        let max_compute_tasks = environment_value("HW_MAX_COMPUTE_TASKS")?
            .unwrap_or(available.min(2))
            .max(1);
        let snapshot_timeout = Duration::from_millis(
            environment_value("HW_SNAPSHOT_TIMEOUT_MS")?
                .unwrap_or(5_000_u64)
                .max(100),
        );
        let defaults = ResourceBudget::INTERACTIVE;
        let budget_ceiling = ResourceBudget::new(
            environment_value("HW_MAX_CANDIDATE_PATCHES")?
                .unwrap_or(defaults.max_candidate_patches),
            environment_value("HW_MAX_RESPONSE_BYTES")?.unwrap_or(defaults.max_output_bytes),
            environment_value("HW_MAX_GENERATED_INSTANCES")?
                .unwrap_or(defaults.max_generated_instances),
            environment_value("HW_MAX_WORK_UNITS")?.unwrap_or(defaults.max_work_units),
        )?
        .clamp_to(ResourceBudget::SERVER_CEILING);
        Ok(Self {
            bind: env::var("HW_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_owned())
                .parse()?,
            worker_threads,
            max_compute_tasks,
            snapshot_timeout,
            budget_ceiling,
        })
    }
}

fn environment_value<T>(name: &str) -> Result<Option<T>, Box<dyn Error>>
where
    T: FromStr,
    T::Err: Error + 'static,
{
    env::var(name)
        .ok()
        .map(|value| value.parse::<T>().map_err(Into::into))
        .transpose()
}

#[derive(Clone, Debug)]
struct AppState {
    materializer: Arc<Materializer>,
    budget_ceiling: ResourceBudget,
    compute_permits: Arc<Semaphore>,
    snapshot_timeout: Duration,
    metrics: Arc<Metrics>,
}

#[derive(Debug, Default)]
struct Metrics {
    requests: AtomicU64,
    errors: AtomicU64,
    snapshots: AtomicU64,
    candidate_patches: AtomicU64,
    work_units: AtomicU64,
    response_bytes: AtomicU64,
}

#[derive(Debug, Serialize)]
struct Health<'a> {
    status: &'a str,
    release_id: String,
}

async fn health(State(state): State<AppState>) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(Health {
            status: "ok",
            release_id: state
                .materializer
                .release()
                .manifest()
                .release_id
                .to_string(),
        }),
    )
        .into_response()
}

async fn metrics(State(state): State<AppState>) -> Response {
    let metrics = &state.metrics;
    let body = format!(
        concat!(
            "# TYPE historyworld_requests_total counter\n",
            "historyworld_requests_total {}\n",
            "# TYPE historyworld_errors_total counter\n",
            "historyworld_errors_total {}\n",
            "# TYPE historyworld_snapshots_total counter\n",
            "historyworld_snapshots_total {}\n",
            "# TYPE historyworld_candidate_patches_total counter\n",
            "historyworld_candidate_patches_total {}\n",
            "# TYPE historyworld_work_units_total counter\n",
            "historyworld_work_units_total {}\n",
            "# TYPE historyworld_response_bytes_total counter\n",
            "historyworld_response_bytes_total {}\n",
            "# EOF\n"
        ),
        metrics.requests.load(Ordering::Relaxed),
        metrics.errors.load(Ordering::Relaxed),
        metrics.snapshots.load(Ordering::Relaxed),
        metrics.candidate_patches.load(Ordering::Relaxed),
        metrics.work_units.load(Ordering::Relaxed),
        metrics.response_bytes.load(Ordering::Relaxed),
    );
    (
        [
            (
                header::CONTENT_TYPE,
                "application/openmetrics-text; version=1.0.0",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
}

async fn current_manifest(State(state): State<AppState>) -> Result<Response, ApiError> {
    state.metrics.requests.fetch_add(1, Ordering::Relaxed);
    let body = serde_json::to_vec(state.materializer.release().manifest())?;
    ensure_response_size(&state, body.len())?;
    response_with_cache(body, None, "no-cache")
}

async fn pinned_manifest(
    State(state): State<AppState>,
    Path(release_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    state.metrics.requests.fetch_add(1, Ordering::Relaxed);
    verify_release(&state, &release_id)?;
    let body = serde_json::to_vec(state.materializer.release().manifest())?;
    ensure_response_size(&state, body.len())?;
    let etag = quoted_etag(blake3::hash(&body).to_hex().as_ref());
    immutable_response(body, &headers, etag)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotQuery {
    cell: Option<String>,
    at_seconds: Option<String>,
    nanos: Option<u32>,
    year: Option<i64>,
    month: Option<u8>,
    day: Option<u8>,
    detail: Option<DetailLevel>,
    layers: Option<String>,
    max_candidate_patches: Option<u32>,
    max_output_bytes: Option<u32>,
    max_instances: Option<u32>,
    max_work_units: Option<u64>,
}

async fn snapshot(
    State(state): State<AppState>,
    Path(release_id): Path<String>,
    query: Result<Query<SnapshotQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    state.metrics.requests.fetch_add(1, Ordering::Relaxed);
    let Query(query) = query.map_err(|error| {
        state.metrics.errors.fetch_add(1, Ordering::Relaxed);
        ApiError::bad_request(error.to_string())
    })?;
    verify_release(&state, &release_id).inspect_err(|_| {
        state.metrics.errors.fetch_add(1, Ordering::Relaxed);
    })?;
    let permit = Arc::clone(&state.compute_permits)
        .try_acquire_owned()
        .map_err(|_| {
            state.metrics.errors.fetch_add(1, Ordering::Relaxed);
            ApiError::service_unavailable("snapshot compute capacity is saturated")
        })?;
    let compute_state = state.clone();
    let computation = tokio::task::spawn_blocking(move || {
        let result = build_snapshot(&compute_state, query)?;
        let body = serde_json::to_vec(&result)?;
        ensure_response_size(&compute_state, body.len())?;
        if u64::try_from(body.len()).unwrap_or(u64::MAX)
            > result.metrics.conservative_output_bound_bytes
        {
            return Err(ApiError::internal(
                "snapshot encoder exceeded its conservative output bound",
            ));
        }
        Ok::<_, ApiError>((result, body, permit))
    });
    let (result, body, permit) = timeout(state.snapshot_timeout, computation)
        .await
        .map_err(|_| {
            state.metrics.errors.fetch_add(1, Ordering::Relaxed);
            ApiError::gateway_timeout("snapshot computation exceeded its wall-time budget")
        })?
        .map_err(ApiError::internal)?
        .inspect_err(|_| {
            state.metrics.errors.fetch_add(1, Ordering::Relaxed);
        })?;
    state.metrics.snapshots.fetch_add(1, Ordering::Relaxed);
    state.metrics.candidate_patches.fetch_add(
        u64::from(result.metrics.candidate_patches),
        Ordering::Relaxed,
    );
    state
        .metrics
        .work_units
        .fetch_add(result.metrics.work_units, Ordering::Relaxed);

    state.metrics.response_bytes.fetch_add(
        u64::try_from(body.len()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    let etag = quoted_etag(blake3::hash(&body).to_hex().as_ref());
    revalidating_snapshot_response(body, &headers, etag, permit)
}

fn build_snapshot(
    state: &AppState,
    query: SnapshotQuery,
) -> Result<historyworld_sim::WorldSnapshot, ApiError> {
    let cell = query
        .cell
        .as_deref()
        .unwrap_or(DEMO_CELL)
        .parse::<WorldCell>()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let at = if let Some(seconds) = query.at_seconds {
        if query.year.is_some() || query.month.is_some() || query.day.is_some() {
            return Err(ApiError::bad_request(
                "at_seconds cannot be combined with calendar fields".to_owned(),
            ));
        }
        WorldInstant::new(
            seconds
                .parse()
                .map_err(|error| ApiError::bad_request(format!("invalid at_seconds: {error}")))?,
            query.nanos.unwrap_or(0),
        )
        .map_err(|error| ApiError::bad_request(error.to_string()))?
    } else {
        if query.nanos.is_some() {
            return Err(ApiError::bad_request(
                "nanos requires at_seconds".to_owned(),
            ));
        }
        WorldInstant::from_ymd_hms(
            query.year.unwrap_or(80),
            query.month.unwrap_or(1),
            query.day.unwrap_or(1),
            0,
            0,
            0,
        )
        .map_err(|error| ApiError::bad_request(error.to_string()))?
    };
    let defaults = ResourceBudget::INTERACTIVE;
    let requested_budget = ResourceBudget::new(
        query
            .max_candidate_patches
            .unwrap_or(defaults.max_candidate_patches),
        query.max_output_bytes.unwrap_or(defaults.max_output_bytes),
        query
            .max_instances
            .unwrap_or(defaults.max_generated_instances),
        query.max_work_units.unwrap_or(defaults.max_work_units),
    )
    .map_err(|error| ApiError::bad_request(error.to_string()))?
    .clamp_to(state.budget_ceiling);
    let layers = parse_layers(query.layers.as_deref())?;

    state
        .materializer
        .snapshot(SnapshotRequest {
            cell,
            at,
            layers,
            requested_detail: query.detail.unwrap_or(DetailLevel::Local),
            budget: requested_budget,
        })
        .map_err(|error| ApiError::from_simulation(&error))
}

fn parse_layers(value: Option<&str>) -> Result<LayerMask, ApiError> {
    let Some(value) = value else {
        return Ok(LayerMask::ALL);
    };
    let mut layers = LayerMask::NONE;
    for name in value.split(',') {
        let layer = match name.trim() {
            "geology" => LayerKind::Geology,
            "hydrology" => LayerKind::Hydrology,
            "climate" => LayerKind::Climate,
            "biome" => LayerKind::Biome,
            "population" => LayerKind::Population,
            "settlement" => LayerKind::Settlement,
            "political" => LayerKind::Political,
            "event" => LayerKind::Event,
            "" => {
                return Err(ApiError::bad_request(
                    "layers must contain at least one layer".to_owned(),
                ));
            }
            unknown => {
                return Err(ApiError::bad_request(format!("unknown layer '{unknown}'")));
            }
        };
        if layers.contains(layer) {
            return Err(ApiError::bad_request(format!(
                "duplicate layer '{}'",
                name.trim()
            )));
        }
        layers = layers.union(LayerMask::only(layer));
    }
    Ok(layers)
}

fn verify_release(state: &AppState, requested: &str) -> Result<(), ApiError> {
    let current = state
        .materializer
        .release()
        .manifest()
        .release_id
        .to_string();
    if requested != current {
        return Err(ApiError::not_found(format!(
            "release '{requested}' is not loaded; current release is '{current}'"
        )));
    }
    Ok(())
}

fn immutable_response(
    body: Vec<u8>,
    request_headers: &HeaderMap,
    etag: HeaderValue,
) -> Result<Response, ApiError> {
    if matches_if_none_match(request_headers, &etag) {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .body(Body::empty())
            .map_err(ApiError::internal);
    }
    response_with_cache(body, Some(etag), "public, max-age=31536000, immutable")
}

fn revalidating_snapshot_response(
    body: Vec<u8>,
    request_headers: &HeaderMap,
    etag: HeaderValue,
    permit: OwnedSemaphorePermit,
) -> Result<Response, ApiError> {
    const CACHE_POLICY: &str = "private, no-cache";
    if matches_if_none_match(request_headers, &etag) {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::CACHE_CONTROL, CACHE_POLICY)
            .body(Body::empty())
            .map_err(ApiError::internal);
    }
    let body = Body::new(PermitBody {
        remaining: Bytes::from(body),
        _permit: permit,
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, CACHE_POLICY)
        .header(header::ETAG, etag)
        .body(body)
        .map_err(ApiError::internal)
}

/// Retains a compute admission permit until the HTTP body is drained/dropped,
/// bounding completed snapshot residency under slow clients as well as CPU.
#[derive(Debug)]
struct PermitBody {
    remaining: Bytes,
    _permit: OwnedSemaphorePermit,
}

impl HttpBody for PermitBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        const CHUNK_BYTES: usize = 64 * 1024;
        let this = self.get_mut();
        if this.remaining.is_empty() {
            return Poll::Ready(None);
        }
        let length = this.remaining.len().min(CHUNK_BYTES);
        Poll::Ready(Some(Ok(Frame::data(this.remaining.split_to(length)))))
    }

    fn is_end_stream(&self) -> bool {
        self.remaining.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(u64::try_from(self.remaining.len()).unwrap_or(u64::MAX))
    }
}

fn ensure_response_size(state: &AppState, actual: usize) -> Result<(), ApiError> {
    let limit = state.budget_ceiling.max_output_bytes as usize;
    if actual > limit {
        return Err(ApiError::response_too_large(actual, limit));
    }
    Ok(())
}

fn response_with_cache(
    body: Vec<u8>,
    etag: Option<HeaderValue>,
    cache_control: &'static str,
) -> Result<Response, ApiError> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, cache_control);
    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }
    builder.body(Body::from(body)).map_err(ApiError::internal)
}

fn quoted_etag(value: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{value}\""))
        .expect("hexadecimal content hashes always form valid ETag headers")
}

fn matches_if_none_match(headers: &HeaderMap, etag: &HeaderValue) -> bool {
    let Some(candidate_list) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Ok(expected) = etag.to_str() else {
        return false;
    };
    candidate_list.split(',').any(|candidate| {
        let candidate = candidate.trim();
        candidate == "*" || candidate == expected || candidate.strip_prefix("W/") == Some(expected)
    })
}

#[derive(Debug, Error)]
#[error("{message}")]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message,
        }
    }

    fn not_found(message: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "release_not_found",
            message,
        }
    }

    fn response_too_large(actual: usize, limit: usize) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "response_budget_exceeded",
            message: format!("encoded response is {actual} bytes, above the {limit}-byte limit"),
        }
    }

    fn service_unavailable(message: &'static str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "service_saturated",
            message: message.to_owned(),
        }
    }

    fn gateway_timeout(message: &'static str) -> Self {
        Self {
            status: StatusCode::GATEWAY_TIMEOUT,
            code: "compute_timeout",
            message: message.to_owned(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: error.to_string(),
        }
    }

    fn from_simulation(error: &SimulationError) -> Self {
        let (status, code) = match error {
            SimulationError::PatchBudgetExceeded { .. }
            | SimulationError::WorkBudgetExceeded { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, "work_budget_exceeded")
            }
            SimulationError::OutputBudgetExceeded { .. } => {
                (StatusCode::PAYLOAD_TOO_LARGE, "response_budget_exceeded")
            }
            SimulationError::Domain(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            SimulationError::Content(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "materialization_failed")
            }
        };
        Self {
            status,
            code,
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self::internal(error)
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: &self.message,
            }),
        )
            .into_response();
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use http::Request;
    use tower::ServiceExt;

    use super::*;

    fn test_state(instance_ceiling: u32) -> AppState {
        let mut budget = ResourceBudget::INTERACTIVE;
        budget.max_generated_instances = instance_ceiling;
        AppState {
            materializer: Arc::new(Materializer::new(
                CompiledRelease::from_json(DEMO_RELEASE).unwrap(),
            )),
            budget_ceiling: budget,
            compute_permits: Arc::new(Semaphore::new(1)),
            snapshot_timeout: Duration::from_secs(1),
            metrics: Arc::new(Metrics::default()),
        }
    }

    fn query() -> SnapshotQuery {
        SnapshotQuery {
            cell: None,
            at_seconds: None,
            nanos: None,
            year: None,
            month: None,
            day: None,
            detail: None,
            layers: None,
            max_candidate_patches: None,
            max_output_bytes: None,
            max_instances: None,
            max_work_units: None,
        }
    }

    #[test]
    fn extreme_time_is_rejected_without_overflow_or_process_abort() {
        let mut extreme = query();
        extreme.at_seconds = Some(i64::MIN.to_string());
        let error = build_snapshot(&test_state(20_000), extreme).unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "invalid_request");
    }

    #[test]
    fn client_instance_budget_cannot_raise_server_ceiling() {
        let mut untrusted = query();
        untrusted.detail = Some(DetailLevel::Street);
        untrusted.max_instances = Some(u32::MAX);
        let snapshot = build_snapshot(&test_state(16), untrusted).unwrap();
        assert_eq!(snapshot.effective_detail, DetailLevel::Continental);
        assert_eq!(snapshot.metrics.suggested_instance_ceiling, 16);
    }

    #[test]
    fn etag_matching_handles_lists_wildcards_and_weak_validators() {
        let etag = quoted_etag("abc");
        for value in ["*", "\"other\", \"abc\"", "W/\"abc\""] {
            let mut headers = HeaderMap::new();
            headers.insert(header::IF_NONE_MATCH, value.parse().unwrap());
            assert!(matches_if_none_match(&headers, &etag));
        }
    }

    #[tokio::test]
    async fn router_turns_query_rejections_into_the_json_error_contract() {
        let state = test_state(20_000);
        let release_id = state.materializer.release().manifest().release_id;
        let response = application(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/v1/releases/{release_id}/snapshot?max_output_byte=1"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["code"], "invalid_request");
        assert_eq!(state.metrics.errors.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn computed_snapshots_are_revalidated_not_marked_immutable() {
        let state = test_state(20_000);
        let release_id = state.materializer.release().manifest().release_id;
        let response = application(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/releases/{release_id}/snapshot"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "private, no-cache"
        );
        let _ = to_bytes(response.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap();
    }
}
