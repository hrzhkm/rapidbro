use crate::busmy_melaka::{
    fetch_melaka_bus_positions, REDIS_MYBAS_MELAKA_INGEST_LAST_KEY,
    REDIS_MYBAS_MELAKA_LAST_SEEN_KEY, REDIS_MYBAS_MELAKA_LATEST_KEY,
    REDIS_MYBAS_MELAKA_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn melaka_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-melaka")
}

// ── GTFS cache ────────────────────────────────────────────────────────────────

#[test]
fn melaka_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&melaka_data_dir()).expect("melaka cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "melaka cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "melaka cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "melaka cache should have route-by-stop index"
    );
}

#[test]
fn melaka_stops_are_in_melaka_region() {
    let cache = GtfsCache::build(&melaka_data_dir()).expect("melaka cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 2.0 && stop.stop_lat <= 2.6,
            "stop {} lat {} outside expected Melaka range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 101.9 && stop.stop_lon <= 102.7,
            "stop {} lon {} outside expected Melaka range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn melaka_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&melaka_data_dir()).expect("melaka cache should build");

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
fn melaka_routes_by_stop_only_contains_routes_with_valid_shapes() {
    let cache = GtfsCache::build(&melaka_data_dir()).expect("melaka cache should build");

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
    assert!(REDIS_MYBAS_MELAKA_LATEST_KEY.contains("mybas-melaka"));
    assert!(REDIS_MYBAS_MELAKA_LAST_SEEN_KEY.contains("mybas-melaka"));
    assert!(REDIS_MYBAS_MELAKA_MOTION_KEY.contains("mybas-melaka"));
    assert!(REDIS_MYBAS_MELAKA_INGEST_LAST_KEY.contains("mybas-melaka"));

    assert_ne!(REDIS_MYBAS_MELAKA_LATEST_KEY, crate::REDIS_BUSES_LATEST_KEY);
    assert_ne!(REDIS_MYBAS_MELAKA_LAST_SEEN_KEY, crate::REDIS_BUSES_LAST_SEEN_KEY);
    assert_ne!(REDIS_MYBAS_MELAKA_MOTION_KEY, crate::REDIS_BUSES_MOTION_KEY);

    use crate::busmy_seremban::{
        REDIS_MYBAS_SEREMBAN_A_LATEST_KEY, REDIS_MYBAS_SEREMBAN_B_LATEST_KEY,
    };
    assert_ne!(REDIS_MYBAS_MELAKA_LATEST_KEY, REDIS_MYBAS_SEREMBAN_A_LATEST_KEY);
    assert_ne!(REDIS_MYBAS_MELAKA_LATEST_KEY, REDIS_MYBAS_SEREMBAN_B_LATEST_KEY);
}

// ── Integration: live fetch ───────────────────────────────────────────────────

#[tokio::test]
async fn fetch_melaka_bus_positions_returns_bus_position_data() {
    let positions = fetch_melaka_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-melaka feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no"
        );
        assert_eq!(pos.provider, "MYBAS-MELAKA", "provider must be MYBAS-MELAKA");
        assert!(
            pos.latitude >= 2.0 && pos.latitude <= 2.6,
            "latitude {} outside expected Melaka range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 101.9 && pos.longitude <= 102.7,
            "longitude {} outside expected Melaka range",
            pos.longitude
        );
        assert!(pos.speed >= 0.0, "speed must be non-negative, got {}", pos.speed);
    }

    println!(
        "[busmy_melaka integration] {} positions fetched, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
