use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use http::request::Parts;

use crate::{
    Context,
    api::error::ApiError,
    auth::extractor::ValidUserSession,
    infra::api::types::{ClientError, FileForm, SuccessfulResponse, client_error, success},
    ingest::{
        self,
        api::scheme::PhotoScheme,
        technicals::form::{FormFile, receive_form_file},
        usecase::append_file::{AppendFileUsecaseError, AppendFileUsecaseParam},
    },
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct AddRelatedPhotoResponse {
    changed: bool,
    photo: PhotoScheme,
}

/// Add related files to the photo
///
/// Upload a photo payload and register. This triggers the image processing.
/// Note that the response from this endpoint does not guarantee (and practically does not mean)
/// that the image has been generated. However representative_rgb is available, and the client can
/// use this value to show the placeholder for the uploaded photo, until the image is being
/// processed.
///
/// You need to be logged in to use this endpoint.
#[utoipa::path(
    put,
    path = "/file",
    security(
        ("session_header" = []),
        ("session_cookie" = [])
    ),
    request_body(content = FileForm, content_type = "multipart/form-data"),
    responses(
        (status = OK, description = "The photo was updated, and the related file was saved", body = SuccessfulResponse<AddRelatedPhotoResponse>),
        (status = NOT_MODIFIED, description = "The request was valid, and no change was introduced"),
        (status = CONFLICT, description = "There already was a related file registered", body = ClientError),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn append_file(
    State(ctx): State<Arc<Context>>,
    ValidUserSession(session): ValidUserSession,
    parts: Parts,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let Some(length) = parts
        .headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return Err(ApiError::BadRequest(
            "Content-Length needs to be provided".to_owned(),
        ));
    };

    let file = match receive_form_file(&mut multipart, &ctx.ingest.sized_sream, length).await {
        Ok(file) => file,
        Err(err_response) => {
            return Ok(err_response);
        }
    };
    let FormFile { payload, file_name } = file;

    if !multipart
        .next_field()
        .await
        .is_ok_and(|content| content.is_none())
    {
        return Err(ApiError::BadRequest(
            "There are multiple files, but this E/P accepts only one".to_string(),
        ));
    }

    let result = ingest::usecase::append_file::append_file(
        &session,
        ctx.clone(),
        AppendFileUsecaseParam {
            file: payload,
            file_name,
        },
    )
    .await;

    let meta = match result {
        Ok(meta) => meta,
        Err(e) => {
            return match e {
                AppendFileUsecaseError::NoFilename => Ok((
                    http::StatusCode::BAD_REQUEST,
                    Json(client_error("The request was valid, but the file's name was empty and currently Iris is not sure how to make a good use of this"))
                )
                    .into_response()),
                AppendFileUsecaseError::UnknownExtension => Ok((
                    http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    Json(client_error("The request was valid, but the file type couldn't be known or not something Iris knows"))
                )
                    .into_response()),
                AppendFileUsecaseError::NoAdequatePhoto => Ok((
                    http::StatusCode::NOT_FOUND,
                    Json(client_error("The request was valid, but the photo to edit wasn't found. Maybe you should try registering first then it works"))
                )
                    .into_response()),
                AppendFileUsecaseError::Conflict(reason) => Ok((
                    http::StatusCode::CONFLICT,
                    Json(client_error(format!("The current PhotoMeta conflicts with the guessed property\n{reason}")))
                )
                    .into_response()),
                AppendFileUsecaseError::InternalError(e) => Ok((
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(client_error(format!("The server could not receive the entire image to calculate hash\n{e}")))
                )
                    .into_response()),
            };
        }
    };

    Ok((
        StatusCode::OK,
        Json(success(AddRelatedPhotoResponse {
            changed: meta.changed,
            photo: meta.meta.into(),
        })),
    )
        .into_response())
}
