mod appointments;
mod availability;
mod dto;
mod events;
mod extract;
mod health;

use axum::Router;
use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::Request;
use tower::ServiceBuilder;
use tower_http::request_id::{
    MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Level, Span};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(info(title = "Unified Service Scheduler", version = "0.1.0"))]
struct ApiDoc;

fn request_span(req: &Request<Body>) -> Span {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str);
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or_default();
    tracing::info_span!("request", method = %req.method(), route, request_id)
}

pub fn router(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::healthz))
        .routes(routes!(appointments::create))
        .routes(routes!(appointments::get))
        .routes(routes!(appointments::cancel))
        .routes(routes!(availability::list))
        .split_for_parts();

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(()),
        )
        .layer(PropagateRequestIdLayer::x_request_id());

    router
        .merge(SwaggerUi::new("/docs").url("/openapi.json", api))
        .layer(middleware)
        .with_state(state)
}
