use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
pub struct FederationConfig {
    hosts: HashMap<String, FederationHost>
}

#[derive(Debug, serde::Deserialize)]
pub struct FederationHost {
    origin: String,
} 

