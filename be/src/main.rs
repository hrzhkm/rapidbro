mod rapidkl;
#[path = "busmy-kangar.rs"]
mod busmy_kangar;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use base64::Engine;
use flate2::read::GzDecoder;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path as StdPath;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};

use rapidkl::{
    BusEta, BusMotionState, BusPosition, GtfsCache, RouteShapeResponse, RouteStopsResponse,
    StopRouteSummary, StopWithDetails, get_pantai_hillpark_phase_5_eta, get_route_t789,
    get_shape_by_route, get_t789_eta, is_bus_on_route, resolve_current_stop, run_bus_ingestor,
};

// ── Generic structs ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NearestStopQuery {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Deserialize)]
pub struct RouteShapeQuery {
    pub stop_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NearestStopResponse {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_desc: String,
    pub stop_lat: f64,
    pub stop_lon: f64,
    pub distance_km: f64,
    pub distance_meters: f64,
}

#[derive(Debug, Serialize)]
pub struct StopRoutesResponse {
    pub stop_id: String,
    pub routes: Vec<StopRouteSummary>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub redis_client: redis::Client,
    pub ingestor_status: Arc<RwLock<IngestorStatus>>,
    pub gtfs_cache: Arc<GtfsCache>,
    pub kangar_gtfs_cache: Arc<GtfsCache>,
    pub kangar_fetch_lock: Arc<Mutex<()>>,
    pub bus_ttl_ms: i64,
    pub stale_after_ms: i64,
    pub stationary_window_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestorStatus {
    pub connected: bool,
    pub reconnect_count: u64,
    pub messages_processed: u64,
    pub buses_written: u64,
    pub decode_failures: u64,
    pub redis_write_failures: u64,
    pub last_message_unix_ms: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetAllMeta {
    pub source: &'static str,
    pub last_ingest_at_unix_ms: Option<i64>,
    pub is_stale: bool,
    pub active_bus_count: usize,
}

#[derive(Debug, Serialize)]
pub struct GetAllResponse {
    pub data: Vec<BusPosition>,
    pub meta: GetAllMeta,
}

#[derive(Debug, Serialize)]
pub struct StopIncomingMeta {
    pub source: &'static str,
    pub generated_at_unix_ms: i64,
    pub last_ingest_at_unix_ms: Option<i64>,
    pub is_stale: bool,
    pub active_bus_count: usize,
    pub incoming_bus_count: usize,
    pub has_incoming_buses: bool,
}

#[derive(Debug, Serialize)]
pub struct StopIncomingResponse {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_desc: String,
    pub data: Vec<BusEta>,
    pub meta: StopIncomingMeta,
}

#[derive(Debug)]
pub struct RedisBusSnapshot {
    pub buses: Vec<BusPosition>,
    pub motion_states: HashMap<String, BusMotionState>,
    pub active_bus_count: usize,
    pub last_ingest_at_unix_ms: Option<i64>,
}

// ── Constants ─────────────────────────────────────────────────────────────────

pub const REDIS_BUSES_LATEST_KEY: &str = "rapidbro:buses:latest";
pub const REDIS_BUSES_LAST_SEEN_KEY: &str = "rapidbro:buses:last_seen";
pub const REDIS_BUSES_MOTION_KEY: &str = "rapidbro:buses:motion";
pub const REDIS_INGEST_LAST_KEY: &str = "rapidbro:ingestor:last_ingest_at";
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/";
const DEFAULT_BUS_TTL_SECONDS: i64 = 300;
const DEFAULT_STALE_AFTER_SECONDS: i64 = 20;
const DEFAULT_STATIONARY_WINDOW_SECONDS: i64 = 300;
pub const MAX_DERIVED_STOP_DISTANCE_KM: f64 = 0.75;
pub const STATIONARY_SPEED_THRESHOLD_KMH: f64 = 1.0;
pub const STATIONARY_DISTANCE_THRESHOLD_KM: f64 = 0.03;

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let bus_ttl_seconds = std::env::var("BUS_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_BUS_TTL_SECONDS);
    let stale_after_seconds = std::env::var("STALE_AFTER_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_STALE_AFTER_SECONDS);
    let stationary_window_seconds = std::env::var("STATIONARY_WINDOW_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_STATIONARY_WINDOW_SECONDS);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let redis_client = redis::Client::open(redis_url.clone()).unwrap_or_else(|error| {
        panic!(
            "Failed to create Redis client for '{}': {}",
            redis_url, error
        );
    });

    let mut redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .unwrap_or_else(|error| panic!("Failed to connect to Redis '{}': {}", redis_url, error));
    let _: String = redis::cmd("PING")
        .query_async(&mut redis_conn)
        .await
        .unwrap_or_else(|error| panic!("Failed to ping Redis '{}': {}", redis_url, error));
    let rapidkl_data_dir = StdPath::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/rapid-kl");
    let gtfs_cache = Arc::new(
        GtfsCache::build(&rapidkl_data_dir)
            .unwrap_or_else(|error| panic!("Failed to build RapidKL GTFS cache: {}", error)),
    );

    let kangar_data_dir = StdPath::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-kangar");
    let kangar_gtfs_cache = Arc::new(
        GtfsCache::build(&kangar_data_dir)
            .unwrap_or_else(|error| panic!("Failed to build Kangar GTFS cache: {}", error)),
    );

    let app_state = AppState {
        redis_client: redis_client.clone(),
        ingestor_status: Arc::new(RwLock::new(IngestorStatus {
            connected: false,
            reconnect_count: 0,
            messages_processed: 0,
            buses_written: 0,
            decode_failures: 0,
            redis_write_failures: 0,
            last_message_unix_ms: None,
            last_error: None,
        })),
        gtfs_cache,
        kangar_gtfs_cache,
        kangar_fetch_lock: Arc::new(Mutex::new(())),
        bus_ttl_ms: bus_ttl_seconds * 1_000,
        stale_after_ms: stale_after_seconds * 1_000,
        stationary_window_ms: stationary_window_seconds * 1_000,
    };

    let ingestor_state = app_state.clone();
    tokio::spawn(async move {
        run_bus_ingestor(ingestor_state).await;
    });

    let app = Router::new()
        .route("/gtfs", get(prasarana_gtfs_data))
        .route("/get-all", get(fetch_all_buses))
        .route("/ingestor/status", get(get_ingestor_status))
        .route("/get-route-t789", get(get_route_t789))
        .route("/get-t789-eta", get(get_t789_eta))
        .route(
            "/get-pantai-hillpark-phase-5-eta",
            get(get_pantai_hillpark_phase_5_eta),
        )
        .route("/route/{route_id}/eta/{stop_id}", get(get_route_eta))
        .route("/stops/{stop_id}/eta", get(get_stop_eta))
        .route("/stops/{stop_id}/routes", get(get_stop_routes))
        .route("/route/{route_id}/stops", get(get_route_stops))
        .route("/route/{route_id}/shape", get(get_route_shape))
        .route("/stops/nearest", get(get_nearest_stop))
        // ── Kangar routes ────────────────────────────────────────────────
        .route("/kangar/get-all", get(busmy_kangar::kangar_fetch_all_buses))
        .route(
            "/kangar/route/{route_id}/eta/{stop_id}",
            get(busmy_kangar::kangar_get_route_eta),
        )
        .route("/kangar/stops/{stop_id}/eta", get(busmy_kangar::kangar_get_stop_eta))
        .route(
            "/kangar/stops/{stop_id}/routes",
            get(busmy_kangar::kangar_get_stop_routes),
        )
        .route(
            "/kangar/route/{route_id}/stops",
            get(busmy_kangar::kangar_get_route_stops),
        )
        .route(
            "/kangar/route/{route_id}/shape",
            get(busmy_kangar::kangar_get_route_shape),
        )
        .route("/kangar/stops/nearest", get(busmy_kangar::kangar_get_nearest_stop))
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();

    println!("Server is running on http://localhost:3030");
    axum::serve(listener, app).await.unwrap();
}

// ── Generic HTTP handlers ─────────────────────────────────────────────────────

async fn fetch_all_buses(
    State(state): State<AppState>,
) -> Result<Json<GetAllResponse>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(&state).await?;
    let now_ms = now_unix_ms();
    let is_stale = match snapshot.last_ingest_at_unix_ms {
        Some(last_ingest_ms) => now_ms - last_ingest_ms > state.stale_after_ms,
        None => true,
    };

    println!(
        "Calling fetch_all_buses via Redis: {} active buses",
        snapshot.buses.len()
    );
    Ok(Json(GetAllResponse {
        data: snapshot.buses,
        meta: GetAllMeta {
            source: "redis",
            last_ingest_at_unix_ms: snapshot.last_ingest_at_unix_ms,
            is_stale,
            active_bus_count: snapshot.active_bus_count,
        },
    }))
}

async fn get_ingestor_status(State(state): State<AppState>) -> Json<IngestorStatus> {
    Json(state.ingestor_status.read().await.clone())
}

async fn get_route_eta(
    Path((route_id, stop_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    let eta_results = calculate_route_eta(&state, &route_id, &stop_id).await?;
    println!(
        "Calling get_route_eta for route_id={}, stop_id={}: {} buses",
        route_id,
        stop_id,
        eta_results.len()
    );
    Ok(Json(eta_results))
}

async fn get_stop_eta(
    Path(stop_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(&state).await?;
    let all_eta_results = calculate_stop_eta_from_snapshot(
        &snapshot,
        state.gtfs_cache.as_ref(),
        &stop_id,
        state.stationary_window_ms,
    );

    println!(
        "Calling get_stop_eta for stop_id={}: {} incoming buses",
        stop_id,
        all_eta_results.len()
    );
    Ok(Json(all_eta_results))
}

async fn get_stop_routes(
    Path(stop_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<StopRoutesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let routes = get_routes_for_stop_from_cache(&stop_id, state.gtfs_cache.as_ref())
        .map_err(|(status, message)| (status, Json(ErrorResponse { error: message })))?;

    println!(
        "Calling get_stop_routes for stop_id={}: {} routes",
        stop_id,
        routes.len()
    );

    Ok(Json(StopRoutesResponse { stop_id, routes }))
}

async fn get_route_stops(
    Path(route_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RouteStopsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match get_route_stops_from_cache(&route_id, state.gtfs_cache.as_ref()) {
        Ok(response) => {
            println!("Calling get_route_stops for route_id={}", route_id);
            Ok(Json(response))
        }
        Err((status, message)) => Err((status, Json(ErrorResponse { error: message }))),
    }
}

async fn get_route_shape(
    Path(route_id): Path<String>,
    Query(query): Query<RouteShapeQuery>,
    State(state): State<AppState>,
) -> Result<Json<RouteShapeResponse>, (StatusCode, Json<ErrorResponse>)> {
    match get_shape_by_route(
        &route_id,
        query.stop_id.as_deref(),
        &state.gtfs_cache.context.routes,
        &state.gtfs_cache.context.trips_by_route,
        &state.gtfs_cache.context.stop_times_by_trip,
        &state.gtfs_cache.shapes_by_id,
    ) {
        Ok(response) => Ok(Json(response)),
        Err((status, message)) => Err((status, Json(ErrorResponse { error: message }))),
    }
}

async fn get_nearest_stop(
    Query(query): Query<NearestStopQuery>,
    State(state): State<AppState>,
) -> Result<Json<NearestStopResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !(-90.0..=90.0).contains(&query.lat) || !(-180.0..=180.0).contains(&query.lon) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid latitude/longitude values".to_string(),
            }),
        ));
    }

