use axum::{extract::State, http::StatusCode, Json};
use futures_util::FutureExt;
use rust_socketio::{asynchronous::ClientBuilder, Payload, TransportType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::MissedTickBehavior;

use crate::{
    decode_bus_data, haversine_distance, now_unix_ms, record_ingestor_error, AppState,
    ErrorResponse, StopIncomingMeta, StopIncomingResponse, REDIS_BUSES_LAST_SEEN_KEY,
    REDIS_BUSES_LATEST_KEY, REDIS_BUSES_MOTION_KEY, REDIS_INGEST_LAST_KEY,
    MAX_DERIVED_STOP_DISTANCE_KM, STATIONARY_DISTANCE_THRESHOLD_KM,
    STATIONARY_SPEED_THRESHOLD_KMH,
};

// ── RapidKL constants ─────────────────────────────────────────────────────────

pub const SOCKET_URL: &str = "https://rapidbus-socketio-avl.prasarana.com.my";
pub const PANTAI_HILLPARK_PHASE_5_STOP_ID: &str = "1008485";

// ── GTFS data structures ──────────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub route_id: String,
    pub agency_id: String,
    pub route_short_name: String,
    pub route_long_name: String,
    pub route_type: u32,
    pub route_color: String,
    pub route_text_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub route_id: String,
    pub service_id: String,
    pub trip_id: String,
    pub shape_id: String,
    pub trip_headsign: Option<String>,
    pub direction_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopTime {
    pub trip_id: String,
    pub arrival_time: String,
    pub departure_time: String,
    pub stop_id: String,
    pub stop_sequence: u32,
    pub stop_headsign: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_desc: String,
    pub stop_lat: f64,
    pub stop_lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopWithDetails {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_desc: String,
    pub stop_lat: f64,
    pub stop_lon: f64,
    pub sequence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapePoint {
    pub shape_id: String,
    pub shape_pt_lat: f64,
    pub shape_pt_lon: f64,
    pub shape_pt_sequence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStopsResponse {
    pub route_id: String,
    pub route_short_name: String,
    pub route_long_name: String,
    pub stops: Vec<StopWithDetails>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShapePointResponse {
    pub lat: f64,
    pub lon: f64,
    pub sequence: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteShapeResponse {
    pub route_id: String,
    pub shape_id: String,
    pub points: Vec<ShapePointResponse>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StopRouteSummary {
    pub route_id: String,
    pub route_short_name: String,
    pub route_long_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StopResolutionSource {
    Live,
    Derived,
}

#[derive(Debug, Clone)]
pub struct ResolvedCurrentStop {
    pub stop_id: String,
    pub stop_name: String,
    pub sequence: u32,
    pub source: StopResolutionSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct BusEta {
    pub route_id: String,
    pub bus_no: String,
    pub current_lat: f64,
    pub current_lon: f64,
    pub current_stop_id: String,
    pub current_stop_name: String,
    pub current_sequence: u32,
    pub stop_resolution_source: StopResolutionSource,
    pub stops_away: u32,
    pub distance_km: f64,
    pub speed_kmh: f64,
    pub eta_minutes: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteBusPositionResponse {
    #[serde(flatten)]
    pub bus: BusPosition,
    pub resolved_stop_id: Option<String>,
    pub resolved_stop_name: Option<String>,
    pub resolved_stop_sequence: Option<u32>,
    pub stop_resolution_source: Option<StopResolutionSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusMotionState {
    pub reference_lat: f64,
    pub reference_lon: f64,
    pub stationary_since_unix_ms: Option<i64>,
}

#[derive(Debug)]
pub struct GtfsContext {
    pub routes: Vec<Route>,
    pub trips_by_route: HashMap<String, Vec<Trip>>,
    pub stop_times_by_trip: HashMap<String, Vec<StopTime>>,
    pub stops_map: HashMap<String, Stop>,
}

#[derive(Debug)]
pub struct GtfsCache {
    pub context: GtfsContext,
    pub shapes_by_id: HashMap<String, Vec<ShapePoint>>,
    pub route_stops_by_route: HashMap<String, RouteStopsResponse>,
    pub routes_by_stop: HashMap<String, Vec<StopRouteSummary>>,
}

// ── GtfsCache builder ─────────────────────────────────────────────────────────

impl GtfsCache {
    pub fn build() -> Result<Self, Box<dyn std::error::Error>> {
        let context = GtfsContext {
            routes: load_routes()?,
            trips_by_route: load_trips()?,
            stop_times_by_trip: load_stop_times()?,
            stops_map: load_stops()?,
        };
        let shapes_by_id = load_shapes()?;
        let mut route_stops_by_route: HashMap<String, RouteStopsResponse> = HashMap::new();

        for route in &context.routes {
            if let Ok(route_stops) = get_stops_by_route(
                &route.route_id,
                &context.routes,
                &context.trips_by_route,
                &context.stop_times_by_trip,
                &context.stops_map,
            ) {
                route_stops_by_route.insert(route.route_id.clone(), route_stops);
            }
        }

        let mut routes_by_stop: HashMap<String, Vec<StopRouteSummary>> = HashMap::new();
        for route_stops in route_stops_by_route.values() {
            let route_summary = StopRouteSummary {
                route_id: route_stops.route_id.clone(),
                route_short_name: route_stops.route_short_name.clone(),
                route_long_name: route_stops.route_long_name.clone(),
            };
            let mut seen_stop_ids: HashSet<String> = HashSet::new();
            for stop in &route_stops.stops {
                if seen_stop_ids.insert(stop.stop_id.clone()) {
                    routes_by_stop
                        .entry(stop.stop_id.clone())
                        .or_default()
                        .push(route_summary.clone());
                }
            }
        }

        for route_summaries in routes_by_stop.values_mut() {
            route_summaries.sort_by(|a, b| {
                a.route_short_name
                    .cmp(&b.route_short_name)
                    .then(a.route_id.cmp(&b.route_id))
            });
        }

        Ok(Self {
            context,
            shapes_by_id,
            route_stops_by_route,
            routes_by_stop,
        })
    }
}

// ── GTFS file loaders (private) ───────────────────────────────────────────────

fn gtfs_data_dir() -> PathBuf {
    if let Ok(path) = env::var("GTFS_DATA_PATH") {
        return PathBuf::from(path);
    }

    StdPath::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/rapid-kl")
}

fn load_routes() -> Result<Vec<Route>, Box<dyn std::error::Error>> {
    let path = gtfs_data_dir().join("routes.txt");
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
    let path = gtfs_data_dir().join("trips.txt");
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
    let path = gtfs_data_dir().join("stop_times.txt");
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
    let path = gtfs_data_dir().join("stops.txt");
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

fn load_shapes() -> Result<HashMap<String, Vec<ShapePoint>>, Box<dyn std::error::Error>> {
    let path = gtfs_data_dir().join("shapes.txt");
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut shapes_by_id: HashMap<String, Vec<ShapePoint>> = HashMap::new();
    for result in rdr.deserialize() {
        let shape_point: ShapePoint = result?;
        shapes_by_id
            .entry(shape_point.shape_id.clone())
            .or_default()
            .push(shape_point);
    }

    for points in shapes_by_id.values_mut() {
        points.sort_by_key(|point| point.shape_pt_sequence);
    }

    Ok(shapes_by_id)
}

// ── GTFS query helpers ────────────────────────────────────────────────────────

pub fn get_stops_by_route(
    route_id: &str,
    routes: &[Route],
    trips_by_route: &HashMap<String, Vec<Trip>>,
    stop_times_by_trip: &HashMap<String, Vec<StopTime>>,
    stops_map: &HashMap<String, Stop>,
) -> Result<RouteStopsResponse, (StatusCode, String)> {
    let route = routes
        .iter()
        .find(|r| r.route_id == route_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Route '{}' not found", route_id),
            )
        })?;

    let trips = trips_by_route.get(route_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("No trips found for route '{}'", route_id),
        )
    })?;

    let first_trip = &trips[0];
    let stop_times = stop_times_by_trip.get(&first_trip.trip_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("No stop times found for trip '{}'", first_trip.trip_id),
        )
    })?;

    let mut sorted_stop_times: Vec<&StopTime> = stop_times.iter().collect();
    sorted_stop_times.sort_by_key(|st| st.stop_sequence);

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

pub fn get_shape_by_route(
    route_id: &str,
    stop_id: Option<&str>,
    routes: &[Route],
    trips_by_route: &HashMap<String, Vec<Trip>>,
    stop_times_by_trip: &HashMap<String, Vec<StopTime>>,
    shapes_by_id: &HashMap<String, Vec<ShapePoint>>,
) -> Result<RouteShapeResponse, (StatusCode, String)> {
    let route = routes
        .iter()
        .find(|r| r.route_id == route_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Route '{}' not found", route_id),
            )
        })?;

    let trips = trips_by_route.get(route_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("No trips found for route '{}'", route_id),
        )
    })?;

    let mut shape_trip_counts: HashMap<String, usize> = HashMap::new();
    let mut shape_trip_counts_for_stop: HashMap<String, usize> = HashMap::new();

    for trip in trips {
        *shape_trip_counts.entry(trip.shape_id.clone()).or_insert(0) += 1;

        if let Some(target_stop_id) = stop_id {
            if let Some(stop_times) = stop_times_by_trip.get(&trip.trip_id) {
                let has_target_stop = stop_times
                    .iter()
                    .any(|stop_time| stop_time.stop_id == target_stop_id);
                if has_target_stop {
                    *shape_trip_counts_for_stop
                        .entry(trip.shape_id.clone())
                        .or_insert(0) += 1;
                }
            }
        }
    }

    let preferred_counts = if shape_trip_counts_for_stop.is_empty() {
        &shape_trip_counts
    } else {
        &shape_trip_counts_for_stop
    };

    let selected_shape_id = preferred_counts
        .iter()
        .max_by(|(shape_a, count_a), (shape_b, count_b)| {
            count_a
                .cmp(count_b)
                .then_with(|| shape_a.cmp(shape_b).reverse())
        })
        .map(|(shape_id, _)| shape_id.clone())
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No shape found for route '{}'", route_id),
            )
        })?;

    let shape_points = shapes_by_id.get(&selected_shape_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!(
                "Shape '{}' not found in shapes.txt for route '{}'",
                selected_shape_id, route_id
            ),
        )
    })?;

    let points = shape_points
        .iter()
        .map(|point| ShapePointResponse {
            lat: point.shape_pt_lat,
            lon: point.shape_pt_lon,
            sequence: point.shape_pt_sequence,
        })
        .collect();

    Ok(RouteShapeResponse {
        route_id: route.route_id.clone(),
        shape_id: selected_shape_id,
        points,
    })
}

