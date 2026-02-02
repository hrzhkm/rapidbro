use base64::Engine;
use flate2::read::GzDecoder;
use futures_util::FutureExt;
use prost::Message;
use rust_socketio::{asynchronous::ClientBuilder, Payload, TransportType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use std::time::Duration;
use axum::{routing::get, Json,Router};

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
    // Get all buses (no route filter)
    // fetch_all_buses().await;
    
    // Or get specific route:
    // get_route_t789().await;

    prasarana_gtfs_data().await;
}

// Fetch all buses - connect without specifying a route to see what we get
async fn fetch_all_buses() {
    println!("Connecting to Socket.IO server: {}", SOCKET_URL);

    let on_any = |event: rust_socketio::Event, payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        async move {
            println!("\n=== Event: {:?} ===", event);
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(encoded_str) = value.as_str() {
                            if let Some(decoded) = decode_bus_data(encoded_str) {
                                // Try to parse as array of bus positions
                                match serde_json::from_str::<Vec<BusPosition>>(&decoded) {
                                    Ok(buses) => {
                                        println!("Received {} buses:", buses.len());
                                        for bus in &buses {
                                            println!("  {} - {} @ ({}, {}) speed: {} km/h", 
                                                bus.route, bus.bus_no, bus.latitude, bus.longitude, bus.speed);
                                        }
                                    }
                                    Err(_) => {
                                        println!("Decoded: {}", decoded);
                                    }
                                }
                            } else {
                                println!("Raw: {}", encoded_str);
                            }
                        } else {
                            println!("JSON: {}", serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()));
                        }
                    }
                }
                Payload::Binary(bin) => println!("Binary: {} bytes", bin.len()),
                _ => println!("Other payload"),
            }
        }
        .boxed()
    };

    let socket = ClientBuilder::new(SOCKET_URL)
        .transport_type(TransportType::Websocket)
        .on_any(on_any)
        .on("connect", |_, socket| {
            async move {
                println!("Connected! Requesting all buses...");
                
                // Empty route to get all buses
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": ""
                });
                
                println!("Emitting onFts-reload: {}", payload);
                if let Err(e) = socket.emit("onFts-reload", payload).await {
                    eprintln!("Failed to emit: {:?}", e);
                }
            }
            .boxed()
        })
        .connect()
        .await;

    match socket {
        Ok(socket) => {
            println!("Socket connected! Waiting for data...\n");
            
            for i in 0..10 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": ""
                });
                
                println!("\n--- Request #{} ---", i + 1);
                if let Err(e) = socket.emit("onFts-reload", payload).await {
                    eprintln!("Failed to emit: {:?}", e);
                    break;
                }
            }
        }
        Err(e) => eprintln!("Failed to connect: {:?}", e),
    }
}

// Get buses for route T789 specifically
#[allow(dead_code)]
async fn get_route_t789() {
    println!("Connecting to Socket.IO server: {}", SOCKET_URL);

    let on_any = |event: rust_socketio::Event, payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        async move {
            println!("\n=== Event: {:?} ===", event);
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(encoded_str) = value.as_str() {
                            if let Some(decoded) = decode_bus_data(encoded_str) {
                                match serde_json::from_str::<serde_json::Value>(&decoded) {
                                    Ok(json) => {
                                        println!("{}", serde_json::to_string_pretty(&json).unwrap());
                                    }
                                    Err(_) => println!("Decoded: {}", decoded),
                                }
                            } else {
                                println!("Raw: {}", encoded_str);
                            }
                        } else {
                            println!("JSON: {}", serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()));
                        }
                    }
                }
                Payload::Binary(bin) => println!("Binary: {} bytes", bin.len()),
                _ => println!("Other payload"),
            }
        }
        .boxed()
    };

    let socket = ClientBuilder::new(SOCKET_URL)
        .transport_type(TransportType::Websocket)
        .on_any(on_any)
        .on("connect", |_, socket| {
            async move {
                println!("Connected! Requesting T789 buses...");
                
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": "T789"
                });
                
                println!("Emitting onFts-reload: {}", payload);
                if let Err(e) = socket.emit("onFts-reload", payload).await {
                    eprintln!("Failed to emit: {:?}", e);
                }
            }
            .boxed()
        })
        .connect()
        .await;

    match socket {
        Ok(socket) => {
            println!("Socket connected! Waiting for T789 data...\n");
            
            for i in 0..10 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                
                let payload = json!({
                    "sid": "",
                    "uid": "",
                    "provider": "RKL",
                    "route": "T789"
                });
                
                println!("\n--- Request #{} ---", i + 1);
                if let Err(e) = socket.emit("onFts-reload", payload).await {
                    eprintln!("Failed to emit: {:?}", e);
                    break;
                }
            }
        }
        Err(e) => eprintln!("Failed to connect: {:?}", e),
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
async fn prasarana_gtfs_data() {
    let endpoint =
        "https://api.data.gov.my/gtfs-realtime/vehicle-position/prasarana?category=rapid-bus-kl";
    let response = reqwest::get(endpoint).await.unwrap();
    let body = response.bytes().await.unwrap();
    let feed = gtfs_realtime::FeedMessage::decode(body).unwrap();

    // Convert entire feed to JSON
    match serde_json::to_string_pretty(&feed) {
        Ok(json) => println!("{}", json),
        Err(e) => {
            eprintln!("Failed to serialize to JSON: {:?}", e);
            println!("GTFS Feed (debug): {:?}", feed);
        }
    }
}
