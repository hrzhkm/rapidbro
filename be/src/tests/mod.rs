mod rapidkl;
mod busmy_kangar;
mod busmy_alor_setar;
mod busmy_kota_bharu;
mod busmy_kuala_terengganu;
mod busmy_ipoh;

use super::*;
use crate::rapidkl::{BusMotionState, BusPosition};

fn mock_bus(bus_no: &str) -> BusPosition {
    BusPosition {
        dt_received: None,
        dt_gps: None,
        latitude: 3.1,
        longitude: 101.6,
        dir: None,
        speed: 0.0,
        angle: 0.0,
        route: "T7890".to_string(),
        bus_no: bus_no.to_string(),
        trip_no: None,
        captain_id: None,
        trip_rev_kind: None,
        engine_status: 1,
        accessibility: 1,
        busstop_id: None,
        provider: "RKL".to_string(),
    }
}

#[test]
fn stationarity_boundary_uses_configured_window() {
    use std::collections::HashMap;

    let window_ms = 300_000;
    let now_ms = 1_000_000;
    let mut motion_states = HashMap::new();
    motion_states.insert(
        "BUS-A".to_string(),
        BusMotionState {
            reference_lat: 3.1,
            reference_lon: 101.6,
            stationary_since_unix_ms: Some(now_ms - window_ms),
        },
    );
    motion_states.insert(
        "BUS-B".to_string(),
        BusMotionState {
            reference_lat: 3.1,
            reference_lon: 101.6,
            stationary_since_unix_ms: Some(now_ms - (window_ms - 1)),
        },
    );

    let snapshot = RedisBusSnapshot {
        buses: vec![mock_bus("BUS-A"), mock_bus("BUS-B"), mock_bus("BUS-C")],
        motion_states,
        active_bus_count: 3,
        last_ingest_at_unix_ms: Some(now_ms),
    };

    assert!(
        is_bus_stationary(&snapshot, "BUS-A", now_ms, window_ms),
        "bus exactly at threshold should be treated as stationary"
    );
    assert!(
        !is_bus_stationary(&snapshot, "BUS-B", now_ms, window_ms),
        "bus below threshold should not be treated as stationary"
    );
    assert!(
        !is_bus_stationary(&snapshot, "BUS-C", now_ms, window_ms),
        "bus with no motion state should not be treated as stationary"
    );
}
