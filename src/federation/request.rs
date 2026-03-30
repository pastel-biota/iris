use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey, pkcs8::DecodePrivateKey as _};
use http::{Extensions, HeaderValue};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next};

pub fn create_client() -> ClientWithMiddleware {
    ClientBuilder::new(reqwest::Client::default())
        .with(AttachHash)
        .build()
}

pub struct AttachHash;

#[async_trait::async_trait]
impl Middleware for AttachHash {
    async fn handle(
        &self,
        mut req: reqwest::Request,
        ext: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let hash = super::auth::create_hash(
            req.method(),
            req.url().path(),
            req.url().query(),
            req.body().and_then(|body| body.as_bytes()),
        ).unwrap();

        let ctx = ext.get::<Arc<crate::Context>>().unwrap();

        let private_key = std::fs::read_to_string(ctx.ingest.config.dir.join("federation.sec")).unwrap();
        let private_key = SigningKey::from_pkcs8_pem(&private_key).unwrap();

        let signed = private_key.sign(hash.as_bytes());
        req.headers_mut().insert(
            "X-Iris-Signature",
            format!("{}:{}:{}", &ctx.base.host, hash, signed).parse().unwrap()
        );

        next.run(req, ext).await
    }
}

