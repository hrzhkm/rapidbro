use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use base64::Engine;
use flate2::read::GzDecoder;
use futures_util::FutureExt;
use prost::Message;
use rust_socketio::{asynchronous::ClientBuilder, Payload, TransportType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path as StdPath;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Notify, RwLock};
use tokio::time::MissedTickBehavior;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusPosition {
    pub dt_received: Option<String>,
    pub dt_gps: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub dir: Option<String>,
    pub speed: f64,
    pub angle: f64,
    pub route: String,
    pub bus_no: String,
    pub trip_no: Option<String>,
    pub captain_id: Option<String>,
    pub trip_rev_kind: Option<String>,
    pub engine_status: i32,
    pub accessibility: i32,
    pub busstop_id: Option<String>,
    pub provider: String,
}

// GTFS data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Route {
    route_id: String,
    agency_id: String,
    route_short_name: String,
    route_long_name: String,
    route_type: u32,
    route_color: String,
    route_text_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Trip {
    route_id: String,
    service_id: String,
    trip_id: String,
    shape_id: String,
    trip_headsign: Option<String>,
    direction_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StopTime {
    trip_id: String,
    arrival_time: String,
    departure_time: String,
    stop_id: String,
    stop_sequence: u32,
    stop_headsign: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stop {
    stop_id: String,
    stop_name: String,
    stop_desc: String,
    stop_lat: f64,
    stop_lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StopWithDetails {
    stop_id: String,
    stop_name: String,
    stop_desc: String,
    stop_lat: f64,
    stop_lon: f64,
    sequence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteStopsResponse {
    route_id: String,
    route_short_name: String,
    route_long_name: String,
    stops: Vec<StopWithDetails>,
}

#[derive(Debug, Deserialize)]
struct NearestStopQuery {
    lat: f64,
    lon: f64,
}

#[derive(Debug, Serialize)]
struct NearestStopResponse {
    stop_id: String,
    stop_name: String,
    stop_desc: String,
    stop_lat: f64,
    stop_lon: f64,
    distance_km: f64,
    distance_meters: f64,
}

#[derive(Debug, Clone, Serialize)]
struct StopRouteSummary {
    route_id: String,
    route_short_name: String,
    route_long_name: String,
}

#[derive(Debug, Serialize)]
struct StopRoutesResponse {
    stop_id: String,
    routes: Vec<StopRouteSummary>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum StopResolutionSource {
    Live,
    Derived,
}

#[derive(Debug, Clone)]
struct ResolvedCurrentStop {
    stop_id: String,
    stop_name: String,
    sequence: u32,
    source: StopResolutionSource,
}

#[derive(Debug, Clone, Serialize)]
struct BusEta {
    route_id: String,
    bus_no: String,
    current_lat: f64,
    current_lon: f64,
    current_stop_id: String,
    current_stop_name: String,
    current_sequence: u32,
    stop_resolution_source: StopResolutionSource,
    stops_away: u32,
    distance_km: f64,
    speed_kmh: f64,
    eta_minutes: f64,
}

#[derive(Debug, Clone)]
struct AppState {
    redis_client: redis::Client,
    ingestor_status: Arc<RwLock<IngestorStatus>>,
    bus_ttl_ms: i64,
    stale_after_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IngestorStatus {
    connected: bool,
    reconnect_count: u64,
    messages_processed: u64,
    buses_written: u64,
    decode_failures: u64,
    redis_write_failures: u64,
    last_message_unix_ms: Option<i64>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct GetAllMeta {
    source: &'static str,
    last_ingest_at_unix_ms: Option<i64>,
    is_stale: bool,
    active_bus_count: usize,
}

#[derive(Debug, Serialize)]
struct GetAllResponse {
    data: Vec<BusPosition>,
    meta: GetAllMeta,
}

#[derive(Debug, Clone, Serialize)]
struct RouteBusPositionResponse {
    #[serde(flatten)]
    bus: BusPosition,
    resolved_stop_id: Option<String>,
    resolved_stop_name: Option<String>,
    resolved_stop_sequence: Option<u32>,
    stop_resolution_source: Option<StopResolutionSource>,
}

#[derive(Debug, Serialize)]
struct StopIncomingMeta {
    source: &'static str,
    generated_at_unix_ms: i64,
    last_ingest_at_unix_ms: Option<i64>,
    is_stale: bool,
    active_bus_count: usize,
    incoming_bus_count: usize,
    has_incoming_buses: bool,
}

#[derive(Debug, Serialize)]
struct StopIncomingResponse {
    stop_id: String,
    stop_name: String,
    stop_desc: String,
    data: Vec<BusEta>,
    meta: StopIncomingMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BusMotionState {
    reference_lat: f64,
    reference_lon: f64,
    stationary_since_unix_ms: Option<i64>,
}

#[derive(Debug)]
struct RedisBusSnapshot {
    buses: Vec<BusPosition>,
    motion_states: HashMap<String, BusMotionState>,
    active_bus_count: usize,
    last_ingest_at_unix_ms: Option<i64>,
}

struct GtfsContext {
    routes: Vec<Route>,
    trips_by_route: HashMap<String, Vec<Trip>>,
    stop_times_by_trip: HashMap<String, Vec<StopTime>>,
    stops_map: HashMap<String, Stop>,
}

const SOCKET_URL: &str = "https://rapidbus-socketio-avl.prasarana.com.my";
const GTFS_DATA_PATH: &str = "../rapid_kl_data";
const REDIS_BUSES_LATEST_KEY: &str = "rapidbro:buses:latest";
const REDIS_BUSES_LAST_SEEN_KEY: &str = "rapidbro:buses:last_seen";
const REDIS_BUSES_MOTION_KEY: &str = "rapidbro:buses:motion";
const REDIS_INGEST_LAST_KEY: &str = "rapidbro:ingestor:last_ingest_at";
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/";
const DEFAULT_BUS_TTL_SECONDS: i64 = 120;
const DEFAULT_STALE_AFTER_SECONDS: i64 = 20;
const MAX_DERIVED_STOP_DISTANCE_KM: f64 = 0.75;
const STATIONARY_SPEED_THRESHOLD_KMH: f64 = 1.0;
const STATIONARY_DISTANCE_THRESHOLD_KM: f64 = 0.03;
const STATIONARY_WINDOW_MS: i64 = 60_000;
const PANTAI_HILLPARK_PHASE_5_STOP_ID: &str = "1008485";

#[tokio::main]
async fn main() {
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let bus_ttl_seconds = env::var("BUS_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_BUS_TTL_SECONDS);
    let stale_after_seconds = env::var("STALE_AFTER_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_STALE_AFTER_SECONDS);

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

    // Fail fast if Redis is unavailable at startup.
    let mut redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .unwrap_or_else(|error| panic!("Failed to connect to Redis '{}': {}", redis_url, error));
    let _: String = redis::cmd("PING")
        .query_async(&mut redis_conn)
        .await
        .unwrap_or_else(|error| panic!("Failed to ping Redis '{}': {}", redis_url, error));

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
        bus_ttl_ms: bus_ttl_seconds * 1_000,
        stale_after_ms: stale_after_seconds * 1_000,
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
        .route("/stops/nearest", get(get_nearest_stop))
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();

    println!("Server is running on http://localhost:3030");
    axum::serve(listener, app).await.unwrap();
}

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

async fn load_active_bus_snapshot(
    state: &AppState,
) -> Result<RedisBusSnapshot, (StatusCode, Json<ErrorResponse>)> {
    let now_ms = now_unix_ms();
    let cutoff_ms = now_ms - state.bus_ttl_ms;
    let mut redis_conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(internal_error)?;

    let stale_bus_ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(REDIS_BUSES_LAST_SEEN_KEY)
        .arg("-inf")
        .arg(cutoff_ms)
        .query_async(&mut redis_conn)
        .await
        .map_err(internal_error)?;

    if !stale_bus_ids.is_empty() {
        let mut delete_pipe = redis::pipe();
        delete_pipe
            .cmd("HDEL")
            .arg(REDIS_BUSES_LATEST_KEY)
            .arg(&stale_bus_ids)
            .ignore();
        delete_pipe
            .cmd("HDEL")
            .arg(REDIS_BUSES_MOTION_KEY)
            .arg(&stale_bus_ids)
            .ignore();
        delete_pipe
            .cmd("ZREMRANGEBYSCORE")
            .arg(REDIS_BUSES_LAST_SEEN_KEY)
            .arg("-inf")
            .arg(cutoff_ms)
            .ignore();
        delete_pipe
            .query_async::<()>(&mut redis_conn)
            .await
            .map_err(internal_error)?;
    }

    let active_bus_ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(REDIS_BUSES_LAST_SEEN_KEY)
        .arg(cutoff_ms + 1)
        .arg("+inf")
        .query_async(&mut redis_conn)
        .await
        .map_err(internal_error)?;

    let buses: Vec<BusPosition> = if active_bus_ids.is_empty() {
        Vec::new()
    } else {
        let raw_buses: Vec<Option<String>> = redis::cmd("HMGET")
            .arg(REDIS_BUSES_LATEST_KEY)
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
            .arg(REDIS_BUSES_MOTION_KEY)
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
        .arg(REDIS_INGEST_LAST_KEY)
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

async fn get_ingestor_status(State(state): State<AppState>) -> Json<IngestorStatus> {
    Json(state.ingestor_status.read().await.clone())
}

async fn run_bus_ingestor(state: AppState) {
    let mut backoff_seconds: u64 = 1;

    loop {
        let redis_conn = match state.redis_client.get_multiplexed_async_connection().await {
            Ok(connection) => connection,
            Err(error) => {
                record_ingestor_error(
                    &state,
                    format!("Redis connection failed before socket connect: {}", error),
                    true,
                )
                .await;
                tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                backoff_seconds = (backoff_seconds * 2).min(30);
                continue;
            }
        };

        let disconnect_notify = Arc::new(Notify::new());
        let on_any_state = state.clone();
        let on_any_conn = redis_conn.clone();

        let on_any = move |_event: rust_socketio::Event,
                           payload: Payload,
                           _socket: rust_socketio::asynchronous::Client| {
            let state = on_any_state.clone();
            let mut redis_conn = on_any_conn.clone();
            async move {
                let now_ms = now_unix_ms();
                let (buses, decode_failures) = parse_bus_positions_from_payload(payload);

                {
                    let mut status = state.ingestor_status.write().await;
                    status.messages_processed += 1;
                    status.last_message_unix_ms = Some(now_ms);
                    status.decode_failures += decode_failures;
                }

                if buses.is_empty() {
                    return;
                }

                match write_buses_to_redis(&mut redis_conn, &buses, now_ms).await {
                    Ok(written_count) => {
                        let mut status = state.ingestor_status.write().await;
                        status.buses_written += written_count as u64;
                        status.last_error = None;
                    }
                    Err(error) => {
                        let mut status = state.ingestor_status.write().await;
                        status.redis_write_failures += 1;
                        status.last_error = Some(format!("Redis write failed: {}", error));
                    }
                }
            }
            .boxed()
        };

        let disconnect_state = state.clone();
        let disconnect_signal = disconnect_notify.clone();
        let disconnect_state_for_error = state.clone();
        let disconnect_signal_for_error = disconnect_notify.clone();

        let socket = ClientBuilder::new(SOCKET_URL)
            .transport_type(TransportType::Websocket)
            .on_any(on_any)
            .on("disconnect", move |_, _| {
                let state = disconnect_state.clone();
                let notify = disconnect_signal.clone();
                async move {
                    {
                        let mut status = state.ingestor_status.write().await;
                        status.connected = false;
                        status.last_error = Some("Socket disconnected".to_string());
                        status.reconnect_count += 1;
                    }
                    notify.notify_one();
                }
                .boxed()
            })
            .on("error", move |_, _| {
                let state = disconnect_state_for_error.clone();
                let notify = disconnect_signal_for_error.clone();
                async move {
                    {
                        let mut status = state.ingestor_status.write().await;
                        status.connected = false;
                        status.last_error = Some("Socket error event".to_string());
                        status.reconnect_count += 1;
                    }
                    notify.notify_one();
                }
                .boxed()
            })
            .connect()
            .await;

        match socket {
            Ok(socket) => {
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": ""
                });
                if let Err(error) = socket.emit("onFts-reload", payload).await {
                    record_ingestor_error(
                        &state,
                        format!("Socket subscribe emit failed: {}", error),
                        true,
                    )
                    .await;
                    tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                    backoff_seconds = (backoff_seconds * 2).min(30);
                    continue;
                }

                {
                    let mut status = state.ingestor_status.write().await;
                    status.connected = true;
                    status.last_error = None;
                }

                backoff_seconds = 1;
                let mut reload_interval = tokio::time::interval(Duration::from_secs(20));
                reload_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                // Consume immediate first tick so the first periodic reload happens after the interval.
                reload_interval.tick().await;

                loop {
                    tokio::select! {
                        _ = disconnect_notify.notified() => {
                            break;
                        }
                        _ = reload_interval.tick() => {
                            let payload = json!({
                                "sid": "",
                                "uid": "",
                                "provider": "RKL",
                                "route": ""
                            });

                            if let Err(error) = socket.emit("onFts-reload", payload).await {
                                record_ingestor_error(
                                    &state,
                                    format!("Periodic socket reload emit failed: {}", error),
                                    true,
                                )
                                .await;
                                break;
                            }
                        }
                    }
                }
                drop(socket);
            }
            Err(error) => {
                record_ingestor_error(&state, format!("Socket connection failed: {}", error), true)
                    .await;
                tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                backoff_seconds = (backoff_seconds * 2).min(30);
            }
        }
    }
}

async fn write_buses_to_redis(
    redis_conn: &mut redis::aio::MultiplexedConnection,
    buses: &[BusPosition],
    now_ms: i64,
) -> Result<usize, String> {
    let mut serialized_entries: Vec<(String, String)> = Vec::new();
    let valid_buses: HashMap<String, &BusPosition> = buses
        .iter()
        .filter(|bus| !bus.bus_no.is_empty())
        .map(|bus| (bus.bus_no.clone(), bus))
        .collect();
    let bus_ids: Vec<String> = valid_buses.keys().cloned().collect();

    let previous_motion_states: HashMap<String, BusMotionState> = if bus_ids.is_empty() {
        HashMap::new()
    } else {
        let raw_states: Vec<Option<String>> = redis::cmd("HMGET")
            .arg(REDIS_BUSES_MOTION_KEY)
            .arg(&bus_ids)
            .query_async(redis_conn)
            .await
            .map_err(|error| error.to_string())?;

        bus_ids
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

    for bus in buses {
        if bus.bus_no.is_empty() {
            continue;
        }

        if let Ok(serialized_bus) = serde_json::to_string(bus) {
            serialized_entries.push((bus.bus_no.clone(), serialized_bus));
        }
    }

    if serialized_entries.is_empty() {
        return Ok(0);
    }

    let mut pipe = redis::pipe();
    for (bus_no, bus_json) in &serialized_entries {
        let Some(bus) = valid_buses.get(bus_no) else {
            continue;
        };
        let motion_state = update_bus_motion_state(previous_motion_states.get(bus_no), bus, now_ms);

        pipe.cmd("HSET")
            .arg(REDIS_BUSES_LATEST_KEY)
            .arg(bus_no)
            .arg(bus_json)
            .ignore();
        pipe.cmd("HSET")
            .arg(REDIS_BUSES_MOTION_KEY)
            .arg(bus_no)
            .arg(serde_json::to_string(&motion_state).map_err(|error| error.to_string())?)
            .ignore();
        pipe.cmd("ZADD")
            .arg(REDIS_BUSES_LAST_SEEN_KEY)
            .arg(now_ms)
            .arg(bus_no)
            .ignore();
    }

    pipe.cmd("SET")
        .arg(REDIS_INGEST_LAST_KEY)
        .arg(now_ms)
        .ignore();

    pipe.query_async::<()>(redis_conn)
        .await
        .map_err(|error| error.to_string())?;

    Ok(serialized_entries.len())
}

fn parse_bus_positions_from_payload(payload: Payload) -> (Vec<BusPosition>, u64) {
    let mut buses = Vec::new();
    let mut decode_failures = 0;

    if let Payload::Text(values) = payload {
        for value in values {
            let Some(encoded_str) = value.as_str() else {
                continue;
            };

            let Some(decoded) = decode_bus_data(encoded_str) else {
                decode_failures += 1;
                continue;
            };

            match parse_bus_positions_from_json(&decoded) {
                Some(mut parsed_buses) => buses.append(&mut parsed_buses),
                None => decode_failures += 1,
            }
        }
    }

    (buses, decode_failures)
}

fn parse_bus_positions_from_json(decoded: &str) -> Option<Vec<BusPosition>> {
    if let Ok(single_bus) = serde_json::from_str::<BusPosition>(decoded) {
        return Some(vec![single_bus]);
    }

    if let Ok(bus_list) = serde_json::from_str::<Vec<BusPosition>>(decoded) {
        return Some(bus_list);
    }

    let value = serde_json::from_str::<serde_json::Value>(decoded).ok()?;
    if let serde_json::Value::Array(entries) = value {
        let buses: Vec<BusPosition> = entries
            .into_iter()
            .filter_map(|entry| serde_json::from_value::<BusPosition>(entry).ok())
            .collect();

        if buses.is_empty() {
            None
        } else {
            Some(buses)
        }
    } else {
        None
    }
}

async fn record_ingestor_error(state: &AppState, message: String, count_reconnect: bool) {
    let mut status = state.ingestor_status.write().await;
    status.connected = false;
    status.last_error = Some(message);
    if count_reconnect {
        status.reconnect_count += 1;
    }
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: format!("Internal server error: {}", error),
        }),
    )
}

fn is_t789_route(route: &str) -> bool {
    normalize_route_code(route) == "T789"
}

fn is_bus_on_route(bus_route: &str, route_id: &str) -> bool {
    let bus_base = normalize_route_code(bus_route);
    let route_base = normalize_route_code(route_id);
    !bus_base.is_empty() && bus_base == route_base
}

fn normalize_route_code(route: &str) -> String {
    route
        .trim()
        .to_uppercase()
        .trim_end_matches('0')
        .to_string()
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

// Get buses for route T789 specifically from Redis snapshot
async fn get_route_t789(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(&state).await?;
    let gtfs = load_gtfs_context()?;
    let visible_buses = filter_non_stationary_buses(&snapshot);
    let route_stops = get_stops_by_route(
        "T7890",
        &gtfs.routes,
        &gtfs.trips_by_route,
        &gtfs.stop_times_by_trip,
        &gtfs.stops_map,
    )
    .map_err(|(status, msg)| (status, Json(ErrorResponse { error: msg })))?;
    let t789_buses: Vec<RouteBusPositionResponse> = visible_buses
        .into_iter()
        .filter(|bus| is_t789_route(&bus.route))
        .map(|bus| {
            let resolved_stop = resolve_current_stop(&bus, &route_stops);
            RouteBusPositionResponse {
                resolved_stop_id: resolved_stop.as_ref().map(|stop| stop.stop_id.clone()),
                resolved_stop_name: resolved_stop.as_ref().map(|stop| stop.stop_name.clone()),
                resolved_stop_sequence: resolved_stop.as_ref().map(|stop| stop.sequence),
                stop_resolution_source: resolved_stop.map(|stop| stop.source),
                bus,
            }
        })
        .collect();

    println!(
        "Calling get_route_t789 via Redis: {} active buses",
        t789_buses.len()
    );

    if t789_buses.len() == 1 {
        let value = serde_json::to_value(&t789_buses[0]).unwrap_or_else(|_| json!({}));
        Ok(Json(value))
    } else {
        let value = serde_json::to_value(&t789_buses).unwrap_or_else(|_| json!([]));
        Ok(Json(value))
    }
}

// Calculate ETA for T789 buses from Redis snapshot to reach stop 1000838 (KL1397 FLAT PKNS KERINCHI/KL GATEWAY)
async fn get_t789_eta(
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    const TARGET_STOP_ID: &str = "1000838";
    let eta_results = calculate_route_eta(&state, "T7890", TARGET_STOP_ID).await?;
    println!(
        "Calling get_t789_eta: found {} buses with ETA",
        eta_results.len()
    );
    Ok(Json(eta_results))
}

// Calculate ETA for all incoming buses to Pantai Hillpark Phase 5 (stop 1008485).
async fn get_pantai_hillpark_phase_5_eta(
    State(state): State<AppState>,
) -> Result<Json<StopIncomingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(&state).await?;
    let gtfs = load_gtfs_context()?;
    let stop = gtfs
        .stops_map
        .get(PANTAI_HILLPARK_PHASE_5_STOP_ID)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!(
                        "Stop '{}' not found in GTFS data",
                        PANTAI_HILLPARK_PHASE_5_STOP_ID
                    ),
                }),
            )
        })?;
    let eta_results =
        calculate_stop_eta_from_snapshot(&snapshot, &gtfs, PANTAI_HILLPARK_PHASE_5_STOP_ID);
    let now_ms = now_unix_ms();
    let is_stale = match snapshot.last_ingest_at_unix_ms {
        Some(last_ingest_ms) => now_ms - last_ingest_ms > state.stale_after_ms,
        None => true,
    };

    println!(
        "Calling get_pantai_hillpark_phase_5_eta: {} incoming buses",
        eta_results.len()
    );

    Ok(Json(StopIncomingResponse {
        stop_id: stop.stop_id.clone(),
        stop_name: stop.stop_name.clone(),
        stop_desc: stop.stop_desc.clone(),
        meta: StopIncomingMeta {
            source: "redis",
            generated_at_unix_ms: now_ms,
            last_ingest_at_unix_ms: snapshot.last_ingest_at_unix_ms,
            is_stale,
            active_bus_count: snapshot.active_bus_count,
            incoming_bus_count: eta_results.len(),
            has_incoming_buses: !eta_results.is_empty(),
        },
        data: eta_results,
    }))
}

// Calculate ETA for buses in route/{route_id} to reach stop/{stop_id}, based on Redis snapshot.
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

// Calculate ETA for all routes incoming to /stops/{stop_id}
async fn get_stop_eta(
    Path(stop_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(&state).await?;
    let gtfs = load_gtfs_context()?;
    let all_eta_results = calculate_stop_eta_from_snapshot(&snapshot, &gtfs, &stop_id);

    println!(
        "Calling get_stop_eta for stop_id={}: {} incoming buses",
        stop_id,
        all_eta_results.len()
    );
    Ok(Json(all_eta_results))
}

async fn get_stop_routes(
    Path(stop_id): Path<String>,
) -> Result<Json<StopRoutesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let gtfs = load_gtfs_context()?;
    let routes = get_routes_for_stop(
        &stop_id,
        &gtfs.routes,
        &gtfs.trips_by_route,
        &gtfs.stop_times_by_trip,
        &gtfs.stops_map,
    )
    .map_err(|(status, message)| (status, Json(ErrorResponse { error: message })))?;

    println!(
        "Calling get_stop_routes for stop_id={}: {} routes",
        stop_id,
        routes.len()
    );

    Ok(Json(StopRoutesResponse { stop_id, routes }))
}

fn calculate_stop_eta_from_snapshot(
    snapshot: &RedisBusSnapshot,
    gtfs: &GtfsContext,
    stop_id: &str,
) -> Vec<BusEta> {
    let visible_buses = filter_non_stationary_buses(snapshot);
    let mut all_eta_results: Vec<BusEta> = Vec::new();
    let mut seen_bus_route: HashSet<String> = HashSet::new();

    for route in &gtfs.routes {
        let route_stops = match get_stops_by_route(
            &route.route_id,
            &gtfs.routes,
            &gtfs.trips_by_route,
            &gtfs.stop_times_by_trip,
            &gtfs.stops_map,
        ) {
            Ok(stops) => stops,
            Err(_) => continue,
        };

        if !route_stops.stops.iter().any(|stop| stop.stop_id == stop_id) {
            continue;
        }

        let route_eta_results = match calculate_route_eta_from_stops(
            &visible_buses,
            &route.route_id,
            stop_id,
            &route_stops,
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

fn update_bus_motion_state(
    previous_state: Option<&BusMotionState>,
    bus: &BusPosition,
    now_ms: i64,
) -> BusMotionState {
    let reference_lat = previous_state
        .map(|state| state.reference_lat)
        .unwrap_or(bus.latitude);
    let reference_lon = previous_state
        .map(|state| state.reference_lon)
        .unwrap_or(bus.longitude);
    let distance_from_reference =
        haversine_distance(bus.latitude, bus.longitude, reference_lat, reference_lon);
    let is_slow = bus.speed <= STATIONARY_SPEED_THRESHOLD_KMH;

    if distance_from_reference >= STATIONARY_DISTANCE_THRESHOLD_KM {
        return BusMotionState {
            reference_lat: bus.latitude,
            reference_lon: bus.longitude,
            stationary_since_unix_ms: is_slow.then_some(now_ms),
        };
    }

    if is_slow {
        return BusMotionState {
            reference_lat,
            reference_lon,
            stationary_since_unix_ms: previous_state
                .and_then(|state| state.stationary_since_unix_ms)
                .or(Some(now_ms)),
        };
    }

    BusMotionState {
        reference_lat: bus.latitude,
        reference_lon: bus.longitude,
        stationary_since_unix_ms: None,
    }
}

fn is_bus_stationary(snapshot: &RedisBusSnapshot, bus_no: &str, now_ms: i64) -> bool {
    snapshot
        .motion_states
        .get(bus_no)
        .and_then(|state| state.stationary_since_unix_ms)
        .map(|since_ms| now_ms - since_ms >= STATIONARY_WINDOW_MS)
        .unwrap_or(false)
}

fn filter_non_stationary_buses(snapshot: &RedisBusSnapshot) -> Vec<BusPosition> {
    let now_ms = now_unix_ms();

    snapshot
        .buses
        .iter()
        .filter(|bus| !is_bus_stationary(snapshot, &bus.bus_no, now_ms))
        .cloned()
        .collect()
}

fn resolve_current_stop(
    bus: &BusPosition,
    route_stops: &RouteStopsResponse,
) -> Option<ResolvedCurrentStop> {
    if let Some(bus_stop_id) = bus.busstop_id.as_ref().filter(|id| !id.is_empty()) {
        if let Some(stop) = route_stops
            .stops
            .iter()
            .find(|stop| stop.stop_id == *bus_stop_id)
        {
            return Some(ResolvedCurrentStop {
                stop_id: stop.stop_id.clone(),
                stop_name: stop.stop_name.clone(),
                sequence: stop.sequence,
                source: StopResolutionSource::Live,
            });
        }
    }

    let nearest_stop = route_stops.stops.iter().min_by(|a, b| {
        let distance_a = haversine_distance(bus.latitude, bus.longitude, a.stop_lat, a.stop_lon);
        let distance_b = haversine_distance(bus.latitude, bus.longitude, b.stop_lat, b.stop_lon);
        distance_a
            .partial_cmp(&distance_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;

    let distance_km = haversine_distance(
        bus.latitude,
        bus.longitude,
        nearest_stop.stop_lat,
        nearest_stop.stop_lon,
    );

    if distance_km > MAX_DERIVED_STOP_DISTANCE_KM {
        return None;
    }

    Some(ResolvedCurrentStop {
        stop_id: nearest_stop.stop_id.clone(),
        stop_name: nearest_stop.stop_name.clone(),
        sequence: nearest_stop.sequence,
        source: StopResolutionSource::Derived,
    })
}

async fn calculate_route_eta(
    state: &AppState,
    route_id: &str,
    target_stop_id: &str,
) -> Result<Vec<BusEta>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = load_active_bus_snapshot(state).await?;
    let visible_buses = filter_non_stationary_buses(&snapshot);
    let gtfs = load_gtfs_context()?;
    let route_stops = get_stops_by_route(
        route_id,
        &gtfs.routes,
        &gtfs.trips_by_route,
        &gtfs.stop_times_by_trip,
        &gtfs.stops_map,
    )
    .map_err(|(status, msg)| (status, Json(ErrorResponse { error: msg })))?;

    calculate_route_eta_from_stops(&visible_buses, route_id, target_stop_id, &route_stops).map_err(
        |message| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: message }),
            )
        },
    )
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

fn load_gtfs_context() -> Result<GtfsContext, (StatusCode, Json<ErrorResponse>)> {
    let routes = load_routes().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load routes: {}", e),
            }),
        )
    })?;

    let trips_by_route = load_trips().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load trips: {}", e),
            }),
        )
    })?;

    let stop_times_by_trip = load_stop_times().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load stop times: {}", e),
            }),
        )
    })?;

    let stops_map = load_stops().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load stops: {}", e),
            }),
        )
    })?;

    Ok(GtfsContext {
        routes,
        trips_by_route,
        stop_times_by_trip,
        stops_map,
    })
}

