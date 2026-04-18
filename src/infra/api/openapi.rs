use utoipa::openapi::{Info, OpenApi, security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme}};

use crate::auth::protocol::SESSION_COOKIE;

pub const AUTH_HEADER: &str = "session_header";
pub const AUTH_COOKIE: &str = "session_cookie";

pub fn finalize_openapi(mut openapi: OpenApi) -> OpenApi {
    let mut info = Info::new("Iris - HTTP Rest API", "1.0.0");
    info.description = Some(indoc::indoc! {r#"
        Iris is a self-host photo album service.
        Iris provides the management layer - the frontend service is separated as "Iridescence"
    "#}.to_string());
    openapi.info = info;

    let components = openapi.components.get_or_insert_default();
    components.add_security_scheme(
        AUTH_HEADER,
        SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer))
    );
    components.add_security_scheme(
        AUTH_COOKIE,
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE)))
    );

    openapi
}

