use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use prost::Message;

use crate::rapidkl::{
    get_shape_by_route, is_bus_on_route, resolve_current_stop, write_buses_to_redis_with_keys,
    BusEta, BusPosition, RouteShapeResponse, RouteStopsResponse,
};
use crate::{
    calculate_stop_eta_from_snapshot, filter_non_stationary_buses, get_route_stops_from_cache,
    get_routes_for_stop_from_cache, haversine_distance, internal_error,
    load_active_bus_snapshot_with_keys, now_unix_ms, AppState, ErrorResponse, GetAllMeta,
    GetAllResponse, NearestStopQuery, NearestStopResponse, RedisBusSnapshot, RouteShapeQuery,
    StopRoutesResponse,
};

// ── Constants ─────────────────────────────────────────────────────────────────

const MYBAS_ALOR_SETAR_GTFS_RT_URL: &str =
    "https://api.data.gov.my/gtfs-realtime/vehicle-position/mybas-alor-setar";

/// TTL for cached bus data: 5 minutes.
const ALOR_SETAR_DATA_TTL_MS: i64 = 300_000;

pub const REDIS_MYBAS_ALOR_SETAR_LATEST_KEY: &str = "rapidbro:mybas-alor-setar:buses:latest";
pub const REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY: &str =
    "rapidbro:mybas-alor-setar:buses:last_seen";
pub const REDIS_MYBAS_ALOR_SETAR_MOTION_KEY: &str = "rapidbro:mybas-alor-setar:buses:motion";
pub const REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY: &str =
    "rapidbro:mybas-alor-setar:ingestor:last_ingest_at";

// ── GTFS-RT fetcher ───────────────────────────────────────────────────────────

pub async fn fetch_alor_setar_bus_positions(
) -> Result<Vec<BusPosition>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(MYBAS_ALOR_SETAR_GTFS_RT_URL).await?;
    let body = response.bytes().await?;
    let feed = gtfs_realtime::FeedMessage::decode(body)?;

    let positions: Vec<BusPosition> = feed
        .entity
        .into_iter()
        .filter_map(|entity| {
            let vehicle = entity.vehicle?;
            let position = vehicle.position?;

            let (vehicle_id, _label) = vehicle
                .vehicle
                .as_ref()
                .map(|v| (v.id.clone().unwrap_or_default(), v.label.clone()))
                .unwrap_or_default();

            if vehicle_id.is_empty() {
                return None;
            }

            Some(BusPosition {
                dt_received: None,
                dt_gps: vehicle.timestamp.map(|ts| ts.to_string()),
                latitude: position.latitude as f64,
                longitude: position.longitude as f64,
                dir: None,
                speed: position.speed.unwrap_or(0.0) as f64 * 3.6, // m/s → km/h
                angle: position.bearing.unwrap_or(0.0) as f64,
                route: vehicle
                    .trip
                    .as_ref()
                    .and_then(|t| t.route_id.clone())
                    .unwrap_or_default(),
                bus_no: vehicle_id,
                trip_no: vehicle.trip.as_ref().and_then(|t| t.trip_id.clone()),
                captain_id: None,
                trip_rev_kind: None,
                engine_status: 1,
                accessibility: 0,
                busstop_id: None,
                provider: "MYBAS-ALOR-SETAR".to_string(),
            })
        })
        .collect();

    Ok(positions)
}

// ── On-demand data freshness (double-check locking) ──────────────────────────

async fn get_alor_setar_last_ingest(
    state: &AppState,
) -> Result<Option<i64>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(internal_error)?;
    let val: Option<i64> = redis::cmd("GET")
        .arg(REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY)
        .query_async(&mut conn)
        .await
        .unwrap_or(None);
    Ok(val)
}

pub async fn ensure_alor_setar_data_fresh(
    state: &AppState,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    // Quick check without lock
    let last_ingest_ms = get_alor_setar_last_ingest(state).await?;
    let now_ms = now_unix_ms();
    if let Some(last_ms) = last_ingest_ms {
        if now_ms - last_ms < ALOR_SETAR_DATA_TTL_MS {
            return Ok(());
        }
    }

    // Acquire lock — only one fetch at a time
    let _guard = state.alor_setar_fetch_lock.lock().await;

    // Re-check after acquiring lock (another request may have fetched while we waited)
    let last_ingest_ms = get_alor_setar_last_ingest(state).await?;
    if let Some(last_ms) = last_ingest_ms {
        if now_unix_ms() - last_ms < ALOR_SETAR_DATA_TTL_MS {
            return Ok(());
        }
    }

    // Fetch from API
    let positions = fetch_alor_setar_bus_positions().await.map_err(|e| {
        internal_error(format!("[mybas-alor-setar] Fetch failed: {}", e))
    })?;

    println!(
        "[mybas-alor-setar] On-demand fetch: {} bus positions",
        positions.len()
    );

    if positions.is_empty() {
        // Still update timestamp so we don't hammer the API
        let mut conn = state
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(internal_error)?;
        let _: () = redis::cmd("SET")
            .arg(REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY)
            .arg(now_unix_ms())
            .query_async(&mut conn)
            .await
            .map_err(internal_error)?;
        return Ok(());
    }

    // Write to Redis using shared infrastructure
    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(internal_error)?;
    write_buses_to_redis_with_keys(
        &mut conn,
        &positions,
        now_unix_ms(),
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY,
        REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY,
        REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
        REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY,
    )
    .await
    .map_err(|e| internal_error(format!("[mybas-alor-setar] Redis write failed: {}", e)))?;

    Ok(())
}

