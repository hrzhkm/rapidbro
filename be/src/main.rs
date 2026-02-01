use base64::Engine;
use flate2::read::GzDecoder;
use futures_util::FutureExt;
use prost::Message;
use regex::Regex;
use rust_socketio::{asynchronous::ClientBuilder, Payload, TransportType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusRoute {
    pub id: String,      // Internal ID (e.g., "733")
    pub name: String,    // Route number/name (e.g., "T789")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub sid: String,
    pub provider: String,
}

#[tokio::main]
async fn main() {
    // Just connect to websocket directly and see what data we get
    simple_websocket_test().await;
}

// Simple websocket test - just connect and see what data comes back
async fn simple_websocket_test() {
    let socket_url = "https://rapidbus-socketio-avl.prasarana.com.my";
    
    println!("Connecting to Socket.IO server: {}", socket_url);

    // Callback for ANY event
    let on_any = |event: rust_socketio::Event, payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        async move {
            println!("\n=== Event: {:?} ===", event);
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(encoded_str) = value.as_str() {
                            // Try to decode as base64+gzip
                            if let Some(decoded) = decode_bus_data(encoded_str) {
                                println!("Decoded data: {}", decoded);
                            } else {
                                println!("Raw string: {}", encoded_str);
                            }
                        } else {
                            println!("JSON: {}", serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()));
                        }
                    }
                }
                Payload::Binary(bin) => {
                    println!("Binary: {} bytes", bin.len());
                }
                _ => {
                    println!("Other payload type");
                }
            }
        }
        .boxed()
    };

    // Build and connect
    let socket = ClientBuilder::new(socket_url)
        .transport_type(TransportType::Websocket)
        .on_any(on_any)
        .on("connect", |_, socket| {
            async move {
                println!("Connected! Sending test request...");
                
                // Try requesting data for route T789
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
            println!("Socket connected! Waiting for data...\n");
            
            // Keep alive and request updates every 5 seconds
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
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
        }
    }
}

// Fetch all available routes from the kiosk page
#[allow(dead_code)]
async fn fetch_all_routes() -> Result<(Vec<BusRoute>, SessionData), Box<dyn std::error::Error>> {
    let kiosk_url = "https://myrapidbus.prasarana.com.my/kiosk";
    
    println!("Fetching routes from kiosk page...");
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let response = client.get(kiosk_url).send().await?;
    let html = response.text().await?;

    // Debug: Check if we're getting blocked
    if html.contains("Incapsula") || html.contains("captcha") || html.contains("blocked") {
        println!("WARNING: Request blocked by bot protection. HTML length: {}", html.len());
        println!("First 500 chars: {}", &html[..html.len().min(500)]);
        return Err("Blocked by bot protection - website requires browser session".into());
    }

    // Extract session data
    let sid_regex = Regex::new(r"var\s+sid\s*=\s*'([^']+)'")?;
    let prm_regex = Regex::new(r"var\s+prm\s*=\s*'([^']*)'")?;
    
    let sid = sid_regex
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    
    let provider = prm_regex
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "RKL".to_string());

    // Extract routes from select options
    // Pattern: <option value="733">T789</option>
    let option_regex = Regex::new(r#"<option value="(\d+)">([^<]+)</option>"#)?;
    
    let routes: Vec<BusRoute> = option_regex
        .captures_iter(&html)
        .map(|cap| BusRoute {
            id: cap.get(1).unwrap().as_str().to_string(),
            name: cap.get(2).unwrap().as_str().to_string(),
        })
        .collect();

    let session = SessionData { sid, provider };
    
    Ok((routes, session))
}

// Data OpenDOSM Prasarana - guna protobuf
#[allow(dead_code)]
async fn prasarana_data() {
    let endpoint =
        "https://api.data.gov.my/gtfs-realtime/vehicle-position/prasarana?category=rapid-bus-kl";
    let response = reqwest::get(endpoint).await.unwrap();
    let body = response.bytes().await.unwrap();
    let feed = gtfs_realtime::FeedMessage::decode(body).unwrap();
    println!("Feed: {:?}", feed);
}

