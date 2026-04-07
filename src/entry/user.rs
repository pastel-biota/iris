use std::sync::Arc;

use crate::{Context, config::UserConfig};

pub async fn create_user(ctx: Arc<Context>, user: UserConfig) -> anyhow::Result<()> {
    let password = loop {
        let password = rpassword::prompt_password(format!("Please type a password for the new user '{}': ", user.name))?;

        if password.trim().len() != password.len() {
            println!("[!] The password contains trailing space characters");
            continue;
        }

        if password.is_empty() {
            println!("[!] You need to type non-empty password");
            continue;
        }

        if user.name == password {
            println!("[!] You cannot use the same password to the username");
            continue;
        }

        break password
    };

    dbg!(password);

    Ok(())
}

