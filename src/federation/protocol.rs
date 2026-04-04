use std::sync::Arc;

use anyhow::bail;
use serde_json::Value;

use crate::infra::api::types::IrisResponse;

#[derive(Debug)]
pub enum RequestError {
    Applictaion(http::StatusCode, String),
    Anyhow(anyhow::Error),
}

impl From<anyhow::Error> for RequestError {
    fn from(value: anyhow::Error) -> Self {
        RequestError::Anyhow(value)
    }
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

pub async fn request<E: Endpoint>(
    ctx: Arc<crate::Context>,
    origin: &str,
    param: E::Request
) -> Result<E::Response, RequestError> {
    let (method, path) = E::PATH;
    let query = to_query_param(param)?;

    let req = super::request::create_client()
        .request(method, format!("{}/federation{}{}", origin, path, query)) .with_extension(ctx.clone()) .send()
        .await
        .unwrap();

    let code = req.status();

    let response = req
        .json::<IrisResponse<E::Response>>()
        .await
        .unwrap();

    match response {
        IrisResponse::Okay { response } => {
            Ok(response)
        },
        IrisResponse::Error { reason } => {
            Err(RequestError::Applictaion(code, reason))
        }
    }

}

fn to_query_param(query: impl serde::Serialize) -> anyhow::Result<String> {
    let query = serde_json::to_value(query)?;

    if query == Value::Null {
        return Ok(String::new());
    }

    let Value::Object(query) = query else {
        bail!("The query value needs to be an object");
    };

    let mut query_strs = Vec::new();

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

        query_strs.push(format!("{}={}", key, urlencoding::encode(&value)));
    }

    Ok(format!("?{}", query_strs.join("&")))
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

