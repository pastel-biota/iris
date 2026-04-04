use std::ops::Deref;

use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum IrisResponse<T> {
    Okay { response: T },
    Error { reason: String },
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SuccessfulResponse<T> {
    #[schema(example = "okay")]
    status: &'static str,

    response: T,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ClientError {
    #[schema(example = "error")]
    status: &'static str,

    #[schema(example = "The value was invalid...")]
    reason: String,
}

pub fn success<T>(reason: T) -> SuccessfulResponse<T> {
    SuccessfulResponse {
        status: "okay",
        response: reason,
    }
}

pub fn client_error(reason: &str) -> ClientError {
    ClientError {
        status: "error",
        reason: reason.to_string(),
    }
}

#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
pub struct BinaryBody(Vec<u8>);

impl Deref for BinaryBody {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<u8>> for BinaryBody {
    fn from(value: Vec<u8>) -> Self {
        BinaryBody(value)
    }
}
