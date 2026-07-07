use std::sync::Mutex;

use crate::{auth::{config::AuthConfig, session::SessionsStore}, repository::{io::ScopedPath, whitelist::WhitelistRepository}};

pub struct AuthContext {
    pub config: AuthConfig,
    pub state: Mutex<RuntimeState>
}

pub struct RuntimeState {
    pub sessions: SessionsStore,
    pub whitelist: WhitelistRepository,
}

impl AuthContext {
    pub fn new(config: AuthConfig, base_dir: &ScopedPath) -> Self {
        if config.unrestricted_instance.is_some_and(|x| x) {
            println!();
            println!(" \x1b[48;5;1;38;5;255;1m                                                 \x1b[m");
            println!(" \x1b[48;5;1;38;5;255;1m  CAUTION REGARDING TO THE SECURITY AND PRIVACY  \x1b[m");
            println!(" \x1b[48;5;1;38;5;255;1m                                                 \x1b[m");
            println!();
            println!("\x1b[38;5;9;1;4m    THIS INSTANCE IS MARKED AS UNRESTRICTED INSTANCE    \x1b[m");
            println!("  This instance has `unrestricted_instance` configured as true.");
            println!("  This will bypass all authentication/authorization, making all stored");
            println!("  data visible. Do make sure this is intended behavior");
            println!();
        }

        AuthContext {
            config,
            state: Mutex::new(RuntimeState {
                sessions: Default::default(),
                whitelist: WhitelistRepository::new(base_dir),
            }),
        }
    }
}

