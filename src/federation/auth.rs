pub mod hash;
pub mod sign;
pub mod verify;

#[derive(Clone, Debug)]
pub struct ChallengePayload<'p> {
    pub host: &'p str,
    pub method: &'p http::Method,
    pub path_name: &'p str,
    pub query: Option<&'p str>,
    pub body: Option<&'p [u8]>
}

