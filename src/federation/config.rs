use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
pub struct FederationConfig {
    pub hosts: HashMap<String, FederationHost>
}

#[derive(Debug, serde::Deserialize)]
pub struct FederationHost {
    pub pubkey: String,
    pub origin: String,
}
