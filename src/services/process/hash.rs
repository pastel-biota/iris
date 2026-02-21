use sha2::{Digest, Sha256};

pub fn retrieve_file_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .windows(2)
        .step_by(2)
        .fold(String::with_capacity(64), |mut acc, bytes| {
            let &[upper, lower] = bytes else {
                panic!("Hash succeeded but the iterator is not iterating in expected form");
            };

            acc.push_str(&format!("{:02x}{:02x}", upper, lower));
            acc
        })
}
