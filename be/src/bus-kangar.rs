use prost::Message;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;

use crate::{now_unix_ms, AppState};

// ── MybasKangar constants ─────────────────────────────────────────────────────

const MYBAS_KANGAR_GTFS_RT_URL: &str =
    "https://api.data.gov.my/gtfs-realtime/vehicle-position/mybas-kangar";
const POLL_INTERVAL_SECONDS: u64 = 15;

pub const REDIS_MYBAS_KANGAR_LATEST_KEY: &str = "rapidbro:mybas-kangar:buses:latest";
pub const REDIS_MYBAS_KANGAR_LAST_SEEN_KEY: &str = "rapidbro:mybas-kangar:buses:last_seen";
pub const REDIS_MYBAS_KANGAR_INGEST_LAST_KEY: &str =
    "rapidbro:mybas-kangar:ingestor:last_ingest_at";

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MybasKangarPosition {
    pub vehicle_id: String,
    pub label: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub speed: f64,
    pub bearing: f64,
    pub route_id: Option<String>,
    pub trip_id: Option<String>,
    pub timestamp: Option<u64>,
}

// ── Fetcher ───────────────────────────────────────────────────────────────────

pub async fn fetch_mybas_kangar_positions(
) -> Result<Vec<MybasKangarPosition>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(MYBAS_KANGAR_GTFS_RT_URL).await?;
    let body = response.bytes().await?;
    let feed = gtfs_realtime::FeedMessage::decode(body)?;

    let positions: Vec<MybasKangarPosition> = feed
        .entity
        .into_iter()
        .filter_map(|entity| {
            let vehicle = entity.vehicle?;
            let position = vehicle.position?;

            let (vehicle_id, label) = vehicle
                .vehicle
                .map(|v| (v.id.unwrap_or_default(), v.label))
                .unwrap_or_default();

            Some(MybasKangarPosition {
                vehicle_id,
                label,
                latitude: position.latitude as f64,
                longitude: position.longitude as f64,
                speed: position.speed.unwrap_or(0.0) as f64,
                bearing: position.bearing.unwrap_or(0.0) as f64,
                route_id: vehicle.trip.as_ref().and_then(|t| t.route_id.clone()),
                trip_id: vehicle.trip.as_ref().and_then(|t| t.trip_id.clone()),
                timestamp: vehicle.timestamp,
            })
        })
        .collect();

    Ok(positions)
}

// ── Polling ingestor ──────────────────────────────────────────────────────────

pub async fn run_mybas_kangar_ingestor(state: AppState) {
    let mut interval = time::interval(Duration::from_secs(POLL_INTERVAL_SECONDS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    println!("[mybas-kangar] Ingestor started, polling every {}s", POLL_INTERVAL_SECONDS);

    loop {
        interval.tick().await;

        let positions = match fetch_mybas_kangar_positions().await {
            Ok(positions) => {
                println!("[mybas-kangar] Fetched {} bus positions", positions.len());
                positions
            }
            Err(error) => {
                eprintln!("[mybas-kangar] Fetch failed: {}", error);
                continue;
            }
        };

        if positions.is_empty() {
            continue;
        }

        let now_ms = now_unix_ms();

        let mut redis_conn = match state.redis_client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(error) => {
                eprintln!("[mybas-kangar] Redis connection failed: {}", error);
                continue;
            }
        };

        let mut pipe = redis::pipe();
        for pos in &positions {
            if pos.vehicle_id.is_empty() {
                continue;
            }
            let serialized = match serde_json::to_string(pos) {
                Ok(value) => value,
                Err(_) => continue,
            };
            pipe.cmd("HSET")
                .arg(REDIS_MYBAS_KANGAR_LATEST_KEY)
                .arg(&pos.vehicle_id)
                .arg(&serialized)
                .ignore();
            pipe.cmd("ZADD")
                .arg(REDIS_MYBAS_KANGAR_LAST_SEEN_KEY)
                .arg(now_ms)
                .arg(&pos.vehicle_id)
                .ignore();
        }
        pipe.cmd("SET")
            .arg(REDIS_MYBAS_KANGAR_INGEST_LAST_KEY)
            .arg(now_ms)
            .ignore();

        if let Err(error) = pipe.query_async::<()>(&mut redis_conn).await {
            eprintln!("[mybas-kangar] Redis write failed: {}", error);
        }
    }
}
