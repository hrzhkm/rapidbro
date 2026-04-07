use crate::busmy_johor_bharu::{
    fetch_johor_bus_positions, REDIS_MYBAS_JOHOR_INGEST_LAST_KEY,
    REDIS_MYBAS_JOHOR_LAST_SEEN_KEY, REDIS_MYBAS_JOHOR_LATEST_KEY, REDIS_MYBAS_JOHOR_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn johor_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-johor-bharu")
}

// ── GTFS cache ────────────────────────────────────────────────────────────────

#[test]
fn johor_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&johor_data_dir()).expect("johor cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "johor cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "johor cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "johor cache should have route-by-stop index"
    );
}

#[test]
fn johor_stops_are_in_johor_region() {
    let cache = GtfsCache::build(&johor_data_dir()).expect("johor cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 1.2 && stop.stop_lat <= 1.9,
            "stop {} lat {} outside expected Johor range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 103.0 && stop.stop_lon <= 104.2,
            "stop {} lon {} outside expected Johor range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn johor_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&johor_data_dir()).expect("johor cache should build");

    let first_route_id = cache
        .route_stops_by_route
        .keys()
        .next()
        .expect("should have at least one route");

    let route = get_route_stops_from_cache(first_route_id, &cache)
        .expect("route stops should be available");

    assert!(!route.stops.is_empty(), "route should include stops");
    let mut last_seq = 0;
    for stop in &route.stops {
        assert!(stop.sequence >= last_seq, "stop sequences must be sorted");
        last_seq = stop.sequence;
    }
}

#[test]
fn johor_routes_by_stop_only_contains_routes_with_valid_shapes() {
    let cache = GtfsCache::build(&johor_data_dir()).expect("johor cache should build");

    for (stop_id, routes) in &cache.routes_by_stop {
        for route in routes {
            let trips = cache.context.trips_by_route.get(&route.route_id);
            let has_valid_shape = trips.map_or(false, |trips| {
                trips
                    .iter()
                    .any(|t| cache.shapes_by_id.contains_key(&t.shape_id))
            });
            assert!(
                has_valid_shape,
                "stop {} lists route {} which has no valid geometry in shapes.txt",
                stop_id, route.route_id
            );
        }
    }
}

// ── Redis key namespacing ─────────────────────────────────────────────────────

#[test]
fn redis_keys_are_namespaced_separately_from_other_providers() {
    assert!(REDIS_MYBAS_JOHOR_LATEST_KEY.contains("mybas-johor"));
    assert!(REDIS_MYBAS_JOHOR_LAST_SEEN_KEY.contains("mybas-johor"));
    assert!(REDIS_MYBAS_JOHOR_MOTION_KEY.contains("mybas-johor"));
    assert!(REDIS_MYBAS_JOHOR_INGEST_LAST_KEY.contains("mybas-johor"));

    assert_ne!(REDIS_MYBAS_JOHOR_LATEST_KEY, crate::REDIS_BUSES_LATEST_KEY);
    assert_ne!(REDIS_MYBAS_JOHOR_LAST_SEEN_KEY, crate::REDIS_BUSES_LAST_SEEN_KEY);
    assert_ne!(REDIS_MYBAS_JOHOR_MOTION_KEY, crate::REDIS_BUSES_MOTION_KEY);

    use crate::busmy_melaka::REDIS_MYBAS_MELAKA_LATEST_KEY;
    assert_ne!(REDIS_MYBAS_JOHOR_LATEST_KEY, REDIS_MYBAS_MELAKA_LATEST_KEY);

    use crate::busmy_seremban::REDIS_MYBAS_SEREMBAN_A_LATEST_KEY;
    assert_ne!(REDIS_MYBAS_JOHOR_LATEST_KEY, REDIS_MYBAS_SEREMBAN_A_LATEST_KEY);
}

// ── Integration: live fetch ───────────────────────────────────────────────────

#[tokio::test]
async fn fetch_johor_bus_positions_returns_bus_position_data() {
    let positions = fetch_johor_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-johor feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no"
        );
        assert_eq!(pos.provider, "MYBAS-JOHOR", "provider must be MYBAS-JOHOR");
        assert!(
            pos.latitude >= 1.2 && pos.latitude <= 1.9,
            "latitude {} outside expected Johor range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 103.0 && pos.longitude <= 104.2,
            "longitude {} outside expected Johor range",
            pos.longitude
        );
        assert!(pos.speed >= 0.0, "speed must be non-negative, got {}", pos.speed);
    }

    println!(
        "[busmy_johor_bharu integration] {} positions fetched, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
