use std::{str::FromStr, sync::LazyLock};

use anyhow::Context as _;
use rand::prelude::*;

static DUMMY_HASH: LazyLock<HashedPassword> = LazyLock::new(|| {
    HashedPassword(bcrypt::hash("dummy", 12).expect("failed to generate dummy hash"))
});

#[derive(Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct Password(String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Password").field(&"***".to_string()).finish()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HashedPassword(String);

impl std::fmt::Debug for HashedPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HashedPassword").field(&"***".to_string()).finish()
    }
}

impl FromStr for Password {
    type Err = anyhow::Error;

    fn from_str(password: &str) -> Result<Self, Self::Err> {
        if password.trim().len() != password.len() {
            anyhow::bail!("The password contains trailing space characters");
        }

        if password.is_empty() {
            anyhow::bail!("You need to type non-empty password");
        }

        if password.as_bytes().len() > 72 {
            anyhow::bail!("Your password contains more than 72 bytes, which can't be used for the password!");
        }

        Ok(Self(password.to_string()))
    }
}

pub fn accept_password_from_cli(username: &str) -> anyhow::Result<HashedPassword> {
    let password = loop {
        let password = rpassword::prompt_password(format!("Please type a password for the new user '{}': ", username))?;
        let password = match password.parse::<Password>() {
            Ok(password) => password,
            Err(err) => {
                println!("[!] {err}");
                continue;
            }
        };

        if password.0 == username {
            println!("[!] You cannot use the same password to the username");
            continue;
        }

        let confirmatory = rpassword::prompt_password(format!("Type again the same password for '{}' to confirm: ", username))?;

        if password.0 != confirmatory {
            println!("[!] The password did not match!");
            continue;
        }

        break password;
    };

    println!("Please wait, hashing the password...");
    let hashed = hash_password(&password)?;

    Ok(hashed)
}

pub fn hash_password(password: &Password) -> anyhow::Result<HashedPassword> {
    let mut salt_rng: StdRng = rand::make_rng();
    let salt: [u8; 16] = salt_rng.random();

    let hashed = bcrypt::hash_with_salt(&password.0.as_bytes(), bcrypt::DEFAULT_COST, salt)
        .context("Couldn't geneerate the hash for the password")?;

    Ok(HashedPassword(hashed.to_string()))
}

pub fn verify_password(password: &Password, hash: Option<&HashedPassword>) -> anyhow::Result<bool> {
    println!("Verifying");
    Ok(bcrypt::verify(&password.0, &hash.as_ref().unwrap_or(&&*DUMMY_HASH).0)?)
}

