use std::str::FromStr as _;

use anyhow::Context as _;
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey, pkcs8::DecodePublicKey as _};

use crate::federation::auth::ChallengePayload;

pub fn get_sender_host(received_payload: &str) -> anyhow::Result<&str> {
    let mut parts = received_payload.splitn(3, ":");
    let (host, _provided_hash, _challenge) = (|| {
        let host = parts.next()?;
        let hash = parts.next()?;
        let challenge = parts.next()?;
        Some((host, hash, challenge))
    })().context("The hash value was not in the expected format")?;

    Ok(host)
}

pub fn verify_challenge(received_payload: &str, pubkey: &str, challenge: &ChallengePayload) -> anyhow::Result<()> {
    let hash = super::hash::create_hash(challenge)
        .context("Failed to calculate the hash")?;

    let mut parts = received_payload.splitn(3, ":");
    let (host, provided_hash, challenge) = (|| {
        let host = parts.next()?;
        let hash = parts.next()?;
        let challenge = parts.next()?;
        Some((host, hash, challenge))
    })().context("The hash value was not in the expected format")?;

    if provided_hash != hash {
        anyhow::bail!("The hash does not match between the provided one and calculated one");
    }

    let verify_key = pubkey.trim();
    let verify_key = VerifyingKey::from_public_key_pem(verify_key)
        .context("The configured public key is not valid")?;

    verify_key.verify(
        provided_hash.as_bytes(),
        &Signature::from_str(challenge).unwrap()
    ).context("The signature is not valid/tampered")?;

    tracing::debug!("Verified request from {host} as valid");

    Ok(())
}

