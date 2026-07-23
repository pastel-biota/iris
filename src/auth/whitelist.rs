use crate::{auth::{context::AuthContext, session::Session}, model::Identifier, repository::whitelist::WhitelistRepositoryError};

#[derive(thiserror::Error, Debug)]
pub enum WhitelistError {
    #[error("The photo is not in the whitelist for the entity")]
    NotAllowed,

    #[error(transparent)]
    FileError(#[from] WhitelistRepositoryError),
}

pub async fn ensure_photo_allowed(ctx: &AuthContext, session: &Session, photo: &Identifier) -> Result<(), WhitelistError> {
    let Some(session) = session.not_bypassed() else {
        return Ok(());
    };

    let allowed = ctx.state.whitelist.photo_allowed(session.entity.name(), photo).await?;

    if allowed {
        Ok(())
    } else {
        Err(WhitelistError::NotAllowed)
    }
}

