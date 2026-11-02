#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::{DateTime, Days, Utc};
use http_body_util::BodyExt;
use keyloop_challenge::api;
use keyloop_challenge::domain::SlotGrid;
use keyloop_challenge::infra::{EventBus, Locks};
use keyloop_challenge::state::AppState;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

pub const D1: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000101);
pub const D2: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000102);
pub const OIL: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000201);
pub const BRAKES: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000202);
pub const INSPECTION: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000203);
pub const CUSTOMER_1: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000501);
pub const CUSTOMER_2: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000502);
pub const VEHICLE_1: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000601);
pub const VEHICLE_2: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000602);
pub const VEHICLE_3: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000603);
pub const SEED_APPOINTMENT: Uuid = Uuid::from_u128(0x00000000_0000_7000_8000_000000000701);

pub fn unknown_id() -> Uuid {
    Uuid::from_u128(1)
}

pub async fn load_seed(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../database/db.sql"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../database/seed.sql"))
        .execute(pool)
        .await
        .unwrap();
}

pub fn new_locks() -> Locks {
    Locks::new(Duration::from_secs(2))
}

pub fn app_with_locks(pool: PgPool, locks: Locks) -> Router {
    api::router(AppState {
        pool,
        locks,
        grid: SlotGrid::new(30),
        bus: EventBus::new(),
        shutdown: CancellationToken::new(),
    })
}

pub async fn add_customers_with_vehicles(pool: &PgPool, count: usize) -> Vec<(Uuid, Uuid)> {
    let mut pairs = Vec::with_capacity(count);
    for i in 0..count {
        let customer: Uuid =
            sqlx::query_scalar("INSERT INTO customer (name) VALUES ($1) RETURNING id")
                .bind(format!("Load {i}"))
                .fetch_one(pool)
                .await
                .unwrap();
        let vehicle: Uuid = sqlx::query_scalar(
            "INSERT INTO vehicle (customer_id, plate) VALUES ($1, $2) RETURNING id",
        )
        .bind(customer)
        .bind(format!("LOAD-{i:04}"))
        .fetch_one(pool)
        .await
        .unwrap();
        pairs.push((customer, vehicle));
    }
    pairs
}

pub fn app(pool: PgPool) -> Router {
    app_with_shutdown(pool).0
}

pub fn app_with_shutdown(pool: PgPool) -> (Router, CancellationToken) {
    let shutdown = CancellationToken::new();
    let router = api::router(AppState {
        pool,
        locks: new_locks(),
        grid: SlotGrid::new(30),
        bus: EventBus::new(),
        shutdown: shutdown.clone(),
    });
    (router, shutdown)
}

pub fn tomorrow_at(hour: u32) -> DateTime<Utc> {
    (Utc::now().date_naive() + Days::new(1))
        .and_hms_opt(hour, 0, 0)
        .unwrap()
        .and_utc()
}

pub fn tomorrow_date() -> String {
    tomorrow_at(0).date_naive().to_string()
}

pub fn booking(
    customer: Uuid,
    vehicle: Uuid,
    dealership: Uuid,
    service: Uuid,
    start: DateTime<Utc>,
) -> Value {
    json!({
        "customer_id": customer,
        "vehicle_id": vehicle,
        "dealership_id": dealership,
        "service_type_id": service,
        "start": start,
    })
}

pub fn get(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}

pub fn post_empty(path: &str) -> Request<Body> {
    Request::post(path).body(Body::empty()).unwrap()
}

pub fn post(path: &str, key: &str, body: &Value) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header("idempotency-key", key)
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

pub struct SseStream {
    body: Body,
    buf: String,
}

impl SseStream {
    pub async fn open(app: &Router, path: &str) -> Self {
        let res = app.clone().oneshot(get(path)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        Self {
            body: res.into_body(),
            buf: String::new(),
        }
    }

    pub async fn next(&mut self, wait: Duration) -> Option<(String, Value)> {
        tokio::time::timeout(wait, async {
            loop {
                if let Some(end) = self.buf.find("\n\n") {
                    let frame: String = self.buf.drain(..end + 2).collect();
                    if let Some(parsed) = parse_frame(&frame) {
                        return parsed;
                    }
                    continue;
                }
                let frame = self.body.frame().await.expect("stream ended").unwrap();
                if let Ok(data) = frame.into_data() {
                    self.buf.push_str(std::str::from_utf8(&data).unwrap());
                }
            }
        })
        .await
        .ok()
    }

    pub async fn closed(&mut self, wait: Duration) -> bool {
        tokio::time::timeout(wait, async { while self.body.frame().await.is_some() {} })
            .await
            .is_ok()
    }
}

fn parse_frame(frame: &str) -> Option<(String, Value)> {
    let mut name = None;
    let mut data = None;
    for line in frame.lines() {
        if let Some(v) = line.strip_prefix("event:") {
            name = Some(v.trim().to_owned());
        } else if let Some(v) = line.strip_prefix("data:") {
            data = Some(serde_json::from_str(v.trim()).unwrap());
        }
    }
    Some((name?, data?))
}
