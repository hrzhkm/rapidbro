use axum::{
    extract::Path,
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
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path as StdPath;
use std::time::Duration;
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

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Clone, Serialize)]
struct BusEta {
    bus_no: String,
    current_lat: f64,
    current_lon: f64,
    current_stop_id: String,
    current_sequence: u32,
    stops_away: u32,
    distance_km: f64,
    speed_kmh: f64,
    eta_minutes: f64,
}

const SOCKET_URL: &str = "https://rapidbus-socketio-avl.prasarana.com.my";
const GTFS_DATA_PATH: &str = "../rapid_kl_data";

#[tokio::main]
async fn main() {

    let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

    let app = Router::new()
    .route("/gtfs", get(prasarana_gtfs_data))
    .route("/get-all", get(fetch_all_buses))
    .route("/get-route-t789", get(get_route_t789))
    .route("/get-t789-eta", get(get_t789_eta))
    .route("/route/{route_id}/stops", get(get_route_stops))
    .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030")
    .await
    .unwrap();

    println!("Server is running on http://localhost:3030");
    axum::serve(listener, app).await.unwrap();
}

// Fetch all buses - connect without specifying a route to see what we get
async fn fetch_all_buses() -> Json<serde_json::Value> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let result = Arc::new(Mutex::new(Vec::new()));
    let result_clone = result.clone();

    let on_any = move |_event: rust_socketio::Event, payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        let result = result_clone.clone();
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(encoded_str) = value.as_str() {
                            if let Some(decoded) = decode_bus_data(encoded_str) {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                                    result.lock().await.push(json);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        .boxed()
    };

    let socket = ClientBuilder::new(SOCKET_URL)
        .transport_type(TransportType::Websocket)
        .on_any(on_any)
        .on("connect", |_, socket| {
            async move {
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": ""
                });
                let _ = socket.emit("onFts-reload", payload).await;
            }
            .boxed()
        })
        .connect()
        .await;

    if let Ok(socket) = socket {
        let payload = json!({
            "sid": "",
            "uid": "",
            "provider": "RKL",
            "route": ""
        });
        let _ = socket.emit("onFts-reload", payload).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let data = result.lock().await;
    println!("Calling fetch_all_buses");
    if data.len() == 1 {
        Json(data[0].clone())
    } else {
        Json(json!(data.clone()))
    }
}

// Get buses for route T789 specifically
async fn get_route_t789() -> Json<serde_json::Value> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let result = Arc::new(Mutex::new(Vec::new()));
    let result_clone = result.clone();

    let on_any = move |_event: rust_socketio::Event, payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        let result = result_clone.clone();
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(encoded_str) = value.as_str() {
                            if let Some(decoded) = decode_bus_data(encoded_str) {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                                    result.lock().await.push(json);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        .boxed()
    };

    let socket = ClientBuilder::new(SOCKET_URL)
        .transport_type(TransportType::Websocket)
        .on_any(on_any)
        .on("connect", |_, socket| {
            async move {
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": "T789"
                });
                let _ = socket.emit("onFts-reload", payload).await;
            }
            .boxed()
        })
        .connect()
        .await;

    if let Ok(socket) = socket {
        let payload = json!({
            "sid": "",
            "uid": "",
            "provider": "RKL",
            "route": "T789"
        });
        let _ = socket.emit("onFts-reload", payload).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let data = result.lock().await;
    println!("Calling get_route_t789");
    if data.len() == 1 {
        Json(data[0].clone())
    } else {
        Json(json!(data.clone()))
    }
}