// ── Route/stop resolution ─────────────────────────────────────────────────────

pub fn is_bus_on_route(bus_route: &str, route_id: &str) -> bool {
    let bus_base = normalize_route_code(bus_route);
    let route_base = normalize_route_code(route_id);
    !bus_base.is_empty() && bus_base == route_base
}

fn is_t789_route(route: &str) -> bool {
    normalize_route_code(route) == "T789"
}

fn normalize_route_code(route: &str) -> String {
    route
        .trim()
        .to_uppercase()
        .trim_end_matches('0')
        .to_string()
}

pub fn resolve_current_stop(
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

// ── Motion state ──────────────────────────────────────────────────────────────

pub fn update_bus_motion_state(
    previous_state: Option<&BusMotionState>,
    bus: &BusPosition,
    now_ms: i64,
    speed_threshold_kmh: f64,
    distance_threshold_km: f64,
) -> BusMotionState {
    let reference_lat = previous_state
        .map(|state| state.reference_lat)
        .unwrap_or(bus.latitude);
    let reference_lon = previous_state
        .map(|state| state.reference_lon)
        .unwrap_or(bus.longitude);
    let distance_from_reference =
        haversine_distance(bus.latitude, bus.longitude, reference_lat, reference_lon);
    let is_slow = bus.speed <= speed_threshold_kmh;

    if distance_from_reference >= distance_threshold_km {
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

// ── Payload parsing ───────────────────────────────────────────────────────────

pub fn parse_bus_positions_from_payload(payload: Payload) -> (Vec<BusPosition>, u64) {
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

// ── Redis write ───────────────────────────────────────────────────────────────

pub async fn write_buses_to_redis(
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
        let motion_state = update_bus_motion_state(
            previous_motion_states.get(bus_no),
            bus,
            now_ms,
            STATIONARY_SPEED_THRESHOLD_KMH,
            STATIONARY_DISTANCE_THRESHOLD_KM,
        );

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

// ── SocketIO ingestor ─────────────────────────────────────────────────────────

pub async fn run_bus_ingestor(state: AppState) {
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

// ── RapidKL-specific HTTP handlers ────────────────────────────────────────────

pub async fn get_route_t789(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = crate::load_active_bus_snapshot(&state).await?;
    let visible_buses = crate::filter_non_stationary_buses(&snapshot, state.stationary_window_ms);
    let route_stops = crate::get_route_stops_from_cache("T7890", state.gtfs_cache.as_ref())
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

pub async fn get_t789_eta(
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    const TARGET_STOP_ID: &str = "1000838";
    let eta_results = crate::calculate_route_eta(&state, "T7890", TARGET_STOP_ID).await?;
    println!(
        "Calling get_t789_eta: found {} buses with ETA",
        eta_results.len()
    );
    Ok(Json(eta_results))
}

pub async fn get_pantai_hillpark_phase_5_eta(
    State(state): State<AppState>,
) -> Result<Json<StopIncomingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = crate::load_active_bus_snapshot(&state).await?;
    let stop = state
        .gtfs_cache
        .context
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
    let eta_results = crate::calculate_stop_eta_from_snapshot(
        &snapshot,
        state.gtfs_cache.as_ref(),
        PANTAI_HILLPARK_PHASE_5_STOP_ID,
        state.stationary_window_ms,
    );
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
