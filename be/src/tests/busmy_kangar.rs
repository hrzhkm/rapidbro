use crate::busmy_kangar::{
    fetch_kangar_bus_positions, REDIS_MYBAS_KANGAR_INGEST_LAST_KEY,
    REDIS_MYBAS_KANGAR_LAST_SEEN_KEY, REDIS_MYBAS_KANGAR_LATEST_KEY,
    REDIS_MYBAS_KANGAR_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn kangar_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-kangar")
}

// ── GTFS cache ────────────────────────────────────────────────────────────────

#[test]
fn kangar_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&kangar_data_dir()).expect("kangar cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "kangar cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "kangar cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "kangar cache should have route-by-stop index"
    );
}

#[test]
fn kangar_stops_are_in_perlis_region() {
    let cache = GtfsCache::build(&kangar_data_dir()).expect("kangar cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 4.0 && stop.stop_lat <= 7.0,
            "stop {} lat {} outside expected Perlis/north Malaysia range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 99.0 && stop.stop_lon <= 102.0,
            "stop {} lon {} outside expected Perlis/north Malaysia range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn kangar_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&kangar_data_dir()).expect("kangar cache should build");

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
        assert!(
            stop.sequence >= last_seq,
            "stop sequences must be sorted"
        );
        last_seq = stop.sequence;
    }
}

// ── Redis key namespacing ─────────────────────────────────────────────────────

#[test]
fn redis_keys_are_namespaced_separately_from_rapidkl() {
    assert!(
        REDIS_MYBAS_KANGAR_LATEST_KEY.contains("mybas-kangar"),
        "latest key must be scoped to mybas-kangar"
    );
    assert!(
        REDIS_MYBAS_KANGAR_LAST_SEEN_KEY.contains("mybas-kangar"),
        "last-seen key must be scoped to mybas-kangar"
    );
    assert!(
        REDIS_MYBAS_KANGAR_MOTION_KEY.contains("mybas-kangar"),
        "motion key must be scoped to mybas-kangar"
    );
    assert!(
        REDIS_MYBAS_KANGAR_INGEST_LAST_KEY.contains("mybas-kangar"),
        "ingest-last key must be scoped to mybas-kangar"
    );

    assert_ne!(
        REDIS_MYBAS_KANGAR_LATEST_KEY,
        crate::REDIS_BUSES_LATEST_KEY,
        "mybas-kangar latest key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_KANGAR_LAST_SEEN_KEY,
        crate::REDIS_BUSES_LAST_SEEN_KEY,
        "mybas-kangar last-seen key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_KANGAR_MOTION_KEY,
        crate::REDIS_BUSES_MOTION_KEY,
        "mybas-kangar motion key must not clash with rapidkl"
    );
}

// ── Integration: live fetch returns BusPosition ──────────────────────────────

#[tokio::test]
async fn fetch_kangar_bus_positions_returns_bus_position_data() {
    let positions = fetch_kangar_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-kangar feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no (mapped from vehicle_id)"
        );
        assert_eq!(
            pos.provider, "MYBAS-KANGAR",
            "provider must be MYBAS-KANGAR"
        );
        assert!(
            pos.latitude >= 4.0 && pos.latitude <= 7.0,
            "latitude {} outside expected Perlis/north Malaysia range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 99.0 && pos.longitude <= 102.0,
            "longitude {} outside expected Perlis/north Malaysia range",
            pos.longitude
        );
        // Speed was converted from m/s to km/h
        assert!(
            pos.speed >= 0.0,
            "speed must be non-negative, got {}",
            pos.speed
        );
    }

    println!(
        "[busmy_kangar integration] {} positions fetched, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
