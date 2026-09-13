use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use reqwest::Client;

use crate::error::ProxyError;

pub async fn forward_request(
    client: &Client,
    url: &str,
    mut req: Request<Body>,
    timeout: Duration,
) -> Result<Response<Body>, ProxyError> {
    let method = req.method().clone();
    let headers = req.headers().clone();
    let uri = req.uri().clone();

    let body_bytes = match req.body_mut().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => return Err(ProxyError::Upstream(e.to_string())),
    };

    let is_upgrade = headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    if is_upgrade {
        return forward_websocket(client, url, &method, &headers, body_bytes, timeout).await;
    }

    let mut outgoing = client.request(method.clone(), url);

    for (name, value) in headers.iter() {
        if name == header::HOST || name == header::TRANSFER_ENCODING || name == header::CONNECTION {
            continue;
        }
        outgoing = outgoing.header(name.clone(), value.clone());
    }

    if !body_bytes.is_empty() {
        outgoing = outgoing.body(body_bytes);
    }

    let resp = outgoing.timeout(timeout).send().await.map_err(|e| {
        if e.is_timeout() {
            ProxyError::Timeout(timeout.as_secs())
        } else if e.is_connect() {
            ProxyError::Connection(e.to_string())
        } else {
            ProxyError::Upstream(e.to_string())
        }
    })?;

    let status = StatusCode::from_u16(resp.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = resp.headers().clone();

    let body = resp
        .bytes()
        .await
        .map_err(|e| ProxyError::Upstream(e.to_string()))?;

    let mut response = Response::builder().status(status);

    for (name, value) in resp_headers.iter() {
        if name == header::TRANSFER_ENCODING || name == header::CONNECTION {
            continue;
        }
        response = response.header(name.clone(), value.clone());
    }

    response
        .body(Body::from(body))
        .map_err(|e| ProxyError::Upstream(e.to_string()))
}

async fn forward_websocket(
    client: &Client,
    url: &str,
    method: &axum::http::Method,
    headers: &axum::http::HeaderMap,
    body: Bytes,
    timeout: Duration,
) -> Result<Response<Body>, ProxyError> {
    let mut req_builder = client.request(method.clone(), url);

    for (name, value) in headers.iter() {
        if name == header::HOST || name == header::TRANSFER_ENCODING {
            continue;
        }
        req_builder = req_builder.header(name.clone(), value.clone());
    }

    if !body.is_empty() {
        req_builder = req_builder.body(body);
    }

    let resp = req_builder
        .timeout(timeout)
        .send()
        .await
        .map_err(ProxyError::from)?;

    let status = StatusCode::from_u16(resp.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = resp.headers().clone();

    let body = resp
        .bytes()
        .await
        .map_err(|e| ProxyError::Upstream(e.to_string()))?;

    let mut response = Response::builder().status(status);

    for (name, value) in resp_headers.iter() {
        response = response.header(name.clone(), value.clone());
    }

    response
        .body(Body::from(body))
        .map_err(|e| ProxyError::Upstream(e.to_string()))
}
