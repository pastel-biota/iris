use crate::{auth::{config::Entity, context::AuthContext, password::Password, session::ValidSession}, model::EntityName};

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("The username or password is not correct")]
    InvalidCredential,

    #[error(transparent)]
    GenericError(#[from] anyhow::Error),
}

pub async fn login_to_entity(ctx: &AuthContext, username: &EntityName, password: &Password) -> Result<String, LoginError> {
    let entity = ctx.config.entities.get(username);
    let hashed = entity.map(|entity| match entity {
        Entity::User(user) => &user.password,
        Entity::Federation(federation) => &federation.password,
    });

    if !super::password::verify_password(password, hashed)? {
        return Err(LoginError::InvalidCredential);
    }

    let session_key = ctx.state.lock()
        .unwrap()
        .sessions
        .new_session(entity.unwrap().clone())?;

    Ok(session_key)
}

pub async fn verify_session(ctx: &AuthContext, session_key: &str) -> anyhow::Result<Option<ValidSession>> {
    ctx.state.lock()
        .unwrap()
        .sessions
        .get_session(session_key)
}
