use crate::busmy_kota_bharu::{
    fetch_kota_bharu_bus_positions, REDIS_MYBAS_KOTA_BHARU_INGEST_LAST_KEY,
    REDIS_MYBAS_KOTA_BHARU_LAST_SEEN_KEY, REDIS_MYBAS_KOTA_BHARU_LATEST_KEY,
    REDIS_MYBAS_KOTA_BHARU_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn kota_bharu_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-kota-bharu")
}

// ── GTFS cache ────────────────────────────────────────────────────────────────

#[test]
fn kota_bharu_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&kota_bharu_data_dir()).expect("kota bharu cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "kota bharu cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "kota bharu cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "kota bharu cache should have route-by-stop index"
    );
}

#[test]
fn kota_bharu_stops_are_in_kelantan_region() {
    let cache = GtfsCache::build(&kota_bharu_data_dir()).expect("kota bharu cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 5.0 && stop.stop_lat <= 7.0,
            "stop {} lat {} outside expected Kelantan range",
            stop.stop_id,
            stop.stop_lat
        );
        assert!(
            stop.stop_lon >= 101.5 && stop.stop_lon <= 103.0,
            "stop {} lon {} outside expected Kelantan range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn kota_bharu_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&kota_bharu_data_dir()).expect("kota bharu cache should build");

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

#[test]
fn kota_bharu_routes_by_stop_only_contains_routes_with_valid_shapes() {
    let cache = GtfsCache::build(&kota_bharu_data_dir()).expect("kota bharu cache should build");

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
    assert!(
        REDIS_MYBAS_KOTA_BHARU_LATEST_KEY.contains("mybas-kota-bharu"),
        "latest key must be scoped to mybas-kota-bharu"
    );
    assert!(
        REDIS_MYBAS_KOTA_BHARU_LAST_SEEN_KEY.contains("mybas-kota-bharu"),
        "last-seen key must be scoped to mybas-kota-bharu"
    );
    assert!(
        REDIS_MYBAS_KOTA_BHARU_MOTION_KEY.contains("mybas-kota-bharu"),
        "motion key must be scoped to mybas-kota-bharu"
    );
    assert!(
        REDIS_MYBAS_KOTA_BHARU_INGEST_LAST_KEY.contains("mybas-kota-bharu"),
        "ingest-last key must be scoped to mybas-kota-bharu"
    );

    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_LATEST_KEY,
        crate::REDIS_BUSES_LATEST_KEY,
        "kota-bharu latest key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_LAST_SEEN_KEY,
        crate::REDIS_BUSES_LAST_SEEN_KEY,
        "kota-bharu last-seen key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_MOTION_KEY,
        crate::REDIS_BUSES_MOTION_KEY,
        "kota-bharu motion key must not clash with rapidkl"
    );

    use crate::busmy_alor_setar::{
        REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY, REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY,
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY, REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
    };
    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_LATEST_KEY,
        REDIS_MYBAS_ALOR_SETAR_LATEST_KEY,
        "kota-bharu latest key must not clash with alor-setar"
    );
    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_LAST_SEEN_KEY,
        REDIS_MYBAS_ALOR_SETAR_LAST_SEEN_KEY,
        "kota-bharu last-seen key must not clash with alor-setar"
    );
    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_MOTION_KEY,
        REDIS_MYBAS_ALOR_SETAR_MOTION_KEY,
        "kota-bharu motion key must not clash with alor-setar"
    );
    assert_ne!(
        REDIS_MYBAS_KOTA_BHARU_INGEST_LAST_KEY,
        REDIS_MYBAS_ALOR_SETAR_INGEST_LAST_KEY,
        "kota-bharu ingest key must not clash with alor-setar"
    );
}

// ── Integration: live fetch returns BusPosition ──────────────────────────────

#[tokio::test]
async fn fetch_kota_bharu_bus_positions_returns_bus_position_data() {
    let positions = fetch_kota_bharu_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-kota-bharu feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no (mapped from vehicle_id)"
        );
        assert_eq!(
            pos.provider, "MYBAS-KOTA-BHARU",
            "provider must be MYBAS-KOTA-BHARU"
        );
        assert!(
            pos.latitude >= 5.0 && pos.latitude <= 7.0,
            "latitude {} outside expected Kelantan range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 101.5 && pos.longitude <= 103.0,
            "longitude {} outside expected Kelantan range",
            pos.longitude
        );
        assert!(
            pos.speed >= 0.0,
            "speed must be non-negative, got {}",
            pos.speed
        );
    }

    println!(
        "[busmy_kota_bharu integration] {} positions fetched, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
