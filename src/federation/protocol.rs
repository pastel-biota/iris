
use anyhow::{Context, bail};
use futures_util::TryStreamExt;
use http::{Method, header};
use serde_json::Value;

use crate::{infra::api::types::IrisResponse, repository::io::LengthedStream};

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    #[error("The remote Iris responded with {0}: {1}")]
    Applictaion(http::StatusCode, String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub trait Endpoint {
    const PATH: (http::Method, &'static str);
    type Request: serde::Serialize + for<'de> serde::Deserialize<'de> + utoipa::ToSchema;
    type Response: serde::Serialize + for<'de> serde::Deserialize<'de> + utoipa::ToSchema;
}

#[macro_export]
macro_rules! path {
    ($endpoint:ty) => {
        <$endpoint as $crate::federation::protocol::Endpoint>::PATH.1
    };
}

fn contrust_request<E: Endpoint>(
    session_id: Option<&str>,
    origin: &str,
    param: E::Request
) -> Result<reqwest_middleware::RequestBuilder, RequestError> {
    let (method, path) = E::PATH;

    let req = if method == Method::GET {
        let path_with_param = to_param(path, param)?;
        super::request::create_client()
            .request(method, format!("{}{}", origin, path_with_param))
    } else {
        super::request::create_client()
            .request(method, format!("{}{}", origin, path))
            .body(serde_json::to_string(&param).context("Could not serialize the body")?)
            .header(header::CONTENT_TYPE, "application/json")
    };

    let req = if let Some(session_id) = session_id {
        req.header(header::AUTHORIZATION, format!("Bearer {}", session_id))
    } else {
        req
    };

    Ok(req)
}

pub async fn request<E: Endpoint>(
    session_id: Option<&str>,
    origin: &str,
    param: E::Request
) -> Result<E::Response, RequestError> {
    let req = contrust_request::<E>(session_id, origin, param)?
        .send()
        .await
        .with_context(|| "Could not send the request")?;

    let code = req.status();

    let response = req.text().await.context("Could not read the response body")?;

    let response = serde_json::from_str::<IrisResponse<E::Response>>(&response)
        .with_context(|| {
            tracing::debug!("Failed to parse [{}{}] ({}): {:?}", origin, E::PATH.1, code, response);
            format!("The request to '{}' was successful, but its shape is not something we wanted", E::PATH.1)
        })?;

    match response {
        IrisResponse::Okay { response } => {
            Ok(response)
        },
        IrisResponse::Error { reason } => {
            Err(RequestError::Applictaion(code, reason))
        }
    }
}

pub async fn request_stream<E: Endpoint>(
    session_id: Option<String>,
    origin: String,
    param: E::Request
) -> Result<LengthedStream, RequestError> {
    let req = contrust_request::<E>(session_id.as_deref(), &origin, param)?
        .send()
        .await
        .context("Could not send the request")?;

    let code = req.status();

    if !code.is_success() {
        let response = req.json::<IrisResponse<E::Response>>().await;

        return if let Ok(IrisResponse::Error { reason }) = response {
            Err(RequestError::Applictaion(code, reason))
        } else {
            Err(RequestError::Anyhow(anyhow::anyhow!("HTTP status code was {code} and it is not application error")))
        };
    }

    Ok(LengthedStream {
        len: req.headers().get(http::header::CONTENT_LENGTH)
            .and_then(|text| text.to_str().ok())
            .and_then(|text| text.parse().ok()),
        stream: Box::pin(req.bytes_stream().map_err(|error| error.to_string()))
    })
}

fn to_param(url: &str, query: impl serde::Serialize) -> anyhow::Result<String> {
    let query = serde_json::to_value(query)?;

    if query == Value::Null {
        return Ok(url.to_owned());
    }

    let Value::Object(query) = query else {
        bail!("The query value needs to be an object");
    };

    let mut query_strs = Vec::new();
    let mut url = url.to_owned();

    for (key, value) in query.into_iter() {
        let value = match value {
            Value::String(value) => value,
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            Value::Array(_) => {
                unimplemented!("Array value was tried to use for the query parameter, but I have no good idea how to serialize this");
            },
            Value::Object(_) => {
                bail!("The query value needs to be flat");
            }
            Value::Null => {
                continue;
            }
        };

        let value = urlencoding::encode(&value);

        let url_tag = format!("{{{key}}}");
        if url.contains(&url_tag) {
            url = url.replace(&url_tag, value.as_ref());
        } else {
            query_strs.push(format!("{}={}", key, value));
        }

    }

    Ok(format!("{}?{}", url, query_strs.join("&")))
}

pub struct ListFederatedPhoto;
impl Endpoint for ListFederatedPhoto {
    const PATH: (http::Method, &str) = (http::Method::GET, "/photos");
    type Request = ListFederatedPhotoRequest;
    type Response = ListFederatedPhotoResponse;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ListFederatedPhotoRequest {
    pub limit: u32,
    pub offset: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ListFederatedPhotoResponse {
    pub photos: Vec<String>,
}

pub struct Health;
impl Endpoint for Health {
    const PATH: (http::Method, &str) = (http::Method::GET, "/health");
    type Request = ();
    type Response = HealthResponse;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
}