fn get_routes_for_stop(
    stop_id: &str,
    routes: &[Route],
    trips_by_route: &HashMap<String, Vec<Trip>>,
    stop_times_by_trip: &HashMap<String, Vec<StopTime>>,
    stops_map: &HashMap<String, Stop>,
) -> Result<Vec<StopRouteSummary>, (StatusCode, String)> {
    if !stops_map.contains_key(stop_id) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Stop '{}' not found", stop_id),
        ));
    }

    let mut stop_routes: Vec<StopRouteSummary> = routes
        .iter()
        .filter_map(|route| {
            let route_stops = get_stops_by_route(
                &route.route_id,
                routes,
                trips_by_route,
                stop_times_by_trip,
                stops_map,
            )
            .ok()?;

            route_stops
                .stops
                .iter()
                .any(|stop| stop.stop_id == stop_id)
                .then(|| StopRouteSummary {
                    route_id: route.route_id.clone(),
                    route_short_name: route.route_short_name.clone(),
                    route_long_name: route.route_long_name.clone(),
                })
        })
        .collect();

    stop_routes.sort_by(|a, b| {
        a.route_short_name
            .cmp(&b.route_short_name)
            .then(a.route_id.cmp(&b.route_id))
    });

    if stop_routes.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("No routes found for stop '{}'", stop_id),
        ));
    }

    Ok(stop_routes)
}

