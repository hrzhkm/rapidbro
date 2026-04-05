use crate::busmy_kangar::{
    fetch_mybas_kangar_positions, MybasKangarPosition, REDIS_MYBAS_KANGAR_INGEST_LAST_KEY,
    REDIS_MYBAS_KANGAR_LAST_SEEN_KEY, REDIS_MYBAS_KANGAR_LATEST_KEY,
};

// ── Serialization ─────────────────────────────────────────────────────────────

#[test]
fn position_serializes_and_deserializes() {
    let original = MybasKangarPosition {
        vehicle_id: "BUS-001".to_string(),
        label: Some("K01".to_string()),
        latitude: 6.1248,
        longitude: 100.3673,
        speed: 40.5,
        bearing: 270.0,
        route_id: Some("KANGAR-01".to_string()),
        trip_id: Some("TRIP-999".to_string()),
        timestamp: Some(1_700_000_000),
    };

    let json = serde_json::to_string(&original).expect("should serialize");
    let restored: MybasKangarPosition =
        serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(restored.vehicle_id, original.vehicle_id);
    assert_eq!(restored.label, original.label);
    assert!((restored.latitude - original.latitude).abs() < 1e-6);
    assert!((restored.longitude - original.longitude).abs() < 1e-6);
    assert!((restored.speed - original.speed).abs() < 1e-6);
    assert!((restored.bearing - original.bearing).abs() < 1e-6);
    assert_eq!(restored.route_id, original.route_id);
    assert_eq!(restored.trip_id, original.trip_id);
    assert_eq!(restored.timestamp, original.timestamp);
}

#[test]
fn position_with_all_optional_fields_none_serializes() {
    let pos = MybasKangarPosition {
        vehicle_id: "BUS-002".to_string(),
        label: None,
        latitude: 6.1248,
        longitude: 100.3673,
        speed: 0.0,
        bearing: 0.0,
        route_id: None,
        trip_id: None,
        timestamp: None,
    };

    let json = serde_json::to_string(&pos).expect("should serialize with None fields");
    let restored: MybasKangarPosition =
        serde_json::from_str(&json).expect("should deserialize with None fields");

    assert_eq!(restored.vehicle_id, "BUS-002");
    assert!(restored.label.is_none());
    assert!(restored.route_id.is_none());
    assert!(restored.trip_id.is_none());
    assert!(restored.timestamp.is_none());
}

// ── Redis key constants ───────────────────────────────────────────────────────

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
}

// ── Integration: live fetch ───────────────────────────────────────────────────

#[tokio::test]
async fn fetch_mybas_kangar_positions_returns_vehicle_data() {
    let positions = fetch_mybas_kangar_positions()
        .await
        .expect("fetch should succeed against live API");

    assert!(
        !positions.is_empty(),
        "expected at least one vehicle position from mybas-kangar feed"
    );

    for pos in &positions {
        assert!(
            !pos.vehicle_id.is_empty(),
            "every position must have a non-empty vehicle_id"
        );
        assert!(
            pos.latitude >= 4.0 && pos.latitude <= 7.0,
            "latitude {} is outside expected Perlis/north Malaysia range",
            pos.latitude
        );
        assert!(
            pos.longitude >= 99.0 && pos.longitude <= 102.0,
            "longitude {} is outside expected Perlis/north Malaysia range",
            pos.longitude
        );
    }

    println!(
        "[busmy_kangar integration] {} positions fetched, first: {:?}",
        positions.len(),
        positions.first()
    );
}
