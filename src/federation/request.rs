use http::Extensions;
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
        // TODO: Revive these later
        //
        // let ctx = ext.get::<Arc<crate::Context>>().unwrap();
        //
        // let private_key = std::fs::read_to_string(ctx.ingest.config.dir.join("federation.sec")).unwrap();
        // let signed = auth::sign::sign_challenge(
        //     &ChallengePayload {
        //         host: &ctx.base.host,
        //         method: req.method(),
        //         path_name: req.url().path(),
        //         query: req.url().query(),
        //         body: req.body().and_then(|body| body.as_bytes()),
        //     },
        //     private_key,
        // ).unwrap();

        // req.headers_mut().insert("X-Iris-Signature", signed.parse().unwrap());

        next.run(req, ext).await
    }
}