    let nearest_stop = state
        .gtfs_cache
        .context
        .stops_map
        .values()
        .map(|stop| {
            let distance_km =
                haversine_distance(query.lat, query.lon, stop.stop_lat, stop.stop_lon);
            (stop, distance_km)
        })
        .min_by(|(_, left_distance), (_, right_distance)| {
            left_distance
                .partial_cmp(right_distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "No stops available".to_string(),
                }),
            )
        })?;

    let (stop, distance_km) = nearest_stop;
    let response = NearestStopResponse {
        stop_id: stop.stop_id.clone(),
        stop_name: stop.stop_name.clone(),
        stop_desc: stop.stop_desc.clone(),
        stop_lat: stop.stop_lat,
        stop_lon: stop.stop_lon,
        distance_km: (distance_km * 1000.0).round() / 1000.0,
        distance_meters: (distance_km * 1000.0 * 10.0).round() / 10.0,
    };

    println!(
        "Calling get_nearest_stop for lat={}, lon={} -> stop_id={}",
        query.lat, query.lon, response.stop_id
    );
    Ok(Json(response))
}

// ── Redis snapshot loading ────────────────────────────────────────────────────

pub async fn load_active_bus_snapshot(
    state: &AppState,
) -> Result<RedisBusSnapshot, (StatusCode, Json<ErrorResponse>)> {
    load_active_bus_snapshot_with_keys(
        state,
        REDIS_BUSES_LATEST_KEY,
        REDIS_BUSES_LAST_SEEN_KEY,
        REDIS_BUSES_MOTION_KEY,
        REDIS_INGEST_LAST_KEY,
    )
    .await
}

