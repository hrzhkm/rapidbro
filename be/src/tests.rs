use super::*;

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
fn gtfs_cache_builds_with_expected_indexes() {
    let cache = GtfsCache::build().expect("cache should build from GTFS files");

    assert!(
        cache.route_stops_by_route.contains_key("T7890"),
        "expected T7890 route to be indexed"
    );
    assert!(
        cache.context.stops_map.contains_key("1000838"),
        "expected stop 1000838 to exist"
    );
}

#[test]
fn route_stops_from_cache_are_sequence_sorted() {
    let cache = GtfsCache::build().expect("cache should build from GTFS files");
    let route = get_route_stops_from_cache("T7890", &cache)
        .expect("T7890 route stops should be available");

    assert!(!route.stops.is_empty(), "route should include stops");
    let mut last_seq = 0;
    for stop in route.stops {
        assert!(stop.sequence >= last_seq, "stop sequences must be sorted");
        last_seq = stop.sequence;
    }
}

#[test]
fn routes_for_stop_from_cache_returns_sorted_summaries() {
    let cache = GtfsCache::build().expect("cache should build from GTFS files");
    let routes = get_routes_for_stop_from_cache("1000838", &cache)
        .expect("routes for stop 1000838 should be available");

    assert!(!routes.is_empty(), "stop should have at least one route");
    let mut sorted = routes.clone();
    sorted.sort_by(|a, b| {
        a.route_short_name
            .cmp(&b.route_short_name)
            .then(a.route_id.cmp(&b.route_id))
    });
    assert_eq!(routes, sorted, "routes must be stable-sorted");
}

#[test]
fn stationarity_boundary_uses_configured_window() {
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
