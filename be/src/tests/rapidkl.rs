use crate::rapidkl::GtfsCache;
use crate::{get_route_stops_from_cache, get_routes_for_stop_from_cache};
use std::path::Path;

fn rapidkl_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bus_data/rapid-kl")
}

#[test]
fn gtfs_cache_builds_with_expected_indexes() {
    let cache = GtfsCache::build(&rapidkl_data_dir()).expect("cache should build from GTFS files");

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
    let cache = GtfsCache::build(&rapidkl_data_dir()).expect("cache should build from GTFS files");
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
    let cache = GtfsCache::build(&rapidkl_data_dir()).expect("cache should build from GTFS files");
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
