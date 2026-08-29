use std::{collections::HashMap, sync::Arc};

use axum::{Json, extract::State, response::IntoResponse};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::error::ApiError,
    infra::api::types::{SuccessfulResponse, success},
    model::{Tag, TagGroup, TagName},
    tag::endpoint::{self, TagDescriptionScheme},
};

pub fn tag_route(_ctx: Arc<crate::Context>) -> OpenApiRouter<Arc<crate::Context>> {
    OpenApiRouter::new().routes(routes!(get_tags))
}

/// Retrieve registered tags and tag groups.
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = OK, description = "The user has been successfully logged in and the session is issued", body = SuccessfulResponse<endpoint::TagRegistryResponse>),
    )
)]
async fn get_tags(State(ctx): State<Arc<crate::Context>>) -> Result<impl IntoResponse, ApiError> {
    let tag_registry = ctx.tag.repo.lock().await;

    let tags: Vec<Tag> = tag_registry
        .get_all_known_tags()
        .await
        .map_err(ApiError::internal_during("Retrieveing tags"))?;
    let tag_groups: HashMap<TagName, TagGroup> = tag_registry
        .get_all_known_tag_groups()
        .await
        .map_err(ApiError::internal_during("Retrieveing tag groups"))?
        .into_iter()
        .map(|group| (group.name.clone(), group))
        .collect();

    let response = endpoint::TagRegistryResponse {
        tags: tags
            .into_iter()
            .map(|tag| TagDescriptionScheme::new(tag_groups.get(&tag.ident.0), tag))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ApiError::internal_during("Retrieveing tag groups"))?,
        tag_groups: tag_groups.into_values().map(Into::into).collect(),
    };

    Ok((StatusCode::OK, Json(success(response))).into_response())
}
