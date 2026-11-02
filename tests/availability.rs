mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, TimeDelta, Utc};
use common::*;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

const STEPS_PER_DAY: usize = 48;

fn availability(dealership: Uuid, service: Uuid, date: &str) -> Request<Body> {
    get(&format!(
        "/dealerships/{dealership}/availability?service_type_id={service}&date={date}"
    ))
}

fn starts(body: &Value) -> Vec<DateTime<Utc>> {
    body.as_array()
        .unwrap()
        .iter()
        .map(|slot| slot["start"].as_str().unwrap().parse().unwrap())
        .collect()
}

#[sqlx::test(migrations = false)]
async fn empty_dealership_offers_full_day(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let (status, body) = call(&app, availability(D2, OIL, &tomorrow_date())).await;
    assert_eq!(status, StatusCode::OK);
    let open = starts(&body);
    assert_eq!(open.len(), STEPS_PER_DAY);
    assert_eq!(open[0], tomorrow_at(0));
    assert_eq!(open[1] - open[0], TimeDelta::minutes(30));
    assert_eq!(
        body[0]["end"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        tomorrow_at(0) + TimeDelta::minutes(30)
    );
}

#[sqlx::test(migrations = false)]
async fn seed_booking_blocks_overlapping_starts(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let (status, body) = call(&app, availability(D1, BRAKES, &tomorrow_date())).await;
    assert_eq!(status, StatusCode::OK);
    let open = starts(&body);
    assert_eq!(open.len(), STEPS_PER_DAY - 2);
    assert!(open.contains(&(tomorrow_at(9) - TimeDelta::hours(1))));
    assert!(!open.contains(&(tomorrow_at(9) - TimeDelta::minutes(30))));
    assert!(!open.contains(&tomorrow_at(9)));
    assert!(open.contains(&(tomorrow_at(9) + TimeDelta::minutes(30))));
}

#[sqlx::test(migrations = false)]
async fn booking_removes_slot_and_cancel_restores_it(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let date = tomorrow_date();

    let body = booking(CUSTOMER_2, VEHICLE_3, D2, OIL, tomorrow_at(9));
    let (status, created) = call(&app, post("/appointments", "k1", &body)).await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, listed) = call(&app, availability(D2, OIL, &date)).await;
    let open = starts(&listed);
    assert_eq!(open.len(), STEPS_PER_DAY - 1);
    assert!(!open.contains(&tomorrow_at(9)));

    let cancel = format!("/appointments/{}/cancel", created["id"].as_str().unwrap());
    let (status, _) = call(&app, post_empty(&cancel)).await;
    assert_eq!(status, StatusCode::OK);

    let (_, listed) = call(&app, availability(D2, OIL, &date)).await;
    assert_eq!(starts(&listed).len(), STEPS_PER_DAY);
}

#[sqlx::test(migrations = false)]
async fn no_qualified_technician_gives_empty_list(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let (status, body) = call(&app, availability(D2, BRAKES, &tomorrow_date())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
}

#[sqlx::test(migrations = false)]
async fn past_day_gives_empty_list(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let yesterday = (tomorrow_at(0) - TimeDelta::days(2))
        .date_naive()
        .to_string();
    let (status, body) = call(&app, availability(D1, OIL, &yesterday)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
}

#[sqlx::test(migrations = false)]
async fn invalid_requests(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let date = tomorrow_date();

    let (status, body) = call(&app, availability(unknown_id(), OIL, &date)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");

    let (status, body) = call(&app, availability(D1, unknown_id(), &date)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");

    let (status, body) = call(&app, availability(D1, OIL, "2026-13-45")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_query");

    let (status, body) = call(&app, get(&format!("/dealerships/{D1}/availability"))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_query");

    let (status, body) = call(&app, get("/dealerships/not-a-uuid/availability")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_id");
}

#[sqlx::test(migrations = false)]
async fn late_booking_blocks_next_day_starts(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let late = tomorrow_at(23) + TimeDelta::minutes(30);

    let body = booking(CUSTOMER_2, VEHICLE_3, D1, INSPECTION, late);
    let (status, _) = call(&app, post("/appointments", "k1", &body)).await;
    assert_eq!(status, StatusCode::CREATED);

    let next_day = tomorrow_at(0) + TimeDelta::days(1);
    let date = next_day.date_naive().to_string();
    let (status, listed) = call(&app, availability(D1, INSPECTION, &date)).await;
    assert_eq!(status, StatusCode::OK);
    let open = starts(&listed);
    assert_eq!(open.len(), STEPS_PER_DAY - 3);
    assert_eq!(open[0], next_day + TimeDelta::minutes(90));
}
