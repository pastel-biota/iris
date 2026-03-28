use axum::{extract::Request, middleware::Next, response::Response};
use tracing::Instrument;

pub async fn access_log(req: Request, next: Next) -> Response {
    let access_id = (0..6)
        .map(|_| ('a' as u8 + (rand::random::<u8>() % 26)) as char)
        .collect::<String>();

    let span = tracing::debug_span!("request", id = access_id);

    tracing::debug!("[{}] {} {}", access_id, req.method(), req.uri());

    let response = next.run(req)
        .instrument(span)
        .await;

    tracing::debug!("[{}] {}", access_id, response.status());

    response
}

