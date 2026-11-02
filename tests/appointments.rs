mod common;

use axum::http::StatusCode;
use chrono::TimeDelta;
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = false)]
async fn create_then_replay(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let body = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, tomorrow_at(9));

    let (status, first) = call(&app, post("/appointments", "k1", &body)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(first["status"], "confirmed");
    assert_eq!(first["start"], json!(tomorrow_at(9)));
    assert_eq!(first["end"], json!(tomorrow_at(9) + TimeDelta::minutes(30)));

    let (status, replay) = call(&app, post("/appointments", "k1", &body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["id"], replay["id"]);
}

#[sqlx::test(migrations = false)]
async fn bays_exhausted_gives_409(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let second = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, tomorrow_at(9));
    let (status, _) = call(&app, post("/appointments", "k1", &second)).await;
    assert_eq!(status, StatusCode::CREATED);

    let third = booking(CUSTOMER_1, VEHICLE_2, D1, OIL, tomorrow_at(9));
    let (status, body) = call(&app, post("/appointments", "k2", &third)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "slot_unavailable");
}

#[sqlx::test(migrations = false)]
async fn busy_technician_with_free_bay_gives_409(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let brakes = booking(CUSTOMER_2, VEHICLE_3, D1, BRAKES, tomorrow_at(9));
    let (status, body) = call(&app, post("/appointments", "k1", &brakes)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "slot_unavailable");
}

#[sqlx::test(migrations = false)]
async fn vehicle_double_booked_gives_409(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let same_vehicle = booking(CUSTOMER_1, VEHICLE_1, D1, OIL, tomorrow_at(9));
    let (status, body) = call(&app, post("/appointments", "k1", &same_vehicle)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "vehicle_unavailable");
}

#[sqlx::test(migrations = false)]
async fn adjacent_slot_is_allowed(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let start = tomorrow_at(9) + TimeDelta::minutes(30);
    let next = booking(CUSTOMER_1, VEHICLE_1, D1, OIL, start);
    let (status, _) = call(&app, post("/appointments", "k1", &next)).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test(migrations = false)]
async fn invalid_requests_give_422(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let start = tomorrow_at(11);

    let cases = [
        (
            "unknown_service_type",
            "k1",
            booking(CUSTOMER_2, VEHICLE_3, D1, unknown_id(), start),
        ),
        (
            "unknown_reference",
            "k2",
            booking(unknown_id(), VEHICLE_3, D1, OIL, start),
        ),
        (
            "unknown_reference",
            "k3",
            booking(CUSTOMER_2, VEHICLE_3, unknown_id(), OIL, start),
        ),
        (
            "start_off_grid",
            "k4",
            booking(
                CUSTOMER_2,
                VEHICLE_3,
                D1,
                OIL,
                start + TimeDelta::minutes(15),
            ),
        ),
        (
            "start_in_past",
            "k5",
            booking(CUSTOMER_2, VEHICLE_3, D1, OIL, start - TimeDelta::days(2)),
        ),
        (
            "invalid_idempotency_key",
            " ",
            booking(CUSTOMER_2, VEHICLE_3, D1, OIL, start),
        ),
    ];
    for (code, key, body) in cases {
        let (status, response) = call(&app, post("/appointments", key, &body)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{code}");
        assert_eq!(response["code"], code);
    }

    let first = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, start);
    let (status, _) = call(&app, post("/appointments", "reused", &first)).await;
    assert_eq!(status, StatusCode::CREATED);

    let other = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, start + TimeDelta::hours(1));
    let (status, response) = call(&app, post("/appointments", "reused", &other)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response["code"], "idempotency_key_reused");
}

#[sqlx::test(migrations = false)]
async fn get_found_and_missing(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let path = format!("/appointments/{SEED_APPOINTMENT}");
    let (status, body) = call(&app, get(&path)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], json!(SEED_APPOINTMENT));
    assert_eq!(body["status"], "confirmed");
    assert_eq!(body["start"], json!(tomorrow_at(9)));

    let path = format!("/appointments/{}", unknown_id());
    let (status, body) = call(&app, get(&path)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");

    let (status, body) = call(&app, get("/appointments/not-a-uuid")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_id");
}

#[sqlx::test(migrations = false)]
async fn cancel_is_idempotent(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let cancel = format!("/appointments/{SEED_APPOINTMENT}/cancel");

    for _ in 0..2 {
        let (status, body) = call(&app, post_empty(&cancel)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "cancelled");
    }

    let (_, body) = call(&app, get(&format!("/appointments/{SEED_APPOINTMENT}"))).await;
    assert_eq!(body["status"], "cancelled");

    let missing = format!("/appointments/{}/cancel", unknown_id());
    let (status, body) = call(&app, post_empty(&missing)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}

#[sqlx::test(migrations = false)]
async fn cancel_frees_slot(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let second = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, tomorrow_at(9));
    let (status, _) = call(&app, post("/appointments", "k1", &second)).await;
    assert_eq!(status, StatusCode::CREATED);

    let third = booking(CUSTOMER_1, VEHICLE_2, D1, OIL, tomorrow_at(9));
    let (status, _) = call(&app, post("/appointments", "k2", &third)).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let cancel = format!("/appointments/{SEED_APPOINTMENT}/cancel");
    let (status, _) = call(&app, post_empty(&cancel)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(&app, post("/appointments", "k2", &third)).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test(migrations = false)]
async fn booking_across_midnight_blocks_next_day(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let late = tomorrow_at(23) + TimeDelta::minutes(30);

    let first = booking(CUSTOMER_2, VEHICLE_3, D1, INSPECTION, late);
    let (status, created) = call(&app, post("/appointments", "k1", &first)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["end"], json!(late + TimeDelta::hours(2)));

    let next_day = tomorrow_at(0) + TimeDelta::days(1);
    let overlapping = booking(CUSTOMER_1, VEHICLE_2, D1, INSPECTION, next_day);
    let (status, body) = call(&app, post("/appointments", "k2", &overlapping)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "slot_unavailable");

    let after = booking(
        CUSTOMER_1,
        VEHICLE_2,
        D1,
        INSPECTION,
        next_day + TimeDelta::minutes(90),
    );
    let (status, _) = call(&app, post("/appointments", "k3", &after)).await;
    assert_eq!(status, StatusCode::CREATED);
}
