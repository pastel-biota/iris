use axum::{
    Json,
    extract::{Multipart, multipart::Field},
    response::{IntoResponse as _, Response},
};
use futures_util::TryStreamExt as _;

use crate::{
    api::error::ApiError,
    infra::api::types::client_error,
    ingest::technicals::stream::{SizedStream, StreamPayloadHandle},
};

pub struct FormFile<'s> {
    pub file_name: Option<String>,
    pub payload: StreamPayloadHandle<'s>,
}

pub async fn receive_form_file<'s>(
    multipart: &mut Multipart,
    receiver: &'s SizedStream,
    length: usize,
) -> Result<FormFile<'s>, Response> {
    let field = next_field(multipart).await?;

    let (file_name, field) = if matches!(field.name(), Some("file") | None) {
        (field.file_name().map(|name| name.to_string()), field)
    } else {
        if !field.name().is_some_and(|name| name == "name") {
            return Err(
                ApiError::BadRequest("The form field name is unknown".into()).into_response(),
            );
        }

        let name = field
            .text()
            .await
            .map_err(ApiError::with_ctx(
                ApiError::BadRequest,
                "name field must contain valid text for the file name",
            ))
            .map_err(|error| error.into_response())?;

        let field = next_field(multipart).await?;

        if !matches!(field.name(), Some("file") | None) {
            return Err(
                ApiError::BadRequest("The form is invalid, this is either of:\n -The file was not specified\n- The form field name cannot appear after the file".into()).into_response()
            );
        }

        (Some(name), field)
    };

    let payload = receiver
        .receive_stream(length, field.into_stream())
        .await
        .map_err(ApiError::internal_during("reading the body content"))
        .map_err(|error| error.into_response())?;

    Ok(FormFile { file_name, payload })
}

async fn next_field(multipart: &mut Multipart) -> Result<Field<'_>, Response> {
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            return Err(
                ApiError::BadRequest("The file is needed but not attached".to_string())
                    .into_response(),
            );
        }
        Err(e) => {
            return Err((
                e.status(),
                Json(client_error(format!(
                    "Failed to parse the multipart request: {}",
                    e.body_text()
                ))),
            )
                .into_response());
        }
    };

    Ok(field)
}
