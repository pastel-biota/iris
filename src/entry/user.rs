#[allow(clippy::disallowed_types)]
use std::path::PathBuf;

use std::{io::Write as _, sync::Arc};

use crate::{Context, model::EntityName};

#[derive(Clone, Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("output")
        .required(true)
        .multiple(false)
        .args(["write_to_file", "write_to_stdout"]),
))]
pub struct UserOptions {
    #[clap(short, long)]
    pub name: EntityName,

    /// The file of the configuration file to be written
    #[clap(short = 'w', long)]
    #[allow(clippy::disallowed_types)]
    pub write_to_file: Option<PathBuf>,

    /// The file of the configuration file to be written
    #[clap(short = 'W', long)]
    pub write_to_stdout: bool,

    /// Register the entity as a federation peer, which can only read whitelisted photos
    /// and cannot use the administrative endpoints
    #[clap(long)]
    pub federation: bool,
}


pub async fn create_user(ctx: Arc<Context>, user: UserOptions) -> anyhow::Result<()> {
    let config_file = user.write_to_file.unwrap();

    if !config_file.is_file() {
        anyhow::bail!("The specified path is not a valid file, or does not exist");
    }

    if ctx.auth.config.entities.contains_key(&user.name) {
        println!("[!] There is already a user with the name '{}'", user.name);
        return Ok(());
    }

    println!();
    println!("This utility is going to modify the file at following");
    println!("       Arg: '{}'", config_file.display());
    println!("  Location: '{}'", std::fs::canonicalize(&config_file)?.display());
    println!();
    println!("The hashed password will be recorded to the path above.");
    println!();
    println!("Please make sure that the file is...");
    println!("  * Intended to store the sensitive information, such as being stored at Secret");
    println!("  * Has the proper permission configuration");
    println!();
    println!("\x1b[38;5;3;1mThe message above is very important! Please read before proceed.\x1b[m");
    println!();

    let password = crate::auth::password::accept_password_from_cli(&user.name)?;

    let new_user_config = crate::auth::serialize::serialize_new_entity(&user.name, password, user.federation)?;

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(config_file)?;

    file.write_all("\n".as_bytes())?;
    file.write_all(new_user_config.as_bytes())?;
    file.write_all("\n".as_bytes())?;

    println!("The hashed password is recorded to the file!");

    Ok(())
}