pub async fn load_active_bus_snapshot_with_keys(
    state: &AppState,
    latest_key: &str,
    last_seen_key: &str,
    motion_key: &str,
    ingest_key: &str,
) -> Result<RedisBusSnapshot, (StatusCode, Json<ErrorResponse>)> {
    let now_ms = now_unix_ms();
    let cutoff_ms = now_ms - state.bus_ttl_ms;
    let mut redis_conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(internal_error)?;

    let stale_bus_ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(last_seen_key)
        .arg("-inf")
        .arg(cutoff_ms)
        .query_async(&mut redis_conn)
        .await
        .map_err(internal_error)?;

    if !stale_bus_ids.is_empty() {
        let mut delete_pipe = redis::pipe();
        delete_pipe
            .cmd("HDEL")
            .arg(latest_key)
            .arg(&stale_bus_ids)
            .ignore();
        delete_pipe
            .cmd("HDEL")
            .arg(motion_key)
            .arg(&stale_bus_ids)
            .ignore();
        delete_pipe
            .cmd("ZREMRANGEBYSCORE")
            .arg(last_seen_key)
            .arg("-inf")
            .arg(cutoff_ms)
            .ignore();
        delete_pipe
            .query_async::<()>(&mut redis_conn)
            .await
            .map_err(internal_error)?;
    }

    let active_bus_ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(last_seen_key)
        .arg(cutoff_ms + 1)
        .arg("+inf")
        .query_async(&mut redis_conn)
        .await
        .map_err(internal_error)?;

    let buses: Vec<BusPosition> = if active_bus_ids.is_empty() {
        Vec::new()
    } else {
        let raw_buses: Vec<Option<String>> = redis::cmd("HMGET")
            .arg(latest_key)
            .arg(&active_bus_ids)
            .query_async(&mut redis_conn)
            .await
            .map_err(internal_error)?;

        raw_buses
            .into_iter()
            .flatten()
            .filter_map(|entry| serde_json::from_str::<BusPosition>(&entry).ok())
            .collect()
    };

    let motion_states: HashMap<String, BusMotionState> = if active_bus_ids.is_empty() {
        HashMap::new()
    } else {
        let raw_states: Vec<Option<String>> = redis::cmd("HMGET")
            .arg(motion_key)
            .arg(&active_bus_ids)
            .query_async(&mut redis_conn)
            .await
            .map_err(internal_error)?;

        active_bus_ids
            .iter()
            .cloned()
            .zip(raw_states.into_iter())
            .filter_map(|(bus_no, raw_state)| {
                raw_state.and_then(|value| {
                    serde_json::from_str::<BusMotionState>(&value)
                        .ok()
                        .map(|state| (bus_no, state))
                })
            })
            .collect()
    };

    let last_ingest_at_unix_ms: Option<i64> = redis::cmd("GET")
        .arg(ingest_key)
        .query_async(&mut redis_conn)
        .await
        .unwrap_or(None);

    Ok(RedisBusSnapshot {
        buses,
        motion_states,
        active_bus_count: active_bus_ids.len(),
        last_ingest_at_unix_ms,
    })
}

