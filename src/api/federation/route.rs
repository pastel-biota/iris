mod ping;

use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::Context;

pub fn federation_route(ctx: Arc<Context>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(ping::ping))
        .with_state(ctx)
}

