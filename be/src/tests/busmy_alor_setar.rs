use crate::busmy_alor_setar::{
    fetch_alor_setar_bus_positions, REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY,
    REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY, REDIS_MYBAS_ALOR_SETAR_LATEST_KEY,
    REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn alor_setar_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-alor-setar")
}

// ── GTFS cache ────────────────────────────────────────────────────────────────

#[test]
fn alor_setar_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&alor_setar_data_dir()).expect("alor setar cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "alor setar cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "alor setar cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "alor setar cache should have route-by-stop index"
    );
}

#[test]
fn alor_setar_stops_are_in_kedah_region() {
    let cache = GtfsCache::build(&alor_setar_data_dir()).expect("alor setar cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 4.0 && stop.stop_lat <= 7.5,
            "stop {} lat {} outside expected Kedah/north Malaysia range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 99.5 && stop.stop_lon <= 101.5,
            "stop {} lon {} outside expected Kedah/north Malaysia range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn alor_setar_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&alor_setar_data_dir()).expect("alor setar cache should build");

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
fn redis_keys_are_namespaced_separately_from_rapidkl_and_kangar() {
    assert!(
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY.contains("mybas-alor-setar"),
        "latest key must be scoped to mybas-alor-setar"
    );
    assert!(
        REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY.contains("mybas-alor-setar"),
        "last-seen key must be scoped to mybas-alor-setar"
    );
    assert!(
        REDIS_MYBAS_ALOR_SETAR_MOTION_KEY.contains("mybas-alor-setar"),
        "motion key must be scoped to mybas-alor-setar"
    );
    assert!(
        REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY.contains("mybas-alor-setar"),
        "ingest-last key must be scoped to mybas-alor-setar"
    );

    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY,
        crate::REDIS_BUSES_LATEST_KEY,
        "mybas-alor-setar latest key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY,
        crate::REDIS_BUSES_LAST_SEEN_KEY,
        "mybas-alor-setar last-seen key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
        crate::REDIS_BUSES_MOTION_KEY,
        "mybas-alor-setar motion key must not clash with rapidkl"
    );

    use crate::busmy_kangar::{
        REDIS_MYBAS_KANGAR_INGEST_LAST_KEY, REDIS_MYBAS_KANGAR_LAST_SEEN_KEY,
        REDIS_MYBAS_KANGAR_LATEST_KEY, REDIS_MYBAS_KANGAR_MOTION_KEY,
    };
    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY,
        REDIS_MYBAS_KANGAR_LATEST_KEY,
        "mybas-alor-setar latest key must not clash with mybas-kangar"
    );
    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY,
        REDIS_MYBAS_KANGAR_LAST_SEEN_KEY,
        "mybas-alor-setar last-seen key must not clash with mybas-kangar"
    );
    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
        REDIS_MYBAS_KANGAR_MOTION_KEY,
        "mybas-alor-setar motion key must not clash with mybas-kangar"
    );
    assert_ne!(
        REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY,
        REDIS_MYBAS_KANGAR_INGEST_LAST_KEY,
        "mybas-alor-setar ingest key must not clash with mybas-kangar"
    );
}

// ── Integration: live fetch returns BusPosition ──────────────────────────────

#[tokio::test]
async fn fetch_alor_setar_bus_positions_returns_bus_position_data() {
    let positions = fetch_alor_setar_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-alor-setar feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no (mapped from vehicle_id)"
        );
        assert_eq!(
            pos.provider, "MYBAS-ALOR-SETAR",
            "provider must be MYBAS-ALOR-SETAR"
        );
        assert!(
            pos.latitude >= 4.0 && pos.latitude <= 7.5,
            "latitude {} outside expected Kedah/north Malaysia range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 99.5 && pos.longitude <= 101.5,
            "longitude {} outside expected Kedah/north Malaysia range",
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
        "[busmy_alor_setar integration] {} positions fetched, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