// ── ETA calculation ───────────────────────────────────────────────────────────

pub fn calculate_stop_eta_from_snapshot(
    snapshot: &RedisBusSnapshot,
    gtfs: &GtfsCache,
    stop_id: &str,
    stationary_window_ms: i64,
) -> Vec<BusEta> {
    let visible_buses = filter_non_stationary_buses(snapshot, stationary_window_ms);
    let mut all_eta_results: Vec<BusEta> = Vec::new();
    let mut seen_bus_route: HashSet<String> = HashSet::new();

    for route_stops in gtfs.route_stops_by_route.values() {
        if !route_stops.stops.iter().any(|stop| stop.stop_id == stop_id) {
            continue;
        }

        let route_eta_results = match calculate_route_eta_from_stops(
            &visible_buses,
            &route_stops.route_id,
            stop_id,
            route_stops,
        ) {
            Ok(results) => results,
            Err(_) => continue,
        };

        for eta in route_eta_results {
            let key = format!("{}::{}", eta.route_id, eta.bus_no);
            if seen_bus_route.insert(key) {
                all_eta_results.push(eta);
            }
        }
    }

    all_eta_results.sort_by(|a, b| {
        a.eta_minutes
            .partial_cmp(&b.eta_minutes)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    all_eta_results
}

pub async fn calculate_route_eta(
    state: &AppState,
    route_id: &str,
    target_stop_id: &str,
) -> Result<Vec<BusEta>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(state).await?;
    let visible_buses = filter_non_stationary_buses(&snapshot, state.stationary_window_ms);
    let route_stops = get_route_stops_from_cache(route_id, state.gtfs_cache.as_ref())
        .map_err(|(status, msg)| (status, Json(ErrorResponse { error: msg })))?;

    calculate_route_eta_from_stops(&visible_buses, route_id, target_stop_id, &route_stops)
        .map_err(|message| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: message }),
            )
        })
}

