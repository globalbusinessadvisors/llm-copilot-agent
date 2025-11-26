//! Axum router configuration

use crate::{
    rest::{handlers, middleware},
    AppState,
};
use axum::{
    http::{header, HeaderValue, Method},
    middleware as axum_middleware,
    routing::{get, post, delete},
    Router,
};
use std::{sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};

/// Create the main API router
pub fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);

    // Create the API v1 router
    let api_v1 = Router::new()
        // Session routes
        .route("/sessions", post(handlers::create_session))
        .route("/sessions/:id", get(handlers::get_session))
        .route("/sessions/:id", delete(handlers::delete_session))
        // Message routes
        .route("/messages", post(handlers::send_message))
        .route("/messages/:session_id", get(handlers::get_messages))
        // Workflow routes
        .route("/workflows", post(handlers::create_workflow))
        .route("/workflows/:id", get(handlers::get_workflow_status))
        .layer(
            ServiceBuilder::new()
                .layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
                .layer(axum_middleware::from_fn(middleware::rate_limit_middleware))
                .layer(axum_middleware::from_fn(middleware::request_id_middleware)),
        );

    // Health check routes (no authentication required)
    let health_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check));

    // Combine all routes
    Router::new()
        .nest("/api/v1", api_v1)
        .merge(health_routes)
        .layer(cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Configure CORS layer
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(
            std::env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .parse::<HeaderValue>()
                .unwrap_or(HeaderValue::from_static("*")),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn create_test_state() -> AppState {
        // This would need actual implementations for testing
        // For now, this is a placeholder
        todo!("Create test state with mock dependencies")
    }

    #[tokio::test]
    async fn test_health_check_route() {
        // let app = create_router(create_test_state());
        // let response = app
        //     .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        //     .await
        //     .unwrap();
        // assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_configuration() {
        // Test CORS headers are properly set
        // This would require setting up a test server
    }
}
