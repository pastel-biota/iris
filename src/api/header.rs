use http::{HeaderMap, HeaderValue, header};

use crate::api::error::ApiError;

/// Headers for serving an immutable, content-addressed binary asset.
///
/// Sets the content-type from a MIME string, a long immutable cache, and an
/// optional content-length when the payload size is known up front.
pub fn immutable_asset(
    mime: &str,
    content_length: Option<u64>,
) -> Result<HeaderMap, ApiError> {
    let content_type: HeaderValue = mime
        .try_into()
        .map_err(ApiError::internal_during("building the content-type header"))?;

    let mut headers = HeaderMap::from_iter([
        (header::CONTENT_TYPE, content_type),
        (
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=2592000, immutable"),
        ),
    ]);

    if let Some(len) = content_length {
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    }

    Ok(headers)
}