fn calculate_route_eta_from_stops(
    buses: &[BusPosition],
    route_id: &str,
    target_stop_id: &str,
    route_stops: &RouteStopsResponse,
) -> Result<Vec<BusEta>, String> {
    const DEFAULT_SPEED_KMH: f64 = 20.0;

    let target_stop = route_stops
        .stops
        .iter()
        .find(|s| s.stop_id == target_stop_id)
        .ok_or_else(|| {
            format!(
                "Target stop '{}' not found in route '{}'",
                target_stop_id, route_id
            )
        })?;
    let target_sequence = target_stop.sequence;

    let mut eta_results: Vec<BusEta> = Vec::new();

    for bus in buses
        .iter()
        .filter(|bus| is_bus_on_route(&bus.route, route_id))
    {
        let resolved_stop = match resolve_current_stop(bus, route_stops) {
            Some(stop) => stop,
            None => continue,
        };

        let current_sequence = resolved_stop.sequence;
        if current_sequence >= target_sequence {
            continue;
        }

        let stops_away = target_sequence - current_sequence;

        let intermediate_stops: Vec<&StopWithDetails> = route_stops
            .stops
            .iter()
            .filter(|s| s.sequence > current_sequence && s.sequence <= target_sequence)
            .collect();

        let mut total_distance_km = 0.0;
        let mut prev_lat = bus.latitude;
        let mut prev_lon = bus.longitude;

        for stop in &intermediate_stops {
            total_distance_km +=
                haversine_distance(prev_lat, prev_lon, stop.stop_lat, stop.stop_lon);
            prev_lat = stop.stop_lat;
            prev_lon = stop.stop_lon;
        }

        let speed = if bus.speed > 0.0 {
            bus.speed
        } else {
            DEFAULT_SPEED_KMH
        };
        let eta_minutes = (total_distance_km / speed) * 60.0;

        eta_results.push(BusEta {
            route_id: route_id.to_string(),
            bus_no: bus.bus_no.clone(),
            current_lat: bus.latitude,
            current_lon: bus.longitude,
            current_stop_id: resolved_stop.stop_id,
            current_stop_name: resolved_stop.stop_name,
            current_sequence,
            stop_resolution_source: resolved_stop.source,
            stops_away,
            distance_km: (total_distance_km * 100.0).round() / 100.0,
            speed_kmh: bus.speed,
            eta_minutes: (eta_minutes * 10.0).round() / 10.0,
        });
    }

    eta_results.sort_by(|a, b| {
        a.eta_minutes
            .partial_cmp(&b.eta_minutes)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(eta_results)
}

// ── Cache lookups ─────────────────────────────────────────────────────────────

pub fn get_route_stops_from_cache(
    route_id: &str,
    gtfs_cache: &GtfsCache,
) -> Result<RouteStopsResponse, (StatusCode, String)> {
    gtfs_cache
        .route_stops_by_route
        .get(route_id)
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Route '{}' not found", route_id),
            )
        })
}