// Calculate ETA for T789 buses to reach stop 1000838 (KL1397 FLAT PKNS KERINCHI/KL GATEWAY)
async fn get_t789_eta() -> Result<Json<Vec<BusEta>>, (StatusCode, Json<ErrorResponse>)> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    const TARGET_STOP_ID: &str = "1000838";
    const DEFAULT_SPEED_KMH: f64 = 20.0;

    // --- Step 1: Fetch live bus positions via websocket (same as get_route_t789) ---
    let result = Arc::new(Mutex::new(Vec::new()));
    let result_clone = result.clone();

    let on_any = move |_event: rust_socketio::Event, payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        let result = result_clone.clone();
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(encoded_str) = value.as_str() {
                            if let Some(decoded) = decode_bus_data(encoded_str) {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                                    result.lock().await.push(json);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        .boxed()
    };

    let socket = ClientBuilder::new(SOCKET_URL)
        .transport_type(TransportType::Websocket)
        .on_any(on_any)
        .on("connect", |_, socket| {
            async move {
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": "T789"
                });
                let _ = socket.emit("onFts-reload", payload).await;
            }
            .boxed()
        })
        .connect()
        .await;

    if let Ok(socket) = socket {
        let payload = json!({
            "sid": "",
            "uid": "",
            "provider": "RKL",
            "route": "T789"
        });
        let _ = socket.emit("onFts-reload", payload).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let raw_data = result.lock().await;

    // Parse bus positions from the raw JSON
    let mut buses: Vec<BusPosition> = Vec::new();
    for value in raw_data.iter() {
        // The data may come as a single object or an array
        if let Ok(bus) = serde_json::from_value::<BusPosition>(value.clone()) {
            buses.push(bus);
        } else if let Some(arr) = value.as_array() {
            for item in arr {
                if let Ok(bus) = serde_json::from_value::<BusPosition>(item.clone()) {
                    buses.push(bus);
                }
            }
        }
    }

    // --- Step 2: Load route stops for T7890 from GTFS data ---
    let routes = load_routes().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load routes: {}", e),
        }))
    })?;

    let trips_by_route = load_trips().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load trips: {}", e),
        }))
    })?;

    let stop_times_by_trip = load_stop_times().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load stop times: {}", e),
        }))
    })?;

    let stops_map = load_stops().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load stops: {}", e),
        }))
    })?;

    let route_stops = get_stops_by_route("T7890", &routes, &trips_by_route, &stop_times_by_trip, &stops_map)
        .map_err(|(status, msg)| (status, Json(ErrorResponse { error: msg })))?;

    // Find the target stop's sequence
    let target_stop = route_stops.stops.iter()
        .find(|s| s.stop_id == TARGET_STOP_ID)
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: format!("Target stop '{}' not found in route stops", TARGET_STOP_ID),
            }))
        })?;
    let target_sequence = target_stop.sequence;

    // --- Step 3: Calculate ETA for each bus ---
    let mut eta_results: Vec<BusEta> = Vec::new();

    for bus in &buses {
        let bus_stop_id = match &bus.busstop_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => continue, // Skip buses without a known stop
        };

        // Find the bus's current stop in the route sequence
        let current_stop = match route_stops.stops.iter().find(|s| s.stop_id == bus_stop_id) {
            Some(stop) => stop,
            None => continue, // Bus is at a stop not on this route, skip
        };

        let current_sequence = current_stop.sequence;

        // Skip buses that have already passed the target stop
        if current_sequence >= target_sequence {
            continue;
        }

        let stops_away = target_sequence - current_sequence;

        // Calculate distance: bus position -> next stops -> ... -> target stop
        // Get the stops between current and target (exclusive of current, inclusive of target)
        let intermediate_stops: Vec<&StopWithDetails> = route_stops.stops.iter()
            .filter(|s| s.sequence > current_sequence && s.sequence <= target_sequence)
            .collect();

        let mut total_distance_km = 0.0;
        let mut prev_lat = bus.latitude;
        let mut prev_lon = bus.longitude;

        for stop in &intermediate_stops {
            total_distance_km += haversine_distance(prev_lat, prev_lon, stop.stop_lat, stop.stop_lon);
            prev_lat = stop.stop_lat;
            prev_lon = stop.stop_lon;
        }

        // Calculate ETA: use bus speed if available, otherwise fallback
        let speed = if bus.speed > 0.0 { bus.speed } else { DEFAULT_SPEED_KMH };
        let eta_minutes = (total_distance_km / speed) * 60.0;

        eta_results.push(BusEta {
            bus_no: bus.bus_no.clone(),
            current_lat: bus.latitude,
            current_lon: bus.longitude,
            current_stop_id: bus_stop_id,
            current_sequence,
            stops_away,
            distance_km: (total_distance_km * 100.0).round() / 100.0,
            speed_kmh: bus.speed,
            eta_minutes: (eta_minutes * 10.0).round() / 10.0,
        });
    }

    // Sort by ETA (closest first)
    eta_results.sort_by(|a, b| a.eta_minutes.partial_cmp(&b.eta_minutes).unwrap_or(std::cmp::Ordering::Equal));

    println!("Calling get_t789_eta: found {} buses with ETA", eta_results.len());
    Ok(Json(eta_results))
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
        trips_by_route.entry(trip.route_id.clone()).or_default().push(trip);
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
        stop_times_by_trip.entry(stop_time.trip_id.clone()).or_default().push(stop_time);
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
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Route '{}' not found", route_id)))?;

    // Get trips for this route
    let trips = trips_by_route
        .get(route_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("No trips found for route '{}'", route_id)))?;

    // Get the first trip's stop times
    let first_trip = &trips[0];
    let stop_times = stop_times_by_trip
        .get(&first_trip.trip_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("No stop times found for trip '{}'", first_trip.trip_id)))?;

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
async fn get_route_stops(Path(route_id): Path<String>) -> Result<Json<RouteStopsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Load GTFS data
    let routes = match load_routes() {
        Ok(r) => r,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to load routes: {}", e),
            })));
        }
    };

    let trips_by_route = match load_trips() {
        Ok(t) => t,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to load trips: {}", e),
            })));
        }
    };

    let stop_times_by_trip = match load_stop_times() {
        Ok(st) => st,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to load stop times: {}", e),
            })));
        }
    };

    let stops_map = match load_stops() {
        Ok(s) => s,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to load stops: {}", e),
            })));
        }
    };

    match get_stops_by_route(&route_id, &routes, &trips_by_route, &stop_times_by_trip, &stops_map) {
        Ok(response) => {
            println!("Calling get_route_stops for route_id={}", route_id);
            Ok(Json(response))
        }
        Err((status, message)) => Err((status, Json(ErrorResponse { error: message }))),
    }
}
