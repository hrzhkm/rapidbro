use crate::busmy_ipoh::{
    fetch_ipoh_bus_positions, REDIS_MYBAS_IPOH_INGEST_LAST_KEY, REDIS_MYBAS_IPOH_LAST_SEEN_KEY,
    REDIS_MYBAS_IPOH_LATEST_KEY, REDIS_MYBAS_IPOH_MOTION_KEY,
};
use crate::rapidkl::GtfsCache;
use crate::get_route_stops_from_cache;
use std::path::Path;

fn ipoh_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/busmy-ipoh")
}

// ── GTFS cache ────────────────────────────────────────────────────────────────

#[test]
fn ipoh_gtfs_cache_builds_successfully() {
    let cache = GtfsCache::build(&ipoh_data_dir()).expect("ipoh cache should build");

    assert!(
        !cache.route_stops_by_route.is_empty(),
        "ipoh cache should have at least one route"
    );
    assert!(
        !cache.context.stops_map.is_empty(),
        "ipoh cache should have at least one stop"
    );
    assert!(
        !cache.routes_by_stop.is_empty(),
        "ipoh cache should have route-by-stop index"
    );
}

#[test]
fn ipoh_stops_are_in_perak_region() {
    let cache = GtfsCache::build(&ipoh_data_dir()).expect("ipoh cache should build");

    for stop in cache.context.stops_map.values() {
        assert!(
            stop.stop_lat >= 3.8 && stop.stop_lat <= 5.0,
            "stop {} lat {} outside expected Perak range",
            stop.stop_id,
            stop.stop_lat
        );
        // Skip stops with clearly invalid zero longitude (bad source data)
        if stop.stop_lon < 50.0 {
            continue;
        }
        assert!(
            stop.stop_lon >= 100.5 && stop.stop_lon <= 102.0,
            "stop {} lon {} outside expected Perak range",
            stop.stop_id,
            stop.stop_lon
        );
    }
}

#[test]
fn ipoh_route_stops_are_sequence_sorted() {
    let cache = GtfsCache::build(&ipoh_data_dir()).expect("ipoh cache should build");

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
fn ipoh_routes_by_stop_only_contains_routes_with_valid_shapes() {
    let cache = GtfsCache::build(&ipoh_data_dir()).expect("ipoh cache should build");

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
        REDIS_MYBAS_IPOH_LATEST_KEY.contains("mybas-ipoh"),
        "latest key must be scoped to mybas-ipoh"
    );
    assert!(
        REDIS_MYBAS_IPOH_LAST_SEEN_KEY.contains("mybas-ipoh"),
        "last-seen key must be scoped to mybas-ipoh"
    );
    assert!(
        REDIS_MYBAS_IPOH_MOTION_KEY.contains("mybas-ipoh"),
        "motion key must be scoped to mybas-ipoh"
    );
    assert!(
        REDIS_MYBAS_IPOH_INGEST_LAST_KEY.contains("mybas-ipoh"),
        "ingest-last key must be scoped to mybas-ipoh"
    );

    assert_ne!(
        REDIS_MYBAS_IPOH_LATEST_KEY,
        crate::REDIS_BUSES_LATEST_KEY,
        "ipoh latest key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_IPOH_LAST_SEEN_KEY,
        crate::REDIS_BUSES_LAST_SEEN_KEY,
        "ipoh last-seen key must not clash with rapidkl"
    );
    assert_ne!(
        REDIS_MYBAS_IPOH_MOTION_KEY,
        crate::REDIS_BUSES_MOTION_KEY,
        "ipoh motion key must not clash with rapidkl"
    );

    use crate::busmy_kota_bharu::{
        REDIS_MYBAS_KOTA_BHARU_INGEST_LAST_KEY, REDIS_MYBAS_KOTA_BHARU_LAST_SEEN_KEY,
        REDIS_MYBAS_KOTA_BHARU_LATEST_KEY, REDIS_MYBAS_KOTA_BHARU_MOTION_KEY,
    };
    assert_ne!(
        REDIS_MYBAS_IPOH_LATEST_KEY,
        REDIS_MYBAS_KOTA_BHARU_LATEST_KEY,
        "ipoh latest key must not clash with kota-bharu"
    );
    assert_ne!(
        REDIS_MYBAS_IPOH_LAST_SEEN_KEY,
        REDIS_MYBAS_KOTA_BHARU_LAST_SEEN_KEY,
        "ipoh last-seen key must not clash with kota-bharu"
    );
    assert_ne!(
        REDIS_MYBAS_IPOH_MOTION_KEY,
        REDIS_MYBAS_KOTA_BHARU_MOTION_KEY,
        "ipoh motion key must not clash with kota-bharu"
    );
    assert_ne!(
        REDIS_MYBAS_IPOH_INGEST_LAST_KEY,
        REDIS_MYBAS_KOTA_BHARU_INGEST_LAST_KEY,
        "ipoh ingest key must not clash with kota-bharu"
    );
}

// ── Integration: live fetch returns BusPosition ──────────────────────────────

#[tokio::test]
async fn fetch_ipoh_bus_positions_returns_bus_position_data() {
    let positions = fetch_ipoh_bus_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-ipoh feed"
    );

    for pos in &positions {
        assert!(
            !pos.bus_no.is_empty(),
            "every position must have a non-empty bus_no (mapped from vehicle_id)"
        );
        assert_eq!(
            pos.provider, "MYBAS-IPOH",
            "provider must be MYBAS-IPOH"
        );
        assert!(
            pos.latitude >= 3.8 && pos.latitude <= 5.0,
            "latitude {} outside expected Perak range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 100.5 && pos.longitude <= 102.0,
            "longitude {} outside expected Perak range",
            pos.longitude
        );
        assert!(
            pos.speed >= 0.0,
            "speed must be non-negative, got {}",
            pos.speed
        );
    }

    println!(
        "[busmy_ipoh integration] {} positions fetched, first bus_no: {}",
        positions.len(),
        positions.first().map(|p| p.bus_no.as_str()).unwrap_or("?")
    );
}
