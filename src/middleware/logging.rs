use axum::body::Body;
use axum::http::Request;
use tower_http::trace::TraceLayer;
use tracing::Span;

pub fn layer() -> tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request<Body>| {
            let method = request.method();
            let uri = request.uri();
            let version = request.version();

            tracing::info_span!(
                "http_request",
                method = %method,
                uri = %uri,
                version = ?version,
            )
        })
        .on_request(|request: &Request<Body>, _span: &Span| {
            tracing::debug!(
                method = %request.method(),
                uri = %request.uri(),
                "Processing request"
            );
        })
        .on_response(
            |response: &axum::http::Response<Body>, latency: std::time::Duration, _span: &Span| {
                tracing::info!(
                    status = %response.status().as_u16(),
                    latency_ms = latency.as_millis() as u64,
                    "Request completed"
                );
            },
        )
        .on_failure(
            |error: tower_http::classify::ServerErrorsFailureClass, latency: std::time::Duration, _span: &Span| {
                tracing::error!(
                    error = %error,
                    latency_ms = latency.as_millis() as u64,
                    "Request failed"
                );
            },
        )
}