// ── Alor Setar Redis snapshot loader ─────────────────────────────────────────

async fn load_alor_setar_bus_snapshot(
    state: &AppState,
) -> Result<RedisBusSnapshot, (StatusCode, Json<ErrorResponse>)> {
    load_active_bus_snapshot_with_keys(
        state,
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY,
        REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY,
        REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
        REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY,
    )
    .await
}

// ── HTTP handlers ─────────────────────────────────────────────────────────────

pub async fn alor_setar_fetch_all_buses(
    State(state): State<AppState>,
) -> Result<Json<GetAllResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_alor_setar_data_fresh(&state).await?;
    let snapshot = load_alor_setar_bus_snapshot(&state).await?;
    let now_ms = now_unix_ms();
    let is_stale = match snapshot.last_ingest_at_unix_ms {
        Some(last_ingest_ms) => now_ms - last_ingest_ms > state.stale_after_ms,
        None => true,
    };

    println!(
        "[mybas-alor-setar] fetch_all_buses: {} active buses",
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

pub async fn alor_setar_get_route_eta(
    Path((route_id, stop_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_alor_setar_data_fresh(&state).await?;
    let snapshot = load_alor_setar_bus_snapshot(&state).await?;
    let visible_buses = filter_non_stationary_buses(&snapshot, state.stationary_window_ms);
    let route_stops =
        get_route_stops_from_cache(&route_id, state.alor_setar_gtfs_cache.as_ref())
            .map_err(|(status, msg)| (status, Json(ErrorResponse { error: msg })))?;

    let eta_results =
        calculate_route_eta_from_stops(&visible_buses, &route_id, &stop_id, &route_stops)?;
    println!(
        "[mybas-alor-setar] get_route_eta route={}, stop={}: {} buses",
        route_id,
        stop_id,
        eta_results.len()
    );
    Ok(Json(eta_results))
}

pub async fn alor_setar_get_stop_eta(
    Path(stop_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_alor_setar_data_fresh(&state).await?;
    let snapshot = load_alor_setar_bus_snapshot(&state).await?;
    let all_eta_results = calculate_stop_eta_from_snapshot(
        &snapshot,
        state.alor_setar_gtfs_cache.as_ref(),
        &stop_id,
        state.stationary_window_ms,
    );

    println!(
        "[mybas-alor-setar] get_stop_eta stop={}: {} incoming buses",
        stop_id,
        all_eta_results.len()
    );
    Ok(Json(all_eta_results))
}

pub async fn alor_setar_get_stop_routes(
    Path(stop_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<StopRoutesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let routes =
        get_routes_for_stop_from_cache(&stop_id, state.alor_setar_gtfs_cache.as_ref())
            .map_err(|(status, message)| (status, Json(ErrorResponse { error: message })))?;

    println!(
        "[mybas-alor-setar] get_stop_routes stop={}: {} routes",
        stop_id,
        routes.len()
    );
    Ok(Json(StopRoutesResponse { stop_id, routes }))
}

pub async fn alor_setar_get_route_stops(
    Path(route_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RouteStopsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match get_route_stops_from_cache(&route_id, state.alor_setar_gtfs_cache.as_ref()) {
        Ok(response) => {
            println!("[mybas-alor-setar] get_route_stops route={}", route_id);
            Ok(Json(response))
        }
        Err((status, message)) => Err((status, Json(ErrorResponse { error: message }))),
    }
}

pub async fn alor_setar_get_route_shape(
    Path(route_id): Path<String>,
    Query(query): Query<RouteShapeQuery>,
    State(state): State<AppState>,
) -> Result<Json<RouteShapeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cache = state.alor_setar_gtfs_cache.as_ref();
    match get_shape_by_route(
        &route_id,
        query.stop_id.as_deref(),
        &cache.context.routes,
        &cache.context.trips_by_route,
        &cache.context.stop_times_by_trip,
        &cache.shapes_by_id,
    ) {
        Ok(response) => Ok(Json(response)),
        Err((status, message)) => Err((status, Json(ErrorResponse { error: message }))),
    }
}

pub async fn alor_setar_get_nearest_stop(
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
        .alor_setar_gtfs_cache
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
        "[mybas-alor-setar] get_nearest_stop lat={}, lon={} -> stop_id={}",
        query.lat, query.lon, response.stop_id
    );
    Ok(Json(response))
}

// ── ETA calculation (reuses core from main.rs) ───────────────────────────────

fn calculate_route_eta_from_stops(
    buses: &[BusPosition],
    route_id: &str,
    target_stop_id: &str,
    route_stops: &RouteStopsResponse,
) -> Result<Vec<BusEta>, (StatusCode, Json<ErrorResponse>)> {
    const DEFAULT_SPEED_KMH: f64 = 20.0;

    let target_stop = route_stops
        .stops
        .iter()
        .find(|s| s.stop_id == target_stop_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!(
                        "Target stop '{}' not found in route '{}'",
                        target_stop_id, route_id
                    ),
                }),
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

        let intermediate_stops: Vec<_> = route_stops
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
