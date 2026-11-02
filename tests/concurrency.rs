mod common;

use axum::Router;
use axum::http::StatusCode;
use common::*;
use serde_json::Value;
use sqlx::PgPool;
use tokio::task::JoinSet;
use uuid::Uuid;

const CONTENDERS: usize = 10;
const D1_OIL_CAPACITY: usize = 2;

async fn fire_all(
    requests: Vec<(Router, axum::http::Request<axum::body::Body>)>,
) -> Vec<(StatusCode, Value)> {
    let mut set = JoinSet::new();
    for (router, request) in requests {
        set.spawn(async move { call(&router, request).await });
    }
    set.join_all().await
}

fn count(results: &[(StatusCode, Value)], status: StatusCode) -> usize {
    results.iter().filter(|(s, _)| *s == status).count()
}

async fn confirmed_at(
    pool: &PgPool,
    dealership: Uuid,
    start: chrono::DateTime<chrono::Utc>,
) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM appointment \
         WHERE dealership_id = $1 AND status = 'confirmed' AND lower(slot) = $2",
    )
    .bind(dealership)
    .bind(start)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn contend_for_slot(
    pairs: &[(Uuid, Uuid)],
    start: chrono::DateTime<chrono::Utc>,
    router_for: impl Fn() -> Router,
) -> Vec<(Router, axum::http::Request<axum::body::Body>)> {
    pairs
        .iter()
        .enumerate()
        .map(|(i, &(customer, vehicle))| {
            let body = booking(customer, vehicle, D1, OIL, start);
            (router_for(), post("/appointments", &format!("k{i}"), &body))
        })
        .collect()
}

#[sqlx::test(migrations = false)]
async fn mutex_and_constraint_grant_exactly_capacity(pool: PgPool) {
    load_seed(&pool).await;
    let start = tomorrow_at(10);
    let pairs = add_customers_with_vehicles(&pool, CONTENDERS).await;
    let app = app(pool.clone());

    let results = fire_all(contend_for_slot(&pairs, start, || app.clone())).await;

    assert_eq!(count(&results, StatusCode::CREATED), D1_OIL_CAPACITY);
    assert_eq!(
        count(&results, StatusCode::CONFLICT),
        CONTENDERS - D1_OIL_CAPACITY
    );
    assert_eq!(confirmed_at(&pool, D1, start).await, D1_OIL_CAPACITY as i64);
}

#[sqlx::test(migrations = false)]
async fn constraint_alone_never_overbooks(pool: PgPool) {
    load_seed(&pool).await;
    let start = tomorrow_at(10);
    let pairs = add_customers_with_vehicles(&pool, CONTENDERS).await;

    let requests = contend_for_slot(&pairs, start, || app_with_locks(pool.clone(), new_locks()));
    let results = fire_all(requests).await;

    let created = count(&results, StatusCode::CREATED);
    assert!((1..=D1_OIL_CAPACITY).contains(&created));
    assert_eq!(created + count(&results, StatusCode::CONFLICT), CONTENDERS);
    assert_eq!(confirmed_at(&pool, D1, start).await, created as i64);
}

#[sqlx::test(migrations = false)]
async fn same_idempotency_key_creates_once(pool: PgPool) {
    load_seed(&pool).await;
    let start = tomorrow_at(10);
    let body = booking(CUSTOMER_2, VEHICLE_3, D1, OIL, start);

    let requests = (0..CONTENDERS)
        .map(|_| {
            (
                app_with_locks(pool.clone(), new_locks()),
                post("/appointments", "same", &body),
            )
        })
        .collect();
    let results = fire_all(requests).await;

    assert_eq!(count(&results, StatusCode::CREATED), 1);
    assert_eq!(count(&results, StatusCode::OK), CONTENDERS - 1);
    let ids: Vec<&Value> = results.iter().map(|(_, body)| &body["id"]).collect();
    assert!(ids.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(confirmed_at(&pool, D1, start).await, 1);
}