// Decode base64 + gzip compressed data from the websocket
fn decode_bus_data(encoded: &str) -> Option<String> {
    // Decode base64
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;

    // Decompress gzip
    let mut decoder = GzDecoder::new(&decoded[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).ok()?;

    Some(decompressed)
}

// Prasarana Websocket data - live bus positions (with pre-fetched session)
async fn prasarana_websocket_with_session(session: &SessionData, route: &str) {
    let socket_url = "https://rapidbus-socketio-avl.prasarana.com.my";

    println!("Connecting to Socket.IO server: {}", socket_url);

    let sid_clone = session.sid.clone();
    let prm_clone = session.provider.clone();
    let route_clone = route.to_string();

    // Callback for receiving bus data
    let callback = |payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        // The data comes as a base64+gzip encoded string
                        if let Some(encoded_str) = value.as_str() {
                            match decode_bus_data(encoded_str) {
                                Some(decoded) => {
                                    // Try to parse as JSON
                                    match serde_json::from_str::<serde_json::Value>(&decoded) {
                                        Ok(json_data) => {
                                            println!("\n=== Live Bus Data ===");
                                            println!(
                                                "{}",
                                                serde_json::to_string_pretty(&json_data).unwrap()
                                            );
                                        }
                                        Err(_) => {
                                            // Not JSON, print raw
                                            println!("\n=== Raw Data ===");
                                            println!("{}", decoded);
                                        }
                                    }
                                }
                                None => {
                                    println!("Failed to decode: {}", encoded_str);
                                }
                            }
                        } else {
                            println!(
                                "Non-string data: {}",
                                serde_json::to_string_pretty(&value)
                                    .unwrap_or_else(|_| value.to_string())
                            );
                        }
                    }
                }
                Payload::Binary(bin) => {
                    println!("Received binary data: {} bytes", bin.len());
                }
                _ => {}
            }
        }
        .boxed()
    };

    let sid_for_connect = sid_clone.clone();
    let prm_for_connect = prm_clone.clone();
    let route_for_connect = route_clone.clone();

    // Build and connect the socket
    let socket = ClientBuilder::new(socket_url)
        .transport_type(TransportType::Websocket)
        .on("onFts-client", callback)
        .on("error", |err, _| {
            async move {
                eprintln!("Socket error: {:?}", err);
            }
            .boxed()
        })
        .on("connect", move |_, socket| {
            let sid = sid_for_connect.clone();
            let prm = prm_for_connect.clone();
            let route = route_for_connect.clone();
            async move {
                println!("Connected to WebSocket server!");

                // Emit the onFts-reload event to request data
                let payload = json!({
                    "sid": sid,
                    "uid": "",
                    "provider": prm,
                    "route": route
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
            println!("Socket connected successfully!");

            // Keep connection alive and periodically request updates
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                let payload = json!({
                    "sid": sid_clone,
                    "uid": "",
                    "provider": prm_clone,
                    "route": route_clone
                });

                if let Err(e) = socket.emit("onFts-reload", payload).await {
                    eprintln!("Failed to emit reload: {:?}", e);
                    break;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
        }
    }
}

// Legacy function - kept for reference
#[allow(dead_code)]
async fn prasarana_websocket() {
    // Fetch a specific route page to get session data
    // Using route 300 (KLCC-Bukit Bintang) as example
    let route_url = "https://myrapidbus.prasarana.com.my/kiosk?route=733&bus=";

    println!("Fetching route page to get session data...");
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .build()
        .unwrap();

    let response = client.get(route_url).send().await.unwrap();
    let html = response.text().await.unwrap();

    // Extract session ID and route info from the page
    let sid_regex = Regex::new(r"var\s+sid\s*=\s*'([^']+)'").unwrap();
    let prm_regex = Regex::new(r"var\s+prm\s*=\s*'([^']*)'").unwrap();
    let route_regex = Regex::new(r"var\s+no_route\s*=\s*'([^']*)'").unwrap();

    let sid = sid_regex
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "".to_string());

    let prm = prm_regex
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "rapidkl".to_string());

    let no_route = route_regex
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "T789".to_string());

    println!(
        "Extracted - sid: {}, prm: {}, no_route: {}",
        sid, prm, no_route
    );

    // Connect to Socket.IO server
    let socket_url = "https://rapidbus-socketio-avl.prasarana.com.my";

    println!("Connecting to Socket.IO server: {}", socket_url);

    let sid_clone = sid.clone();
    let prm_clone = prm.clone();
    let no_route_clone = no_route.clone();

    // Callback for receiving bus data
    let callback = |payload: Payload, _socket: rust_socketio::asynchronous::Client| {
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        // The data comes as a base64+gzip encoded string
                        if let Some(encoded_str) = value.as_str() {
                            match decode_bus_data(encoded_str) {
                                Some(decoded) => {
                                    // Try to parse as JSON
                                    match serde_json::from_str::<serde_json::Value>(&decoded) {
                                        Ok(json_data) => {
                                            println!("\n=== Live Bus Data ===");
                                            println!(
                                                "{}",
                                                serde_json::to_string_pretty(&json_data).unwrap()
                                            );
                                        }
                                        Err(_) => {
                                            // Not JSON, print raw
                                            println!("\n=== Raw Data ===");
                                            println!("{}", decoded);
                                        }
                                    }
                                }
                                None => {
                                    println!("Failed to decode: {}", encoded_str);
                                }
                            }
                        } else {
                            println!(
                                "Non-string data: {}",
                                serde_json::to_string_pretty(&value)
                                    .unwrap_or_else(|_| value.to_string())
                            );
                        }
                    }
                }
                Payload::Binary(bin) => {
                    println!("Received binary data: {} bytes", bin.len());
                }
                _ => {}
            }
        }
        .boxed()
    };

    // Build and connect the socket
    let socket = ClientBuilder::new(socket_url)
        .transport_type(TransportType::Websocket)
        .on("onFts-client", callback)
        .on("error", |err, _| {
            async move {
                eprintln!("Socket error: {:?}", err);
            }
            .boxed()
        })
        .on("connect", move |_, socket| {
            let sid = sid_clone.clone();
            let prm = prm_clone.clone();
            let no_route = no_route_clone.clone();
            async move {
                println!("Connected to WebSocket server!");

                // Emit the onFts-reload event to request data
                let payload = json!({
                    "sid": sid,
                    "uid": "",
                    "provider": prm,
                    "route": no_route
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
            println!("Socket connected successfully!");

            // Keep connection alive and periodically request updates
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                let payload = json!({
                    "sid": sid,
                    "uid": "",
                    "provider": prm,
                    "route": no_route
                });

                if let Err(e) = socket.emit("onFts-reload", payload).await {
                    eprintln!("Failed to emit reload: {:?}", e);
                    break;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
        }
    }
}
