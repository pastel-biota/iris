use crate::{api::error::ApiError, model::Identifier};

pub fn parse_identifier(id: &str) -> Result<Identifier, ApiError> {
    id.parse::<Identifier>()
        .map_err(ApiError::with_ctx(ApiError::BadRequest, "Invalid ID"))
}
