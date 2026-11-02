mod common;

use std::time::Duration;

use axum::http::StatusCode;
use chrono::TimeDelta;
use common::*;
use serde_json::json;
use sqlx::PgPool;

const WAIT: Duration = Duration::from_secs(2);
const QUIET: Duration = Duration::from_millis(300);

#[sqlx::test(migrations = false)]
async fn booked_event_after_create(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let mut sse = SseStream::open(&app, &events_path(D1)).await;

    let body = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, tomorrow_at(9));
    let (_, created) = call(&app, post("/appointments", "k1", &body)).await;

    let (name, data) = sse.next(WAIT).await.expect("no event");
    assert_eq!(name, "booked");
    assert_eq!(data["appointment_id"], created["id"]);
    assert_eq!(data["start"], json!(tomorrow_at(9)));
    assert_eq!(data["end"], json!(tomorrow_at(9) + TimeDelta::minutes(30)));
}

#[sqlx::test(migrations = false)]
async fn cancelled_event_after_cancel(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let mut sse = SseStream::open(&app, &events_path(D1)).await;

    let cancel = format!("/appointments/{SEED_APPOINTMENT}/cancel");
    call(&app, post_empty(&cancel)).await;

    let (name, data) = sse.next(WAIT).await.expect("no event");
    assert_eq!(name, "cancelled");
    assert_eq!(data["appointment_id"], json!(SEED_APPOINTMENT));
}

#[sqlx::test(migrations = false)]
async fn no_event_for_other_dealership_replay_or_repeat_cancel(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);
    let mut sse = SseStream::open(&app, &events_path(D1)).await;

    let other = booking(CUSTOMER_2, VEHICLE_3, D2, OIL, tomorrow_at(9));
    call(&app, post("/appointments", "k1", &other)).await;
    assert!(sse.next(QUIET).await.is_none());

    let body = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, tomorrow_at(10));
    call(&app, post("/appointments", "k2", &body)).await;
    assert_eq!(sse.next(WAIT).await.expect("no event").0, "booked");

    let (status, _) = call(&app, post("/appointments", "k2", &body)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(sse.next(QUIET).await.is_none());

    let cancel = format!("/appointments/{SEED_APPOINTMENT}/cancel");
    call(&app, post_empty(&cancel)).await;
    assert_eq!(sse.next(WAIT).await.expect("no event").0, "cancelled");

    call(&app, post_empty(&cancel)).await;
    assert!(sse.next(QUIET).await.is_none());
}

#[sqlx::test(migrations = false)]
async fn invalid_dealership_is_rejected_before_stream(pool: PgPool) {
    load_seed(&pool).await;
    let app = app(pool);

    let (status, body) = call(&app, get(&events_path(unknown_id()))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");

    let (status, body) = call(&app, get("/dealerships/not-a-uuid/events")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_id");
}

#[sqlx::test(migrations = false)]
async fn shutdown_closes_stream(pool: PgPool) {
    load_seed(&pool).await;
    let (app, shutdown) = app_with_shutdown(pool);
    let mut sse = SseStream::open(&app, &events_path(D1)).await;

    assert!(!sse.closed(QUIET).await);
    shutdown.cancel();
    assert!(sse.closed(WAIT).await);
}

fn events_path(dealership: uuid::Uuid) -> String {
    format!("/dealerships/{dealership}/events")
}
