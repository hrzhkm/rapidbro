use crate::busmy_seremban::{
    fetch_seremban_a_bus_positions, fetch_seremban_b_bus_positions,
    REDIS_MYBAS_SEREMBAN_A_INGEST_LAST_KEY, REDIS_MYBAS_SEREMBAN_A_LAST_SEEN_KEY,
    REDIS_MYBAS_SEREMBAN_A_LATEST_KEY, REDIS_MYBAS_SEREMBAN_A_MOTION_KEY,
    REDIS_MYBAS_SEREMBAN_B_INGEST_LAST_KEY, REDIS_MYBAS_SEREMBAN_B_LAST_SEEN_KEY,
    REDIS_MYBAS_SEREMBAN_B_LATEST_KEY, REDIS_MYBAS_SEREMBAN_B_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn seremban_a_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-seremban-a")
}

fn seremban_b_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-seremban-b")
}

// ── GTFS cache — Seremban A ───────────────────────────────────────────────────

#[test]
fn seremban_a_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&seremban_a_data_dir()).expect("seremban-a cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "seremban-a cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "seremban-a cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "seremban-a cache should have route-by-stop index"
    );
}

#[test]
fn seremban_a_stops_are_in_negeri_sembilan_region() {
    let cache = GtfsCache::build(&seremban_a_data_dir()).expect("seremban-a cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 2.3 && stop.stop_lat <= 3.2,
            "stop {} lat {} outside expected Negeri Sembilan range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 101.5 && stop.stop_lon <= 102.5,
            "stop {} lon {} outside expected Negeri Sembilan range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn seremban_a_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&seremban_a_data_dir()).expect("seremban-a cache should build");

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
fn seremban_a_routes_by_stop_only_contains_routes_with_valid_shapes() {
    let cache = GtfsCache::build(&seremban_a_data_dir()).expect("seremban-a cache should build");

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

// ── GTFS cache — Seremban B ───────────────────────────────────────────────────

#[test]
fn seremban_b_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&seremban_b_data_dir()).expect("seremban-b cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "seremban-b cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "seremban-b cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "seremban-b cache should have route-by-stop index"
    );
}

#[test]
fn seremban_b_stops_are_in_negeri_sembilan_region() {
    let cache = GtfsCache::build(&seremban_b_data_dir()).expect("seremban-b cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 2.3 && stop.stop_lat <= 3.2,
            "stop {} lat {} outside expected Negeri Sembilan range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 101.5 && stop.stop_lon <= 102.5,
            "stop {} lon {} outside expected Negeri Sembilan range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn seremban_b_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&seremban_b_data_dir()).expect("seremban-b cache should build");

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
fn seremban_b_routes_by_stop_only_contains_routes_with_valid_shapes() {
    let cache = GtfsCache::build(&seremban_b_data_dir()).expect("seremban-b cache should build");

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
fn redis_keys_are_namespaced_separately() {
    // A keys scoped correctly
    assert!(REDIS_MYBAS_SEREMBAN_A_LATEST_KEY.contains("mybas-seremban-a"));
    assert!(REDIS_MYBAS_SEREMBAN_A_LAST_SEEN_KEY.contains("mybas-seremban-a"));
    assert!(REDIS_MYBAS_SEREMBAN_A_MOTION_KEY.contains("mybas-seremban-a"));
    assert!(REDIS_MYBAS_SEREMBAN_A_INGEST_LAST_KEY.contains("mybas-seremban-a"));

    // B keys scoped correctly
    assert!(REDIS_MYBAS_SEREMBAN_B_LATEST_KEY.contains("mybas-seremban-b"));
    assert!(REDIS_MYBAS_SEREMBAN_B_LAST_SEEN_KEY.contains("mybas-seremban-b"));
    assert!(REDIS_MYBAS_SEREMBAN_B_MOTION_KEY.contains("mybas-seremban-b"));
    assert!(REDIS_MYBAS_SEREMBAN_B_INGEST_LAST_KEY.contains("mybas-seremban-b"));

    // A and B must not clash with each other
    assert_ne!(REDIS_MYBAS_SEREMBAN_A_LATEST_KEY, REDIS_MYBAS_SEREMBAN_B_LATEST_KEY);
    assert_ne!(REDIS_MYBAS_SEREMBAN_A_LAST_SEEN_KEY, REDIS_MYBAS_SEREMBAN_B_LAST_SEEN_KEY);
    assert_ne!(REDIS_MYBAS_SEREMBAN_A_MOTION_KEY, REDIS_MYBAS_SEREMBAN_B_MOTION_KEY);
    assert_ne!(REDIS_MYBAS_SEREMBAN_A_INGEST_LAST_KEY, REDIS_MYBAS_SEREMBAN_B_INGEST_LAST_KEY);

    // Must not clash with rapidkl
    assert_ne!(REDIS_MYBAS_SEREMBAN_A_LATEST_KEY, crate::REDIS_BUSES_LATEST_KEY);
    assert_ne!(REDIS_MYBAS_SEREMBAN_B_LATEST_KEY, crate::REDIS_BUSES_LATEST_KEY);

    // Must not clash with kota-bharu
    use crate::busmy_kota_bharu::REDIS_MYBAS_KOTA_BHARU_LATEST_KEY;
    assert_ne!(REDIS_MYBAS_SEREMBAN_A_LATEST_KEY, REDIS_MYBAS_KOTA_BHARU_LATEST_KEY);
    assert_ne!(REDIS_MYBAS_SEREMBAN_B_LATEST_KEY, REDIS_MYBAS_KOTA_BHARU_LATEST_KEY);
}

// ── Integration: live fetch ───────────────────────────────────────────────────

#[tokio::test]
async fn fetch_seremban_a_bus_positions_returns_bus_position_data() {
    let positions = fetch_seremban_a_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-seremban-a feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no"
        );
        assert_eq!(pos.provider, "MYBAS-SEREMBAN-A", "provider must be MYBAS-SEREMBAN-A");
        assert!(
            pos.latitude >= 2.3 && pos.latitude <= 3.2,
            "latitude {} outside expected Negeri Sembilan range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 101.5 && pos.longitude <= 102.5,
            "longitude {} outside expected Negeri Sembilan range",
            pos.longitude
        );
        assert!(pos.speed >= 0.0, "speed must be non-negative, got {}", pos.speed);
    }

    println!(
        "[busmy_seremban integration] seremban-a: {} positions, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}

#[tokio::test]
async fn fetch_seremban_b_bus_positions_returns_bus_position_data() {
    let positions = fetch_seremban_b_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-seremban-b feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no"
        );
        assert_eq!(pos.provider, "MYBAS-SEREMBAN-B", "provider must be MYBAS-SEREMBAN-B");
        assert!(
            pos.latitude >= 2.3 && pos.latitude <= 3.2,
            "latitude {} outside expected Negeri Sembilan range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 101.5 && pos.longitude <= 102.5,
            "longitude {} outside expected Negeri Sembilan range",
            pos.longitude
        );
        assert!(pos.speed >= 0.0, "speed must be non-negative, got {}", pos.speed);
    }

    println!(
        "[busmy_seremban integration] seremban-b: {} positions, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
