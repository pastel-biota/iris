use std::io::Write;

use sha2::Digest;

pub fn create_hash(
    method: &http::Method,
    path_name: &str,
    query: Option<&str>,
    body: Option<&[u8]>
) -> anyhow::Result<String> {
    let mut hasher = sha2::Sha512::new();

    let mut hash_content = Vec::<u8>::new();

    hash_content.write(method.to_string().as_bytes())?;
    hash_content.write(path_name.as_bytes())?;

    if let Some(query) = query {
        hash_content.write(query.as_bytes())?;
    } else {
        hash_content.write(b"empty-query")?;
    }

    if let Some(body) = body {
        hash_content.write(body)?;
    } else {
        hash_content.write(b"empty-body")?;
    }

    println!("Hash = `{}`", String::from_utf8(hash_content.clone()).unwrap());

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