pub fn get_routes_for_stop_from_cache(
    stop_id: &str,
    gtfs_cache: &GtfsCache,
) -> Result<Vec<StopRouteSummary>, (StatusCode, String)> {
    if !gtfs_cache.context.stops_map.contains_key(stop_id) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Stop '{}' not found", stop_id),
        ));
    }

    let stop_routes = gtfs_cache
        .routes_by_stop
        .get(stop_id)
        .cloned()
        .unwrap_or_default();
    if stop_routes.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("No routes found for stop '{}'", stop_id),
        ));
    }

    Ok(stop_routes)
}

// ── Bus stationarity filtering ────────────────────────────────────────────────

pub fn is_bus_stationary(
    snapshot: &RedisBusSnapshot,
    bus_no: &str,
    now_ms: i64,
    stationary_window_ms: i64,
) -> bool {
    snapshot
        .motion_states
        .get(bus_no)
        .and_then(|state| state.stationary_since_unix_ms)
        .map(|since_ms| now_ms - since_ms >= stationary_window_ms)
        .unwrap_or(false)
}

pub fn filter_non_stationary_buses(
    snapshot: &RedisBusSnapshot,
    stationary_window_ms: i64,
) -> Vec<BusPosition> {
    let now_ms = now_unix_ms();

    snapshot
        .buses
        .iter()
        .filter(|bus| !is_bus_stationary(snapshot, &bus.bus_no, now_ms, stationary_window_ms))
        .cloned()
        .collect()
}

// ── Shared utilities ──────────────────────────────────────────────────────────

pub async fn record_ingestor_error(state: &AppState, message: String, count_reconnect: bool) {
    let mut status = state.ingestor_status.write().await;
    status.connected = false;
    status.last_error = Some(message);
    if count_reconnect {
        status.reconnect_count += 1;
    }
}

pub fn internal_error(error: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: format!("Internal server error: {}", error),
        }),
    )
}

pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn decode_bus_data(encoded: &str) -> Option<String> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;

    let mut decoder = GzDecoder::new(&decoded[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).ok()?;

    Some(decompressed)
}

pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

// ── Dead code: alternative protobuf data source ───────────────────────────────

#[allow(dead_code)]
async fn prasarana_gtfs_data() -> Json<gtfs_realtime::FeedMessage> {
    let endpoint =
        "https://api.data.gov.my/gtfs-realtime/vehicle-position/prasarana?category=rapid-bus-kl";
    let response = reqwest::get(endpoint).await.unwrap();
    let body = response.bytes().await.unwrap();
    let feed = gtfs_realtime::FeedMessage::decode(body).unwrap();

    println!("Calling prasarana_gtfs_data");
    Json(feed)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
