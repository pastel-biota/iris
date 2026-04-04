use anyhow::Context;
use ed25519_dalek::{Signer as _, SigningKey, pkcs8::DecodePrivateKey as _};

use crate::federation::auth::ChallengePayload;

pub fn sign_challenge(challenge: &ChallengePayload, secret_pem: String) -> anyhow::Result<String> {
    let hash = super::hash::create_hash(challenge)
        .context("Failed to calculate the hash")?;

    let private_key = SigningKey::from_pkcs8_pem(&secret_pem)
        .context("The private key file is not valid PKCS#8 PEM file")?;

    let signed = private_key.sign(hash.as_bytes());
    Ok(format!("{}:{}:{}", &challenge.host, hash, signed))
}