// Decode base64 + gzip compressed data from the websocket
fn decode_bus_data(encoded: &str) -> Option<String> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;

    let mut decoder = GzDecoder::new(&decoded[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).ok()?;

    Some(decompressed)
}

// Calculate haversine distance between two GPS coordinates (returns km)
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth radius in km
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

// Data OpenDOSM Prasarana - uses protobuf (alternative data source)
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

// GTFS data loading functions
fn load_routes() -> Result<Vec<Route>, Box<dyn std::error::Error>> {
    let path = StdPath::new(GTFS_DATA_PATH).join("routes.txt");
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut routes = Vec::new();
    for result in rdr.deserialize() {
        let route: Route = result?;
        routes.push(route);
    }
    Ok(routes)
}

fn load_trips() -> Result<HashMap<String, Vec<Trip>>, Box<dyn std::error::Error>> {
    let path = StdPath::new(GTFS_DATA_PATH).join("trips.txt");
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut trips_by_route: HashMap<String, Vec<Trip>> = HashMap::new();
    for result in rdr.deserialize() {
        let trip: Trip = result?;
        trips_by_route
            .entry(trip.route_id.clone())
            .or_default()
            .push(trip);
    }
    Ok(trips_by_route)
}

fn load_stop_times() -> Result<HashMap<String, Vec<StopTime>>, Box<dyn std::error::Error>> {
    let path = StdPath::new(GTFS_DATA_PATH).join("stop_times.txt");
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut stop_times_by_trip: HashMap<String, Vec<StopTime>> = HashMap::new();
    for result in rdr.deserialize() {
        let stop_time: StopTime = result?;
        stop_times_by_trip
            .entry(stop_time.trip_id.clone())
            .or_default()
            .push(stop_time);
    }
    Ok(stop_times_by_trip)
}

