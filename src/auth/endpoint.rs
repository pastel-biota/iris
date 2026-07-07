use http::Method;
use crate::{federation::protocol::Endpoint, model::EntityName};

pub struct LoginEndpoint;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LoginBody {
    // TODO: Stop using username/password based auth
    pub username: EntityName,
    pub password: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub session_key: String,
}

impl Endpoint for LoginEndpoint {
    const PATH: (Method, &str) = (Method::POST, "/auth/login");
    type Request = LoginBody;
    type Response = LoginResponse;
}

pub struct MeEndpoint;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub name: EntityName,
}

impl Endpoint for MeEndpoint {
    const PATH: (Method, &str) = (Method::GET, "/auth/me");
    type Request = ();
    type Response = MeResponse;
}
