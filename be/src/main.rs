use base64::Engine;
use flate2::read::GzDecoder;
use futures_util::FutureExt;
use prost::Message;
use rust_socketio::{asynchronous::ClientBuilder, Payload, TransportType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use std::time::Duration;
use axum::{routing::get, Json, Router};
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

const SOCKET_URL: &str = "https://rapidbus-socketio-avl.prasarana.com.my";

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