fn load_stops() -> Result<HashMap<String, Stop>, Box<dyn std::error::Error>> {
    let path = StdPath::new(GTFS_DATA_PATH).join("stops.txt");
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut stops_map = HashMap::new();
    for result in rdr.deserialize() {
        let stop: Stop = result?;
        stops_map.insert(stop.stop_id.clone(), stop);
    }
    Ok(stops_map)
}

// Get stops by route_id
fn get_stops_by_route(
    route_id: &str,
    routes: &[Route],
    trips_by_route: &HashMap<String, Vec<Trip>>,
    stop_times_by_trip: &HashMap<String, Vec<StopTime>>,
    stops_map: &HashMap<String, Stop>,
) -> Result<RouteStopsResponse, (StatusCode, String)> {
    // Find the route
    let route = routes
        .iter()
        .find(|r| r.route_id == route_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Route '{}' not found", route_id),
            )
        })?;

    // Get trips for this route
    let trips = trips_by_route.get(route_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("No trips found for route '{}'", route_id),
        )
    })?;

    // Get the first trip's stop times
    let first_trip = &trips[0];
    let stop_times = stop_times_by_trip.get(&first_trip.trip_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("No stop times found for trip '{}'", first_trip.trip_id),
        )
    })?;

    // Sort by stop_sequence
    let mut sorted_stop_times: Vec<&StopTime> = stop_times.iter().collect();
    sorted_stop_times.sort_by_key(|st| st.stop_sequence);

    // Build response with stop details
    let stops: Vec<StopWithDetails> = sorted_stop_times
        .into_iter()
        .filter_map(|st| {
            stops_map.get(&st.stop_id).map(|stop| StopWithDetails {
                stop_id: stop.stop_id.clone(),
                stop_name: stop.stop_name.clone(),
                stop_desc: stop.stop_desc.clone(),
                stop_lat: stop.stop_lat,
                stop_lon: stop.stop_lon,
                sequence: st.stop_sequence,
            })
        })
        .collect();

    Ok(RouteStopsResponse {
        route_id: route.route_id.clone(),
        route_short_name: route.route_short_name.clone(),
        route_long_name: route.route_long_name.clone(),
        stops,
    })
}

