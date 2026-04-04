use std::io::Write;

use sha2::Digest;

use crate::federation::auth::ChallengePayload;

pub fn create_hash(payload: &ChallengePayload) -> anyhow::Result<String> {
    let mut hasher = sha2::Sha512::new();

    let mut hash_content = Vec::<u8>::new();

    hash_content.write(b"iris")?;
    hash_content.write(payload.host.as_bytes())?;
    hash_content.write(payload.method.to_string().as_bytes())?;
    hash_content.write(payload.path_name.as_bytes())?;

    if let Some(query) = payload.query {
        hash_content.write(query.as_bytes())?;
    } else {
        hash_content.write(b"empty-query")?;
    }

    if let Some(body) = payload.body && body.len() > 0 {
        hash_content.write(body)?;
    } else {
        hash_content.write(b"empty-body")?;
    }

    hasher.write(&hash_content)?;
    let hashed = hasher.finalize();
    Ok(hashed
        .windows(2)
        .step_by(2)
        .fold(String::with_capacity(64), |mut acc, bytes| {
            let &[upper, lower] = bytes else {
                panic!("Hash succeeded but the iterator is not iterating in expected form");
            };

            acc.push_str(&format!("{:02x}{:02x}", upper, lower));
            acc
        }))
}

