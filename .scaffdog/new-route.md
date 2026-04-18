---
name: 'new-route'
root: 'src'
output: '*/api'
questions:
  name: 'Please enter the handler name.'
---

# `{{ inputs.name | snake }}.rs`
```rust
use std::sync::Arc;

use axum::{Json, extract::{Path, State}, http::StatusCode, response::IntoResponse};

use crate::{Context, infra::api::types::{ClientError, SuccessfulResponse, client_error, success}};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct {{ inputs.name | pascal }}Param {
    /// A new field.
    #[schema(example = "request_text")]
    new_field: String,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct {{ inputs.name | pascal }}Response {
    /// A new field in response.
    #[schema(example = "response_text")]
    field: String,
}

/// A new field
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    post,
    // TODO: Replace with the correct path - use {xxx} to accept path parameter
    path = "/",
    params(
        ("path_parameter_name" = String, Path),
    ),
    request_body(content_type = "application/json", content = {{ inputs.name | pascal }}Param),
    responses(
        (status = OK, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<{{ inputs.name | pascal }}Response>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn {{ inputs.name | snake }}(
    State(ctx): State<Arc<Context>>,
    Path((parameter,)): Path<(String,)>,
    Json(param): Json<{{ inputs.name | pascal }}Param>,
) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(client_error("Not implemented"))).into_response()
}
```