// Axum handler for /route/:route_id/stops
async fn get_route_stops(
    Path(route_id): Path<String>,
) -> Result<Json<RouteStopsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Load GTFS data
    let routes = match load_routes() {
        Ok(r) => r,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load routes: {}", e),
                }),
            ));
        }
    };

    let trips_by_route = match load_trips() {
        Ok(t) => t,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load trips: {}", e),
                }),
            ));
        }
    };

    let stop_times_by_trip = match load_stop_times() {
        Ok(st) => st,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load stop times: {}", e),
                }),
            ));
        }
    };

    let stops_map = match load_stops() {
        Ok(s) => s,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load stops: {}", e),
                }),
            ));
        }
    };

    match get_stops_by_route(
        &route_id,
        &routes,
        &trips_by_route,
        &stop_times_by_trip,
        &stops_map,
    ) {
        Ok(response) => {
            println!("Calling get_route_stops for route_id={}", route_id);
            Ok(Json(response))
        }
        Err((status, message)) => Err((status, Json(ErrorResponse { error: message }))),
    }
}

// Axum handler for /stops/nearest?lat={lat}&lon={lon}
async fn get_nearest_stop(
    Query(query): Query<NearestStopQuery>,
) -> Result<Json<NearestStopResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !(-90.0..=90.0).contains(&query.lat) || !(-180.0..=180.0).contains(&query.lon) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid latitude/longitude values".to_string(),
            }),
        ));
    }

    let stops_map = load_stops().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load stops: {}", e),
            }),
        )
    })?;

    let nearest_stop = stops_map
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
